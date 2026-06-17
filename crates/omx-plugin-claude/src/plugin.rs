use omx_core::{
    AccountRef, AccountStatus, Availability, ConfigProfile, ConfigSwitchReport, DoctorCheck,
    DoctorReport, ImportConfigOptions, ImportedConfig, LoginOptions, OpenMuxError,
    PlatformCapabilities, PlatformInfo, PlatformInstall, PlatformPlugin, PlatformPoolSummary,
    Result, SaveOptions, SwitchReport, UseReport, platform_info,
    storage::{
        create_dir_private, data_local_dir, display_path, home_dir, io_error, read_file,
        set_private_file_permissions, sha256_hex, unix_now, unix_now_nanos,
        write_file_atomic_private,
    },
};
use registry_io::{
    encode_account_registry, encode_registry, parse_account_registry, parse_registry,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

#[path = "registry_io.rs"]
mod registry_io;

const REGISTRY_SCHEMA_VERSION: u32 = 1;
const SETTINGS_FILE_NAME: &str = "settings.json";
const MANAGED_ENV_KEYS: &[&str] = &[
    "ANTHROPIC_BASE_URL",
    "ANTHROPIC_AUTH_TOKEN",
    "ANTHROPIC_API_KEY",
    "ANTHROPIC_MODEL",
    "CLAUDE_CODE_USE_BEDROCK",
    "CLAUDE_CODE_USE_VERTEX",
    "CLAUDE_CODE_USE_FOUNDRY",
    "CLAUDE_CODE_SKIP_BEDROCK_AUTH",
    "CLAUDE_CODE_SKIP_VERTEX_AUTH",
];

#[derive(Debug, Clone, Default)]
pub struct ClaudePlugin {
    claude_home: Option<PathBuf>,
    state_root: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct ClaudeAccountPlugin {
    claude_home: Option<PathBuf>,
    state_root: Option<PathBuf>,
    claude_executable: PathBuf,
    credential_backend: Option<CredentialBackendConfig>,
    #[cfg(test)]
    fail_settings_write: bool,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
enum CredentialBackendConfig {
    FakeKeychain(PathBuf),
    FakeKeychainWriteFailure(PathBuf),
}

trait CredentialBackend {
    fn label(&self) -> &'static str;
    fn location(&self) -> &Path;
    fn exists(&self) -> bool;
    fn read(&self) -> Result<Vec<u8>>;
    fn write(&self, bytes: &[u8]) -> Result<()>;
    fn restore(&self, bytes: Option<&[u8]>) -> Result<()>;
}

#[derive(Debug, Clone)]
struct PlaintextCredentialBackend {
    path: PathBuf,
}

#[derive(Debug, Clone)]
struct FakeKeychainCredentialBackend {
    path: PathBuf,
}

#[derive(Debug, Clone)]
struct FakeKeychainWriteFailureCredentialBackend {
    path: PathBuf,
}

#[cfg(target_os = "macos")]
#[derive(Debug, Clone)]
struct MacOsKeychainCredentialBackend {
    service: String,
    account: String,
    display_path: PathBuf,
}

impl CredentialBackend for PlaintextCredentialBackend {
    fn label(&self) -> &'static str {
        "plaintext"
    }

    fn location(&self) -> &Path {
        &self.path
    }

    fn exists(&self) -> bool {
        self.path.exists()
    }

    fn read(&self) -> Result<Vec<u8>> {
        read_file(&self.path)
    }

    fn write(&self, bytes: &[u8]) -> Result<()> {
        write_file_atomic_private(&self.path, bytes)
    }

    fn restore(&self, bytes: Option<&[u8]>) -> Result<()> {
        rollback_file(&self.path, bytes)
    }
}

impl CredentialBackend for FakeKeychainCredentialBackend {
    fn label(&self) -> &'static str {
        "keychain/fake"
    }

    fn location(&self) -> &Path {
        &self.path
    }

    fn exists(&self) -> bool {
        self.path.exists()
    }

    fn read(&self) -> Result<Vec<u8>> {
        read_file(&self.path)
    }

    fn write(&self, bytes: &[u8]) -> Result<()> {
        write_file_atomic_private(&self.path, bytes)
    }

    fn restore(&self, bytes: Option<&[u8]>) -> Result<()> {
        rollback_file(&self.path, bytes)
    }
}

impl CredentialBackend for FakeKeychainWriteFailureCredentialBackend {
    fn label(&self) -> &'static str {
        "keychain/fake-write-failure"
    }

    fn location(&self) -> &Path {
        &self.path
    }

    fn exists(&self) -> bool {
        self.path.exists()
    }

    fn read(&self) -> Result<Vec<u8>> {
        read_file(&self.path)
    }

    fn write(&self, _bytes: &[u8]) -> Result<()> {
        Err(OpenMuxError::Message(
            "injected Claude credential backend write failure".to_string(),
        ))
    }

    fn restore(&self, bytes: Option<&[u8]>) -> Result<()> {
        rollback_file(&self.path, bytes)
    }
}

#[cfg(target_os = "macos")]
impl CredentialBackend for MacOsKeychainCredentialBackend {
    fn label(&self) -> &'static str {
        "keychain"
    }

    fn location(&self) -> &Path {
        &self.display_path
    }

    fn exists(&self) -> bool {
        macos_keychain::exists(&self.service, &self.account)
    }

    fn read(&self) -> Result<Vec<u8>> {
        macos_keychain::read(&self.service, &self.account)
    }

    fn write(&self, bytes: &[u8]) -> Result<()> {
        macos_keychain::write(&self.service, &self.account, bytes)
    }

    fn restore(&self, bytes: Option<&[u8]>) -> Result<()> {
        match bytes {
            Some(bytes) => self.write(bytes),
            None => macos_keychain::delete(&self.service, &self.account),
        }
    }
}

#[derive(Debug, Clone)]
struct Registry {
    schema_version: u32,
    active_profile_number: Option<u32>,
    next_profile_number: u32,
    profiles: Vec<StoredProfile>,
}

#[derive(Debug, Clone)]
struct StoredProfile {
    number: u32,
    name: String,
    auth_type: String,
    base_url: Option<String>,
    model: Option<String>,
    secret_hash: String,
    snapshot_path: String,
    imported_at_unix: u64,
    last_activated_at_unix: Option<u64>,
}

#[derive(Debug, Clone)]
struct AccountRegistry {
    schema_version: u32,
    active_account_number: Option<u32>,
    next_account_number: u32,
    accounts: Vec<StoredAccount>,
}

#[derive(Debug, Clone)]
struct StoredAccount {
    number: u32,
    name: String,
    email: Option<String>,
    account_uuid_hash: Option<String>,
    organization_uuid_hash: Option<String>,
    scopes: Option<String>,
    expires_at_unix: i64,
    refresh_token_hash: String,
    snapshot_hash: String,
    snapshot_path: String,
    oauth_account_path: String,
    partial_metadata: bool,
    imported_at_unix: u64,
    last_activated_at_unix: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ProfileSnapshot {
    env: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
struct ParsedProfile {
    name: String,
    auth_type: String,
    base_url: Option<String>,
    model: Option<String>,
    env: BTreeMap<String, String>,
}

impl Default for Registry {
    fn default() -> Self {
        Self {
            schema_version: REGISTRY_SCHEMA_VERSION,
            active_profile_number: None,
            next_profile_number: 1,
            profiles: Vec::new(),
        }
    }
}

impl Default for AccountRegistry {
    fn default() -> Self {
        Self {
            schema_version: REGISTRY_SCHEMA_VERSION,
            active_account_number: None,
            next_account_number: 1,
            accounts: Vec::new(),
        }
    }
}

impl Default for ClaudeAccountPlugin {
    fn default() -> Self {
        Self {
            claude_home: None,
            state_root: None,
            claude_executable: env::var_os("OMUX_CLAUDE_BIN")
                .filter(|value| !value.is_empty())
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("claude")),
            credential_backend: None,
            #[cfg(test)]
            fail_settings_write: false,
        }
    }
}

impl ClaudePlugin {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_paths(claude_home: impl Into<PathBuf>, state_root: impl Into<PathBuf>) -> Self {
        Self {
            claude_home: Some(claude_home.into()),
            state_root: Some(state_root.into()),
        }
    }

    fn info(&self) -> PlatformInfo {
        platform_info(self.id(), self.name())
    }

    fn claude_home(&self) -> Result<PathBuf> {
        if let Some(path) = &self.claude_home {
            return Ok(path.clone());
        }

        if let Some(path) = env::var_os("CLAUDE_CONFIG_DIR").filter(|value| !value.is_empty()) {
            return Ok(PathBuf::from(path));
        }

        home_dir()
            .map(|path| path.join(".claude"))
            .ok_or_else(|| OpenMuxError::Message("could not resolve the home directory".into()))
    }

    fn state_root(&self) -> Result<PathBuf> {
        if let Some(path) = &self.state_root {
            return Ok(path.clone());
        }

        if let Some(path) = env::var_os("OMUX_STATE_ROOT").filter(|value| !value.is_empty()) {
            return Ok(PathBuf::from(path));
        }

        data_local_dir()
            .map(|path| path.join("openmux"))
            .ok_or_else(|| {
                OpenMuxError::Message("could not resolve the OpenMux data directory".into())
            })
    }

    fn platform_state_dir(&self) -> Result<PathBuf> {
        Ok(self.state_root()?.join("platforms").join(self.id()))
    }

    fn profiles_dir(&self) -> Result<PathBuf> {
        Ok(self.platform_state_dir()?.join("profiles"))
    }

    fn backups_dir(&self) -> Result<PathBuf> {
        Ok(self.platform_state_dir()?.join("backups"))
    }

    fn registry_path(&self) -> Result<PathBuf> {
        Ok(self.platform_state_dir()?.join("registry.omx"))
    }

    fn account_registry_path(&self) -> Result<PathBuf> {
        Ok(self.platform_state_dir()?.join("account-registry.omx"))
    }

    fn settings_path(&self) -> Result<PathBuf> {
        Ok(self.claude_home()?.join(SETTINGS_FILE_NAME))
    }

    fn profile_snapshot_path(&self, number: u32) -> Result<PathBuf> {
        Ok(self.profiles_dir()?.join(format!("{number}.profile.json")))
    }

    fn load_registry(&self) -> Result<Registry> {
        let path = self.registry_path()?;
        if !path.exists() {
            return Ok(Registry::default());
        }

        let text = String::from_utf8(read_file(&path)?)
            .map_err(|err| OpenMuxError::Message(format!("{}: {err}", display_path(&path))))?;
        let mut registry = parse_registry(&path, &text)?;
        if registry.schema_version > REGISTRY_SCHEMA_VERSION {
            return Err(OpenMuxError::Message(format!(
                "registry schema {} is newer than this OpenMux build supports",
                registry.schema_version
            )));
        }
        registry.profiles.sort_by_key(|profile| profile.number);
        registry.next_profile_number = registry.next_profile_number.max(
            registry
                .profiles
                .iter()
                .map(|profile| profile.number)
                .max()
                .unwrap_or(0)
                + 1,
        );
        Ok(registry)
    }

    fn save_registry(&self, registry: &Registry) -> Result<()> {
        write_file_atomic_private(&self.registry_path()?, encode_registry(registry).as_bytes())
    }

    fn load_account_registry(&self) -> Result<AccountRegistry> {
        let path = self.account_registry_path()?;
        if !path.exists() {
            return Ok(AccountRegistry::default());
        }
        let text = String::from_utf8(read_file(&path)?)
            .map_err(|err| OpenMuxError::Message(format!("{}: {err}", display_path(&path))))?;
        parse_account_registry(&path, &text)
    }

    fn save_account_registry(&self, registry: &AccountRegistry) -> Result<()> {
        write_file_atomic_private(
            &self.account_registry_path()?,
            encode_account_registry(registry).as_bytes(),
        )
    }

    fn account_active(&self) -> Result<bool> {
        Ok(self
            .load_account_registry()?
            .active_account_number
            .is_some())
    }

    fn deactivate_account(&self) -> Result<()> {
        let mut registry = self.load_account_registry()?;
        if registry.active_account_number.is_none() {
            return Ok(());
        }
        registry.active_account_number = None;
        self.save_account_registry(&registry)
    }

    fn resolve_profile<'a>(
        &self,
        registry: &'a Registry,
        selector: &str,
    ) -> Result<&'a StoredProfile> {
        if let Ok(number) = selector.parse::<u32>() {
            return registry
                .profiles
                .iter()
                .find(|profile| profile.number == number)
                .ok_or_else(|| OpenMuxError::AccountNotFound {
                    platform: self.id().to_string(),
                    account: selector.to_string(),
                });
        }

        registry
            .profiles
            .iter()
            .find(|profile| profile.name == selector)
            .ok_or_else(|| OpenMuxError::AccountNotFound {
                platform: self.id().to_string(),
                account: selector.to_string(),
            })
    }

    fn profile_status(&self, profile: &StoredProfile, active_number: Option<u32>) -> ConfigProfile {
        ConfigProfile {
            platform: self.info(),
            name: profile.name.clone(),
            active: active_number == Some(profile.number),
            config_path: profile.snapshot_path.clone(),
            provider_id: None,
            base_url: profile.base_url.clone(),
            model: profile.model.clone(),
            number: Some(profile.number),
            auth_type: Some(profile.auth_type.clone()),
        }
    }

    fn import_profile(&self, options: ImportConfigOptions) -> Result<ImportedConfig> {
        let parsed = parse_profile(&options.content, options.name.as_deref())?;
        let snapshot = ProfileSnapshot {
            env: parsed.env.clone(),
        };
        let snapshot_bytes = serde_json::to_vec_pretty(&snapshot).map_err(|err| {
            OpenMuxError::Message(format!("failed to encode Claude profile: {err}"))
        })?;
        let secret_hash = sha256_hex(&snapshot_bytes);
        let mut registry = self.load_registry()?;
        let now = unix_now();

        let existing_index = registry
            .profiles
            .iter()
            .position(|profile| profile.secret_hash == secret_hash || profile.name == parsed.name);
        let number = if let Some(index) = existing_index {
            registry.profiles[index].number
        } else {
            let number = registry.next_profile_number;
            registry.next_profile_number += 1;
            number
        };
        let snapshot_path = self.profile_snapshot_path(number)?;
        write_file_atomic_private(&snapshot_path, &snapshot_bytes)?;

        let stored = StoredProfile {
            number,
            name: parsed.name.clone(),
            auth_type: parsed.auth_type.clone(),
            base_url: parsed.base_url.clone(),
            model: parsed.model.clone(),
            secret_hash,
            snapshot_path: display_path(&snapshot_path),
            imported_at_unix: now,
            last_activated_at_unix: existing_index
                .and_then(|index| registry.profiles[index].last_activated_at_unix),
        };
        if let Some(index) = existing_index {
            registry.profiles[index] = stored;
        } else {
            registry.profiles.push(stored);
        }
        registry.profiles.sort_by_key(|profile| profile.number);
        self.save_registry(&registry)?;

        Ok(ImportedConfig {
            platform: self.info(),
            profile_name: parsed.name,
            config_path: display_path(&snapshot_path),
            provider_id: None,
            base_url: parsed.base_url,
            model: parsed.model,
            number: Some(number),
            auth_type: Some(parsed.auth_type),
        })
    }

    fn use_profile(&self, selector: &str) -> Result<ConfigSwitchReport> {
        let mut registry = self.load_registry()?;
        let profile = self.resolve_profile(&registry, selector)?.clone();
        let snapshot_path = PathBuf::from(&profile.snapshot_path);
        let snapshot_bytes = read_file(&snapshot_path)?;
        if sha256_hex(&snapshot_bytes) != profile.secret_hash {
            return Err(OpenMuxError::Message(format!(
                "stored Claude profile #{} failed hash verification",
                profile.number
            )));
        }
        let snapshot: ProfileSnapshot = serde_json::from_slice(&snapshot_bytes).map_err(|err| {
            OpenMuxError::Message(format!("invalid Claude profile snapshot: {err}"))
        })?;

        let settings_path = self.settings_path()?;
        let current_bytes = if settings_path.exists() {
            Some(read_file(&settings_path)?)
        } else {
            None
        };
        let mut settings = current_bytes
            .as_deref()
            .map(parse_settings)
            .transpose()?
            .unwrap_or_else(|| Value::Object(Map::new()));
        apply_env_patch(&mut settings, &snapshot.env)?;
        let next_bytes = serde_json::to_vec_pretty(&settings).map_err(|err| {
            OpenMuxError::Message(format!("failed to encode Claude settings: {err}"))
        })?;

        let backup_path = if let Some(current) = &current_bytes {
            if current != &next_bytes {
                let path = self
                    .backups_dir()?
                    .join(format!("settings.json.bak.{}", unix_now_nanos()));
                if let Some(parent) = path.parent() {
                    create_dir_private(parent)?;
                }
                fs::copy(&settings_path, &path).map_err(|err| io_error(&path, err))?;
                set_private_file_permissions(&path)?;
                Some(display_path(&path))
            } else {
                None
            }
        } else {
            None
        };

        write_file_atomic_private(&settings_path, &next_bytes)?;
        registry.active_profile_number = Some(profile.number);
        if let Some(stored) = registry
            .profiles
            .iter_mut()
            .find(|stored| stored.number == profile.number)
        {
            stored.last_activated_at_unix = Some(unix_now());
        }
        if let Err(err) = self.save_registry(&registry) {
            let rollback = match current_bytes {
                Some(bytes) => write_file_atomic_private(&settings_path, &bytes),
                None => fs::remove_file(&settings_path)
                    .map_err(|remove_err| io_error(&settings_path, remove_err)),
            };
            return match rollback {
                Ok(()) => Err(OpenMuxError::Message(format!(
                    "failed to update Claude registry after applying profile; settings were rolled back: {err}"
                ))),
                Err(rollback_err) => Err(OpenMuxError::Message(format!(
                    "failed to update Claude registry after applying profile and rollback failed: {err}; rollback error: {rollback_err}"
                ))),
            };
        }
        self.deactivate_account()?;

        Ok(ConfigSwitchReport {
            platform: self.info(),
            profile: self.profile_status(&profile, Some(profile.number)),
            config_path: display_path(&settings_path),
            backup_path,
        })
    }
}

impl PlatformPlugin for ClaudePlugin {
    fn id(&self) -> &'static str {
        "claude"
    }

    fn name(&self) -> &'static str {
        "Claude Code"
    }

    fn capabilities(&self) -> PlatformCapabilities {
        PlatformCapabilities {
            profiles: true,
            profile_import: true,
            ..PlatformCapabilities::default()
        }
    }

    fn detect(&self) -> Result<PlatformInstall> {
        let settings_path = self.settings_path()?;
        Ok(PlatformInstall {
            platform: self.info(),
            config_path: settings_path.exists().then(|| display_path(&settings_path)),
            auth_path: None,
        })
    }

    fn pool_summary(&self) -> Result<PlatformPoolSummary> {
        let profiles = self.list_configs()?;
        let active_profile = profiles
            .iter()
            .find(|profile| profile.active)
            .map(|profile| profile.name.clone());
        Ok(PlatformPoolSummary {
            platform: self.info(),
            account_count: 0,
            active: None,
            profile_count: profiles.len(),
            active_profile,
            availability: Availability::unknown(),
        })
    }

    fn current(&self) -> Result<Option<AccountStatus>> {
        Ok(None)
    }

    fn list_accounts(&self) -> Result<Vec<AccountStatus>> {
        Ok(Vec::new())
    }

    fn list_configs(&self) -> Result<Vec<ConfigProfile>> {
        let registry = self.load_registry()?;
        let active_number = if self.account_active()? {
            None
        } else {
            registry.active_profile_number
        };
        Ok(registry
            .profiles
            .iter()
            .map(|profile| self.profile_status(profile, active_number))
            .collect())
    }

    fn login(&self, _options: LoginOptions) -> Result<AccountRef> {
        Err(deferred_account_error())
    }

    fn save_current(&self, _options: SaveOptions) -> Result<AccountRef> {
        Err(deferred_account_error())
    }

    fn import_config(&self, options: ImportConfigOptions) -> Result<ImportedConfig> {
        self.import_profile(options)
    }

    fn use_target(&self, selector: &str) -> Result<UseReport> {
        self.use_profile(selector).map(UseReport::Config)
    }

    fn switch_to(&self, _selector: &str) -> Result<SwitchReport> {
        Err(deferred_account_error())
    }

    fn set_alias(&self, _selector: &str, _alias: &str) -> Result<AccountRef> {
        Err(deferred_account_error())
    }

    fn doctor(&self) -> Result<DoctorReport> {
        let claude_home = self.claude_home()?;
        let settings_path = self.settings_path()?;
        let registry_path = self.registry_path()?;
        let registry = self.load_registry();
        Ok(DoctorReport {
            platform: self.id().to_string(),
            checks: vec![
                DoctorCheck {
                    name: "claude-home".to_string(),
                    ok: claude_home.exists(),
                    message: display_path(&claude_home),
                },
                DoctorCheck {
                    name: "user-settings".to_string(),
                    ok: settings_path.exists(),
                    message: display_path(&settings_path),
                },
                DoctorCheck {
                    name: "registry".to_string(),
                    ok: registry.is_ok(),
                    message: if registry_path.exists() {
                        display_path(&registry_path)
                    } else {
                        "not created yet".to_string()
                    },
                },
            ],
        })
    }
}

impl ClaudeAccountPlugin {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_paths(claude_home: impl Into<PathBuf>, state_root: impl Into<PathBuf>) -> Self {
        Self {
            claude_home: Some(claude_home.into()),
            state_root: Some(state_root.into()),
            credential_backend: None,
            ..Self::default()
        }
    }

    #[cfg(test)]
    fn with_paths_and_claude_executable(
        claude_home: impl Into<PathBuf>,
        state_root: impl Into<PathBuf>,
        claude_executable: impl Into<PathBuf>,
    ) -> Self {
        Self {
            claude_home: Some(claude_home.into()),
            state_root: Some(state_root.into()),
            claude_executable: claude_executable.into(),
            credential_backend: None,
            fail_settings_write: false,
        }
    }

    #[cfg(test)]
    fn with_paths_and_fake_keychain(
        claude_home: impl Into<PathBuf>,
        state_root: impl Into<PathBuf>,
        fake_keychain_path: impl Into<PathBuf>,
    ) -> Self {
        Self {
            claude_home: Some(claude_home.into()),
            state_root: Some(state_root.into()),
            claude_executable: Self::default().claude_executable,
            credential_backend: Some(CredentialBackendConfig::FakeKeychain(
                fake_keychain_path.into(),
            )),
            fail_settings_write: false,
        }
    }

    #[cfg(test)]
    fn with_paths_and_fake_keychain_settings_write_failure(
        claude_home: impl Into<PathBuf>,
        state_root: impl Into<PathBuf>,
        fake_keychain_path: impl Into<PathBuf>,
    ) -> Self {
        Self {
            claude_home: Some(claude_home.into()),
            state_root: Some(state_root.into()),
            claude_executable: Self::default().claude_executable,
            credential_backend: Some(CredentialBackendConfig::FakeKeychain(
                fake_keychain_path.into(),
            )),
            fail_settings_write: true,
        }
    }

    #[cfg(test)]
    fn with_paths_and_fake_keychain_credential_write_failure(
        claude_home: impl Into<PathBuf>,
        state_root: impl Into<PathBuf>,
        fake_keychain_path: impl Into<PathBuf>,
    ) -> Self {
        Self {
            claude_home: Some(claude_home.into()),
            state_root: Some(state_root.into()),
            claude_executable: Self::default().claude_executable,
            credential_backend: Some(CredentialBackendConfig::FakeKeychainWriteFailure(
                fake_keychain_path.into(),
            )),
            fail_settings_write: false,
        }
    }

    fn info(&self) -> PlatformInfo {
        platform_info(self.id(), self.name())
    }

    fn claude_home(&self) -> Result<PathBuf> {
        if let Some(path) = &self.claude_home {
            return Ok(path.clone());
        }

        if let Some(path) = env::var_os("CLAUDE_CONFIG_DIR").filter(|value| !value.is_empty()) {
            return Ok(PathBuf::from(path));
        }

        home_dir()
            .map(|path| path.join(".claude"))
            .ok_or_else(|| OpenMuxError::Message("could not resolve the home directory".into()))
    }

    fn state_root(&self) -> Result<PathBuf> {
        if let Some(path) = &self.state_root {
            return Ok(path.clone());
        }

        if let Some(path) = env::var_os("OMUX_STATE_ROOT").filter(|value| !value.is_empty()) {
            return Ok(PathBuf::from(path));
        }

        data_local_dir()
            .map(|path| path.join("openmux"))
            .ok_or_else(|| {
                OpenMuxError::Message("could not resolve the OpenMux data directory".into())
            })
    }

    fn platform_state_dir(&self) -> Result<PathBuf> {
        Ok(self.state_root()?.join("platforms").join("claude"))
    }

    fn accounts_dir(&self) -> Result<PathBuf> {
        Ok(self.platform_state_dir()?.join("accounts"))
    }

    fn backups_dir(&self) -> Result<PathBuf> {
        Ok(self.platform_state_dir()?.join("backups"))
    }

    fn registry_path(&self) -> Result<PathBuf> {
        Ok(self.platform_state_dir()?.join("account-registry.omx"))
    }

    fn profile_registry_path(&self) -> Result<PathBuf> {
        Ok(self.platform_state_dir()?.join("registry.omx"))
    }

    fn credentials_path(&self) -> Result<PathBuf> {
        Ok(self.claude_home()?.join(".credentials.json"))
    }

    fn credential_backend(&self) -> Result<Box<dyn CredentialBackend>> {
        match &self.credential_backend {
            Some(CredentialBackendConfig::FakeKeychain(path)) => {
                Ok(Box::new(FakeKeychainCredentialBackend {
                    path: path.clone(),
                }))
            }
            Some(CredentialBackendConfig::FakeKeychainWriteFailure(path)) => {
                Ok(Box::new(FakeKeychainWriteFailureCredentialBackend {
                    path: path.clone(),
                }))
            }
            #[cfg(target_os = "macos")]
            None if self.claude_home.is_none() && env::var_os("CLAUDE_CONFIG_DIR").is_none() => {
                let service = env::var("OMUX_CLAUDE_KEYCHAIN_SERVICE")
                    .unwrap_or_else(|_| "Claude Code".to_string());
                let account = env::var("OMUX_CLAUDE_KEYCHAIN_ACCOUNT")
                    .unwrap_or_else(|_| "claudeAiOauth".to_string());
                Ok(Box::new(MacOsKeychainCredentialBackend {
                    display_path: PathBuf::from(format!("keychain:{service}/{account}")),
                    service,
                    account,
                }))
            }
            None => Ok(Box::new(PlaintextCredentialBackend {
                path: self.credentials_path()?,
            })),
        }
    }

    fn settings_path(&self) -> Result<PathBuf> {
        Ok(self.claude_home()?.join(SETTINGS_FILE_NAME))
    }

    fn account_snapshot_path(&self, number: u32) -> Result<PathBuf> {
        Ok(self
            .accounts_dir()?
            .join(format!("{number}.credentials.snapshot")))
    }

    fn oauth_account_path(&self, number: u32) -> Result<PathBuf> {
        Ok(self
            .accounts_dir()?
            .join(format!("{number}.oauth-account.json")))
    }

    fn load_registry(&self) -> Result<AccountRegistry> {
        let path = self.registry_path()?;
        if !path.exists() {
            return Ok(AccountRegistry::default());
        }
        let text = String::from_utf8(read_file(&path)?)
            .map_err(|err| OpenMuxError::Message(format!("{}: {err}", display_path(&path))))?;
        let mut registry = parse_account_registry(&path, &text)?;
        if registry.schema_version > REGISTRY_SCHEMA_VERSION {
            return Err(OpenMuxError::Message(format!(
                "registry schema {} is newer than this OpenMux build supports",
                registry.schema_version
            )));
        }
        registry.accounts.sort_by_key(|account| account.number);
        registry.next_account_number = registry.next_account_number.max(
            registry
                .accounts
                .iter()
                .map(|account| account.number)
                .max()
                .unwrap_or(0)
                + 1,
        );
        Ok(registry)
    }

    fn save_registry(&self, registry: &AccountRegistry) -> Result<()> {
        write_file_atomic_private(
            &self.registry_path()?,
            encode_account_registry(registry).as_bytes(),
        )
    }

    fn load_profile_registry(&self) -> Result<Registry> {
        let path = self.profile_registry_path()?;
        if !path.exists() {
            return Ok(Registry::default());
        }
        let text = String::from_utf8(read_file(&path)?)
            .map_err(|err| OpenMuxError::Message(format!("{}: {err}", display_path(&path))))?;
        parse_registry(&path, &text)
    }

    fn save_profile_registry(&self, registry: &Registry) -> Result<()> {
        write_file_atomic_private(
            &self.profile_registry_path()?,
            encode_registry(registry).as_bytes(),
        )
    }

    fn profile_active(&self) -> Result<bool> {
        Ok(self
            .load_profile_registry()?
            .active_profile_number
            .is_some())
    }

    fn deactivate_profile(&self) -> Result<()> {
        let mut registry = self.load_profile_registry()?;
        if registry.active_profile_number.is_none() {
            return Ok(());
        }
        registry.active_profile_number = None;
        self.save_profile_registry(&registry)
    }

    fn account_ref(&self, account: &StoredAccount) -> AccountRef {
        AccountRef {
            platform: self.id().to_string(),
            number: account.number,
            alias: Some(account.name.clone()),
        }
    }

    fn account_status(&self, account: &StoredAccount, active_number: Option<u32>) -> AccountStatus {
        AccountStatus {
            account: self.account_ref(account),
            active: active_number == Some(account.number),
            account_label: account.email.clone(),
            plan_label: None,
            auth_type: Some(if account.partial_metadata {
                "oauth/partial".to_string()
            } else {
                "oauth/full".to_string()
            }),
            expires_at_unix: Some(account.expires_at_unix),
            availability: Availability::unknown(),
            usage: None,
        }
    }

    fn resolve_account<'a>(
        &self,
        registry: &'a AccountRegistry,
        selector: &str,
    ) -> Result<&'a StoredAccount> {
        if let Ok(number) = selector.parse::<u32>() {
            return registry
                .accounts
                .iter()
                .find(|account| account.number == number)
                .ok_or_else(|| OpenMuxError::AccountNotFound {
                    platform: self.id().to_string(),
                    account: selector.to_string(),
                });
        }

        registry
            .accounts
            .iter()
            .find(|account| account.name == selector)
            .ok_or_else(|| OpenMuxError::AccountNotFound {
                platform: self.id().to_string(),
                account: selector.to_string(),
            })
    }

    fn login_with_official_cli(&self, options: LoginOptions) -> Result<AccountRef> {
        let mut command = Command::new(&self.claude_executable);
        command
            .arg("auth")
            .arg("login")
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());
        if options.device_auth {
            command.arg("--device-auth");
        }
        if let Some(path) = &self.claude_home {
            command.env("CLAUDE_CONFIG_DIR", path);
        }

        let status = command.status().map_err(|err| {
            OpenMuxError::Message(format!(
                "failed to run {} auth login: {err}",
                display_path(&self.claude_executable)
            ))
        })?;
        if !status.success() {
            return Err(OpenMuxError::Message(
                "Claude official login did not complete successfully".to_string(),
            ));
        }

        let account = self.import_account(options.alias)?;
        if options.activate {
            self.switch_to(&account.number.to_string())
                .map(|report| report.current)
        } else {
            Ok(account)
        }
    }

    fn import_account(&self, name: Option<String>) -> Result<AccountRef> {
        let backend = self.credential_backend()?;
        if env::var_os("CLAUDE_CODE_OAUTH_TOKEN").is_some() && !backend.exists() {
            return Err(OpenMuxError::Message(
                "CLAUDE_CODE_OAUTH_TOKEN is inference-only and cannot be imported as a full OAuth account snapshot".into(),
            ));
        }

        if !backend.exists() {
            return Err(OpenMuxError::PlatformNotDetected(format!(
                "Claude credentials backend {} at {}",
                backend.label(),
                display_path(backend.location())
            )));
        }
        let credentials = backend.read()?;
        let parsed = parse_credentials(&credentials)?;
        let settings = read_settings_or_empty(&self.settings_path()?)?;
        let oauth_account = settings
            .get("oauthAccount")
            .cloned()
            .unwrap_or(Value::Object(Map::new()));
        let safe = safe_account_metadata(&parsed, &oauth_account);
        let snapshot_hash = sha256_hex(&credentials);
        let duplicate_key = parsed.refresh_token_hash;
        let mut registry = self.load_registry()?;
        let existing_index = registry
            .accounts
            .iter()
            .position(|account| account.snapshot_hash == snapshot_hash);
        let number = if let Some(index) = existing_index {
            registry.accounts[index].number
        } else {
            let number = registry.next_account_number;
            registry.next_account_number += 1;
            number
        };

        let snapshot_path = self.account_snapshot_path(number)?;
        let oauth_path = self.oauth_account_path(number)?;
        write_file_atomic_private(&snapshot_path, &credentials)?;
        let oauth_bytes = serde_json::to_vec_pretty(&oauth_account).map_err(|err| {
            OpenMuxError::Message(format!(
                "failed to encode Claude oauthAccount metadata: {err}"
            ))
        })?;
        write_file_atomic_private(&oauth_path, &oauth_bytes)?;

        let account_name = name
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| {
                safe.email
                    .clone()
                    .unwrap_or_else(|| format!("account-{number}"))
            });
        let stored = StoredAccount {
            number,
            name: sanitize_profile_name(&account_name),
            email: safe.email,
            account_uuid_hash: safe.account_uuid_hash,
            organization_uuid_hash: safe.organization_uuid_hash,
            scopes: parsed.scopes,
            expires_at_unix: parsed.expires_at_unix,
            refresh_token_hash: duplicate_key.clone(),
            snapshot_hash,
            snapshot_path: display_path(&snapshot_path),
            oauth_account_path: display_path(&oauth_path),
            partial_metadata: safe.partial_metadata,
            imported_at_unix: unix_now(),
            last_activated_at_unix: existing_index
                .and_then(|index| registry.accounts[index].last_activated_at_unix),
        };

        if let Some(index) = existing_index.or_else(|| {
            registry.accounts.iter().position(|account| {
                account
                    .account_uuid_hash
                    .as_ref()
                    .zip(stored.account_uuid_hash.as_ref())
                    .is_some_and(|(left, right)| left == right)
                    || account.refresh_token_hash == duplicate_key
            })
        }) {
            registry.accounts[index] = stored;
        } else {
            registry.accounts.push(stored);
        }
        registry.accounts.sort_by_key(|account| account.number);
        let account = registry
            .accounts
            .iter()
            .find(|account| account.number == number)
            .expect("imported account should exist");
        let account_ref = self.account_ref(account);
        self.save_registry(&registry)?;
        Ok(account_ref)
    }
}

impl PlatformPlugin for ClaudeAccountPlugin {
    fn id(&self) -> &'static str {
        "claude-account"
    }

    fn name(&self) -> &'static str {
        "Claude Account"
    }

    fn capabilities(&self) -> PlatformCapabilities {
        PlatformCapabilities {
            accounts: true,
            account_login: true,
            account_import: true,
            ..PlatformCapabilities::default()
        }
    }

    fn detect(&self) -> Result<PlatformInstall> {
        let backend = self.credential_backend()?;
        let settings_path = self.settings_path()?;
        Ok(PlatformInstall {
            platform: self.info(),
            config_path: settings_path.exists().then(|| display_path(&settings_path)),
            auth_path: backend.exists().then(|| display_path(backend.location())),
        })
    }

    fn pool_summary(&self) -> Result<PlatformPoolSummary> {
        let registry = self.load_registry()?;
        let active_number = if self.profile_active()? {
            None
        } else {
            registry.active_account_number
        };
        let active = active_number
            .and_then(|number| {
                registry
                    .accounts
                    .iter()
                    .find(|account| account.number == number)
            })
            .map(|account| self.account_ref(account));
        Ok(PlatformPoolSummary {
            platform: self.info(),
            account_count: registry.accounts.len(),
            active,
            profile_count: 0,
            active_profile: None,
            availability: Availability::unknown(),
        })
    }

    fn current(&self) -> Result<Option<AccountStatus>> {
        if self.profile_active()? {
            return Ok(None);
        }
        let registry = self.load_registry()?;
        Ok(registry.active_account_number.and_then(|number| {
            registry
                .accounts
                .iter()
                .find(|account| account.number == number)
                .map(|account| self.account_status(account, Some(number)))
        }))
    }

    fn list_accounts(&self) -> Result<Vec<AccountStatus>> {
        let registry = self.load_registry()?;
        let active_number = if self.profile_active()? {
            None
        } else {
            registry.active_account_number
        };
        Ok(registry
            .accounts
            .iter()
            .map(|account| self.account_status(account, active_number))
            .collect())
    }

    fn login(&self, options: LoginOptions) -> Result<AccountRef> {
        self.login_with_official_cli(options)
    }

    fn save_current(&self, _options: SaveOptions) -> Result<AccountRef> {
        self.import_account(None)
    }

    fn import_config(&self, options: ImportConfigOptions) -> Result<ImportedConfig> {
        let account = self.import_account(options.name)?;
        Ok(ImportedConfig {
            platform: self.info(),
            profile_name: account.alias.unwrap_or_else(|| account.number.to_string()),
            config_path: display_path(&self.account_snapshot_path(account.number)?),
            provider_id: Some("claude-ai-oauth".to_string()),
            base_url: None,
            model: None,
            number: Some(account.number),
            auth_type: Some("oauth".to_string()),
        })
    }

    fn use_target(&self, selector: &str) -> Result<UseReport> {
        self.switch_to(selector).map(UseReport::Account)
    }

    fn switch_to(&self, selector: &str) -> Result<SwitchReport> {
        let mut registry = self.load_registry()?;
        let account = self.resolve_account(&registry, selector)?.clone();
        let snapshot_path = PathBuf::from(&account.snapshot_path);
        let snapshot = read_file(&snapshot_path)?;
        if sha256_hex(&snapshot) != account.snapshot_hash {
            return Err(OpenMuxError::Message(format!(
                "stored Claude account #{} failed hash verification",
                account.number
            )));
        }
        let oauth_account = read_file(Path::new(&account.oauth_account_path))?;
        let oauth_value: Value = serde_json::from_slice(&oauth_account).map_err(|err| {
            OpenMuxError::Message(format!("invalid stored oauthAccount metadata: {err}"))
        })?;
        let backend = self.credential_backend()?;
        let settings_path = self.settings_path()?;
        let current_credentials = if backend.exists() {
            Some(backend.read()?)
        } else {
            None
        };
        let current_settings = if settings_path.exists() {
            Some(read_file(&settings_path)?)
        } else {
            None
        };
        let credential_backup = if let Some(current) = &current_credentials {
            let path = self
                .backups_dir()?
                .join(format!("credentials.snapshot.bak.{}", unix_now_nanos()));
            if let Some(parent) = path.parent() {
                create_dir_private(parent)?;
            }
            write_file_atomic_private(&path, current)?;
            Some(path)
        } else {
            None
        };
        let settings_backup = if let Some(current) = &current_settings {
            let path = self
                .backups_dir()?
                .join(format!("settings.json.bak.{}", unix_now_nanos()));
            if let Some(parent) = path.parent() {
                create_dir_private(parent)?;
            }
            write_file_atomic_private(&path, current)?;
            Some(path)
        } else {
            None
        };

        backend.write(&snapshot)?;
        let mut settings = current_settings
            .as_deref()
            .map(parse_settings)
            .transpose()?
            .unwrap_or_else(|| Value::Object(Map::new()));
        set_oauth_account(&mut settings, oauth_value);
        let settings_bytes = serde_json::to_vec_pretty(&settings).map_err(|err| {
            OpenMuxError::Message(format!("failed to encode Claude settings: {err}"))
        })?;
        #[cfg(test)]
        if self.fail_settings_write {
            let _ = backend.restore(current_credentials.as_deref());
            return Err(OpenMuxError::Message(
                "failed to update Claude oauthAccount metadata; credential rollback attempted: injected settings write failure".to_string(),
            ));
        }
        if let Err(err) = write_file_atomic_private(&settings_path, &settings_bytes) {
            let _ = backend.restore(current_credentials.as_deref());
            return Err(OpenMuxError::Message(format!(
                "failed to update Claude oauthAccount metadata; credential rollback attempted: {err}"
            )));
        }

        let previous = registry
            .active_account_number
            .filter(|number| *number != account.number)
            .and_then(|number| {
                registry
                    .accounts
                    .iter()
                    .find(|stored| stored.number == number)
                    .map(|stored| self.account_ref(stored))
            });
        registry.active_account_number = Some(account.number);
        if let Some(stored) = registry
            .accounts
            .iter_mut()
            .find(|stored| stored.number == account.number)
        {
            stored.last_activated_at_unix = Some(unix_now());
        }
        if let Err(err) = self.save_registry(&registry) {
            let credential_rollback = backend.restore(current_credentials.as_deref());
            let settings_rollback = rollback_file(&settings_path, current_settings.as_deref());
            return Err(OpenMuxError::Message(format!(
                "failed to update Claude account registry after switching; credential rollback: {}; settings rollback: {}; error: {}; backups: {}, {}",
                rollback_status(credential_rollback),
                rollback_status(settings_rollback),
                err,
                credential_backup
                    .as_deref()
                    .map(display_path)
                    .unwrap_or_else(|| "none".to_string()),
                settings_backup
                    .as_deref()
                    .map(display_path)
                    .unwrap_or_else(|| "none".to_string())
            )));
        }
        self.deactivate_profile()?;

        Ok(SwitchReport {
            previous,
            current: self.account_ref(&account),
        })
    }

    fn set_alias(&self, _selector: &str, _alias: &str) -> Result<AccountRef> {
        Err(OpenMuxError::Message(
            "renaming Claude accounts is not implemented yet; re-import with --name".into(),
        ))
    }

    fn doctor(&self) -> Result<DoctorReport> {
        let backend = self.credential_backend()?;
        let registry_path = self.registry_path()?;
        let registry = self.load_registry();
        Ok(DoctorReport {
            platform: self.id().to_string(),
            checks: vec![
                DoctorCheck {
                    name: format!("{}-credentials", backend.label()),
                    ok: backend.exists(),
                    message: display_path(backend.location()),
                },
                DoctorCheck {
                    name: "account-registry".to_string(),
                    ok: registry.is_ok(),
                    message: if registry_path.exists() {
                        display_path(&registry_path)
                    } else {
                        "not created yet".to_string()
                    },
                },
            ],
        })
    }
}

fn deferred_account_error() -> OpenMuxError {
    OpenMuxError::Message(
        "Claude OAuth account switching is deferred; use `omx import claude` for profiles"
            .to_string(),
    )
}

#[derive(Debug, Clone)]
struct ParsedCredentials {
    expires_at_unix: i64,
    scopes: Option<String>,
    refresh_token_hash: String,
}

#[derive(Debug, Clone)]
struct SafeAccountMetadata {
    email: Option<String>,
    account_uuid_hash: Option<String>,
    organization_uuid_hash: Option<String>,
    partial_metadata: bool,
}

fn parse_credentials(bytes: &[u8]) -> Result<ParsedCredentials> {
    let value: Value = serde_json::from_slice(bytes)
        .map_err(|err| OpenMuxError::Message(format!("invalid Claude credentials JSON: {err}")))?;
    let oauth = value
        .get("claudeAiOauth")
        .or_else(|| value.pointer("/secureStorage/claudeAiOauth"))
        .ok_or_else(|| {
            OpenMuxError::Message("Claude credentials do not contain claudeAiOauth".into())
        })?;
    let access_token = oauth
        .get("accessToken")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty());
    let refresh_token = oauth
        .get("refreshToken")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            OpenMuxError::Message(
                "Claude OAuth account snapshot requires a refresh token; inference-only tokens cannot be imported".into(),
            )
        })?;
    if access_token.is_none() {
        return Err(OpenMuxError::Message(
            "Claude OAuth account snapshot requires an access token".into(),
        ));
    }
    let expires_at_unix = oauth
        .get("expiresAt")
        .and_then(json_number_as_i64)
        .ok_or_else(|| {
            OpenMuxError::Message(
                "Claude OAuth account snapshot requires expiresAt; inference-only tokens cannot be imported".into(),
            )
        })?;
    let scopes = oauth.get("scopes").and_then(|value| {
        value.as_array().map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(" ")
        })
    });
    Ok(ParsedCredentials {
        expires_at_unix,
        scopes,
        refresh_token_hash: sha256_hex(refresh_token.as_bytes()),
    })
}

fn safe_account_metadata(
    _credentials: &ParsedCredentials,
    oauth_account: &Value,
) -> SafeAccountMetadata {
    let email = oauth_account
        .get("email")
        .or_else(|| oauth_account.get("accountEmail"))
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(mask_email);
    let account_uuid_hash = oauth_account
        .get("accountUuid")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(short_hash);
    let organization_uuid_hash = oauth_account
        .get("organizationUuid")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(short_hash);
    let partial_metadata = email.is_none() || account_uuid_hash.is_none();
    SafeAccountMetadata {
        email,
        account_uuid_hash,
        organization_uuid_hash,
        partial_metadata,
    }
}

fn read_settings_or_empty(path: &Path) -> Result<Value> {
    if path.exists() {
        parse_settings(&read_file(path)?)
    } else {
        Ok(Value::Object(Map::new()))
    }
}

fn set_oauth_account(settings: &mut Value, oauth_account: Value) {
    if !settings.is_object() {
        *settings = Value::Object(Map::new());
    }
    settings
        .as_object_mut()
        .expect("settings should be object")
        .insert("oauthAccount".to_string(), oauth_account);
}

fn rollback_file(path: &Path, bytes: Option<&[u8]>) -> Result<()> {
    match bytes {
        Some(bytes) => write_file_atomic_private(path, bytes),
        None => fs::remove_file(path).map_err(|err| io_error(path, err)),
    }
}

fn rollback_status(result: Result<()>) -> &'static str {
    if result.is_ok() { "ok" } else { "failed" }
}

fn mask_email(value: &str) -> String {
    let Some((name, domain)) = value.split_once('@') else {
        return "redacted".to_string();
    };
    let first = name.chars().next().unwrap_or('*');
    format!("{first}***@{domain}")
}

fn short_hash(value: &str) -> String {
    sha256_hex(value.as_bytes()).chars().take(12).collect()
}

fn json_number_as_i64(value: &Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_u64().and_then(|value| i64::try_from(value).ok()))
        .or_else(|| value.as_f64().map(|value| value.round() as i64))
}

fn parse_profile(content: &str, requested_name: Option<&str>) -> Result<ParsedProfile> {
    let raw = content.trim();
    if raw.is_empty() {
        return Err(OpenMuxError::Message(
            "missing Claude profile content to import".into(),
        ));
    }
    let vars = if raw.starts_with('{') {
        parse_json_vars(raw)?
    } else if raw.lines().any(|line| line.trim_start().starts_with('[')) {
        parse_toml_vars(raw)?
    } else {
        parse_shell_like_kv(raw)?
    };
    let env: BTreeMap<String, String> = vars
        .into_iter()
        .filter(|(key, value)| MANAGED_ENV_KEYS.contains(&key.as_str()) && !value.is_empty())
        .collect();
    if env.is_empty() {
        return Err(OpenMuxError::Message(
            "Claude profile import needs Anthropic or Claude Code env keys".into(),
        ));
    }
    let base_url = env.get("ANTHROPIC_BASE_URL").cloned();
    let model = env.get("ANTHROPIC_MODEL").cloned();
    let auth_type = if env.contains_key("ANTHROPIC_AUTH_TOKEN") {
        "bearer-token"
    } else if env.contains_key("ANTHROPIC_API_KEY") {
        "api-key"
    } else if env
        .keys()
        .any(|key| key.contains("BEDROCK") || key.contains("VERTEX") || key.contains("FOUNDRY"))
    {
        "cloud-provider"
    } else {
        "none"
    }
    .to_string();
    let name = resolve_profile_name(
        requested_name,
        base_url.as_deref(),
        env.get("OMUX_PROFILE").map(String::as_str),
        "claude-profile",
    )?;
    Ok(ParsedProfile {
        name,
        auth_type,
        base_url,
        model,
        env,
    })
}

fn parse_json_vars(content: &str) -> Result<Vec<(String, String)>> {
    let value: Value = serde_json::from_str(content)
        .map_err(|err| OpenMuxError::Message(format!("invalid Claude JSON profile: {err}")))?;
    let object = value
        .as_object()
        .ok_or_else(|| OpenMuxError::Message("Claude JSON profile must be an object".into()))?;
    Ok(object
        .iter()
        .filter_map(|(key, value)| value.as_str().map(|value| (key.clone(), value.to_string())))
        .collect())
}

fn parse_toml_vars(content: &str) -> Result<Vec<(String, String)>> {
    let value: toml::Value = toml::from_str(content)
        .map_err(|err| OpenMuxError::Message(format!("invalid Claude TOML profile: {err}")))?;
    let Some(table) = value.as_table() else {
        return Ok(Vec::new());
    };
    Ok(table
        .iter()
        .filter_map(|(key, value)| value.as_str().map(|value| (key.clone(), value.to_string())))
        .collect())
}

fn parse_shell_like_kv(content: &str) -> Result<Vec<(String, String)>> {
    let mut vars = Vec::new();
    for token in shell_like_tokens(content) {
        let token = token.strip_prefix("export ").unwrap_or(&token);
        let Some((key, value)) = token.split_once('=') else {
            continue;
        };
        let key = key.trim();
        if key.is_empty()
            || !key
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        {
            continue;
        }
        vars.push((key.to_string(), unquote_kv_value(value.trim()).to_string()));
    }
    if vars.is_empty() {
        return Err(OpenMuxError::Message(
            "could not find KEY=VALUE pairs to import".into(),
        ));
    }
    Ok(vars)
}

fn shell_like_tokens(content: &str) -> Vec<String> {
    content
        .lines()
        .flat_map(|line| {
            let line = line.trim();
            if line.starts_with("export ") || line.matches('=').count() <= 1 {
                vec![line.to_string()]
            } else {
                line.split_whitespace().map(str::to_string).collect()
            }
        })
        .collect()
}

fn unquote_kv_value(value: &str) -> &str {
    if value.len() >= 2
        && ((value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('\'') && value.ends_with('\'')))
    {
        &value[1..value.len() - 1]
    } else {
        value
    }
}

fn resolve_profile_name(
    requested_name: Option<&str>,
    base_url: Option<&str>,
    explicit_name: Option<&str>,
    fallback: &str,
) -> Result<String> {
    let raw = requested_name
        .or(explicit_name)
        .or_else(|| base_url.and_then(host_from_url))
        .unwrap_or(fallback);
    let name = sanitize_profile_name(raw);
    if name.is_empty() {
        return Err(OpenMuxError::Message("profile name cannot be empty".into()));
    }
    Ok(name)
}

fn sanitize_profile_name(value: &str) -> String {
    let mut name = String::new();
    let mut last_was_dash = false;
    for byte in value.bytes() {
        let ch = byte.to_ascii_lowercase() as char;
        if byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-') {
            name.push(ch);
            last_was_dash = false;
        } else if !last_was_dash {
            name.push('-');
            last_was_dash = true;
        }
    }
    name.trim_matches('-').to_string()
}

fn host_from_url(url: &str) -> Option<&str> {
    let without_scheme = url.split_once("://").map(|(_, rest)| rest).unwrap_or(url);
    let host = without_scheme
        .split(['/', '?', '#'])
        .next()
        .unwrap_or(without_scheme)
        .split('@')
        .next_back()
        .unwrap_or(without_scheme)
        .split(':')
        .next()
        .unwrap_or(without_scheme);
    (!host.is_empty()).then_some(host)
}

fn parse_settings(bytes: &[u8]) -> Result<Value> {
    serde_json::from_slice(bytes)
        .map_err(|err| OpenMuxError::Message(format!("invalid Claude settings JSON: {err}")))
}

fn apply_env_patch(settings: &mut Value, env: &BTreeMap<String, String>) -> Result<()> {
    if !settings.is_object() {
        *settings = Value::Object(Map::new());
    }
    let object = settings.as_object_mut().expect("settings should be object");
    let env_value = object
        .entry("env")
        .or_insert_with(|| Value::Object(Map::new()));
    if !env_value.is_object() {
        *env_value = Value::Object(Map::new());
    }
    let env_object = env_value.as_object_mut().expect("env should be object");
    for key in MANAGED_ENV_KEYS {
        env_object.remove(*key);
    }
    for (key, value) in env {
        env_object.insert(key.clone(), Value::String(value.clone()));
    }
    Ok(())
}

#[cfg(target_os = "macos")]
mod macos_keychain {
    use omx_core::{OpenMuxError, Result};
    use std::{
        ffi::{c_char, c_void},
        ptr, slice,
    };

    const ERR_SEC_SUCCESS: i32 = 0;
    const ERR_SEC_ITEM_NOT_FOUND: i32 = -25300;
    const ERR_SEC_DUPLICATE_ITEM: i32 = -25299;

    #[link(name = "Security", kind = "framework")]
    unsafe extern "C" {
        fn SecKeychainFindGenericPassword(
            keychain_or_array: *mut c_void,
            service_name_length: u32,
            service_name: *const c_char,
            account_name_length: u32,
            account_name: *const c_char,
            password_length: *mut u32,
            password_data: *mut *mut c_void,
            item_ref: *mut *mut c_void,
        ) -> i32;

        fn SecKeychainAddGenericPassword(
            keychain: *mut c_void,
            service_name_length: u32,
            service_name: *const c_char,
            account_name_length: u32,
            account_name: *const c_char,
            password_length: u32,
            password_data: *const c_void,
            item_ref: *mut *mut c_void,
        ) -> i32;

        fn SecKeychainItemModifyAttributesAndData(
            item_ref: *mut c_void,
            attr_list: *const c_void,
            length: u32,
            data: *const c_void,
        ) -> i32;

        fn SecKeychainItemDelete(item_ref: *mut c_void) -> i32;

        fn SecKeychainItemFreeContent(attr_list: *mut c_void, data: *mut c_void) -> i32;
    }

    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        fn CFRelease(cf: *const c_void);
    }

    pub(super) fn exists(service: &str, account: &str) -> bool {
        find_item(service, account).is_ok()
    }

    pub(super) fn read(service: &str, account: &str) -> Result<Vec<u8>> {
        let service = checked_bytes(service, "Claude Keychain service")?;
        let account = checked_bytes(account, "Claude Keychain account")?;
        let mut password_length = 0u32;
        let mut password_data: *mut c_void = ptr::null_mut();
        let mut item_ref: *mut c_void = ptr::null_mut();
        let status = unsafe {
            SecKeychainFindGenericPassword(
                ptr::null_mut(),
                service.len() as u32,
                service.as_ptr().cast(),
                account.len() as u32,
                account.as_ptr().cast(),
                &mut password_length,
                &mut password_data,
                &mut item_ref,
            )
        };
        if status == ERR_SEC_ITEM_NOT_FOUND {
            return Err(OpenMuxError::Message(
                "Claude Keychain credential was not found".to_string(),
            ));
        }
        if status != ERR_SEC_SUCCESS {
            return Err(keychain_error("read Claude Keychain credential", status));
        }

        let bytes = unsafe {
            slice::from_raw_parts(password_data.cast::<u8>(), password_length as usize).to_vec()
        };
        unsafe {
            let _ = SecKeychainItemFreeContent(ptr::null_mut(), password_data);
            if !item_ref.is_null() {
                CFRelease(item_ref.cast_const());
            }
        }
        Ok(bytes)
    }

    pub(super) fn write(service: &str, account: &str, bytes: &[u8]) -> Result<()> {
        let service = checked_bytes(service, "Claude Keychain service")?;
        let account = checked_bytes(account, "Claude Keychain account")?;
        let password_length = checked_len(bytes.len(), "Claude Keychain payload")?;
        let status = unsafe {
            SecKeychainAddGenericPassword(
                ptr::null_mut(),
                service.len() as u32,
                service.as_ptr().cast(),
                account.len() as u32,
                account.as_ptr().cast(),
                password_length,
                bytes.as_ptr().cast(),
                ptr::null_mut(),
            )
        };
        match status {
            ERR_SEC_SUCCESS => Ok(()),
            ERR_SEC_DUPLICATE_ITEM => {
                let item = find_item_bytes(service, account)?;
                let status = unsafe {
                    SecKeychainItemModifyAttributesAndData(
                        item.as_ptr(),
                        ptr::null(),
                        password_length,
                        bytes.as_ptr().cast(),
                    )
                };
                if status == ERR_SEC_SUCCESS {
                    Ok(())
                } else {
                    Err(keychain_error("update Claude Keychain credential", status))
                }
            }
            other => Err(keychain_error("write Claude Keychain credential", other)),
        }
    }

    pub(super) fn delete(service: &str, account: &str) -> Result<()> {
        match find_item(service, account) {
            Ok(item) => {
                let status = unsafe { SecKeychainItemDelete(item.as_ptr()) };
                if status == ERR_SEC_SUCCESS || status == ERR_SEC_ITEM_NOT_FOUND {
                    Ok(())
                } else {
                    Err(keychain_error("delete Claude Keychain credential", status))
                }
            }
            Err(OpenMuxError::Message(message)) if message.contains("not found") => Ok(()),
            Err(err) => Err(err),
        }
    }

    fn find_item(service: &str, account: &str) -> Result<KeychainItem> {
        let service = checked_bytes(service, "Claude Keychain service")?;
        let account = checked_bytes(account, "Claude Keychain account")?;
        find_item_bytes(service, account)
    }

    fn find_item_bytes(service: &[u8], account: &[u8]) -> Result<KeychainItem> {
        let mut item_ref: *mut c_void = ptr::null_mut();
        let status = unsafe {
            SecKeychainFindGenericPassword(
                ptr::null_mut(),
                service.len() as u32,
                service.as_ptr().cast(),
                account.len() as u32,
                account.as_ptr().cast(),
                ptr::null_mut(),
                ptr::null_mut(),
                &mut item_ref,
            )
        };
        if status == ERR_SEC_ITEM_NOT_FOUND {
            return Err(OpenMuxError::Message(
                "Claude Keychain credential was not found".to_string(),
            ));
        }
        if status != ERR_SEC_SUCCESS {
            return Err(keychain_error("find Claude Keychain credential", status));
        }
        Ok(KeychainItem(item_ref))
    }

    fn checked_bytes<'a>(value: &'a str, label: &str) -> Result<&'a [u8]> {
        let bytes = value.as_bytes();
        checked_len(bytes.len(), label)?;
        Ok(bytes)
    }

    fn checked_len(len: usize, label: &str) -> Result<u32> {
        u32::try_from(len)
            .map_err(|_| OpenMuxError::Message(format!("{label} is too large for Keychain API")))
    }

    fn keychain_error(action: &str, status: i32) -> OpenMuxError {
        OpenMuxError::Message(format!("failed to {action}: OSStatus {status}"))
    }

    struct KeychainItem(*mut c_void);

    impl KeychainItem {
        fn as_ptr(&self) -> *mut c_void {
            self.0
        }
    }

    impl Drop for KeychainItem {
        fn drop(&mut self) {
            if !self.0.is_null() {
                unsafe { CFRelease(self.0.cast_const()) };
            }
        }
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
