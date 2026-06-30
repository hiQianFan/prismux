use omx_core::{
    AccountRecord, AccountRef, AccountStatus, Availability, ConfigProfile, ConfigSwitchReport,
    DoctorCheck, DoctorReport, ImportConfigOptions, ImportedConfig, LOGIN_TIMEOUT, LoginOptions,
    OpenMuxError, PlatformCapabilities, PlatformInfo, PlatformInstall, PlatformPlugin,
    PlatformPoolSummary, ProfileRecord, RemoveReport, RemovedAccount, RemovedConfig, Result,
    SaveOptions, StateStore, SwitchReport, TargetCatalog, TargetKind, TargetResolution,
    UpsertAccount, UpsertProfile, UseReport, platform_info, run_cancellable_login,
    storage::{
        create_dir_private, display_path, home_dir, io_error, prune_backup_files, read_file,
        sha256_hex, state_root as default_state_root, unix_now, unix_now_nanos,
        write_file_atomic_private,
    },
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

const CLAUDE_STATE_PROVIDER: &str = "claude";
const SETTINGS_FILE_NAME: &str = "settings.json";
const BACKUP_RETENTION_PER_KIND: usize = 3;
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

#[derive(Debug, Clone)]
pub struct ClaudePlugin {
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

impl Default for ClaudePlugin {
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
            ..Self::default()
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

        default_state_root()
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

    fn state_store(&self) -> Result<StateStore> {
        StateStore::open(&self.state_root()?)
    }

    fn settings_path(&self) -> Result<PathBuf> {
        Ok(self.claude_home()?.join(SETTINGS_FILE_NAME))
    }

    fn profile_snapshot_path(&self, secret_hash: &str) -> Result<PathBuf> {
        Ok(self
            .profiles_dir()?
            .join(format!("{secret_hash}.profile.json")))
    }

    #[cfg(test)]
    fn profile_snapshot_path_for_number(&self, number: u32) -> Result<PathBuf> {
        let profile = self
            .state_store()?
            .profile_by_selector(CLAUDE_STATE_PROVIDER, &number.to_string())?
            .ok_or_else(|| OpenMuxError::AccountNotFound {
                platform: self.id().to_string(),
                account: number.to_string(),
            })?;
        Ok(PathBuf::from(profile.secret_ref))
    }

    fn resolve_profile(&self, selector: &str) -> Result<ProfileRecord> {
        let store = self.state_store()?;
        if let Some(profile) = store.profile_by_local_id(selector)?
            && profile.provider == CLAUDE_STATE_PROVIDER
        {
            return Ok(profile);
        }
        store
            .profile_by_selector(CLAUDE_STATE_PROVIDER, selector)?
            .ok_or_else(|| OpenMuxError::AccountNotFound {
                platform: self.id().to_string(),
                account: selector.to_string(),
            })
    }

    fn profile_status(
        &self,
        profile: &ProfileRecord,
        active_local_id: Option<&str>,
    ) -> ConfigProfile {
        profile.to_config_profile(
            self.info(),
            active_local_id == Some(profile.local_id.as_str()),
        )
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
        let now = unix_now();
        let snapshot_path = self.profile_snapshot_path(&secret_hash)?;
        write_file_atomic_private(&snapshot_path, &snapshot_bytes)?;
        let profile = self.state_store()?.upsert_profile(UpsertProfile {
            provider: CLAUDE_STATE_PROVIDER.to_string(),
            name: parsed.name.clone(),
            label: None,
            profile_kind: "env".to_string(),
            provider_id: None,
            base_url: parsed.base_url.clone(),
            model: parsed.model.clone(),
            auth_type: Some(parsed.auth_type.clone()),
            config_hash: secret_hash,
            secret_ref: display_path(&snapshot_path),
            imported_at_unix: now,
        })?;

        Ok(ImportedConfig {
            platform: self.info(),
            profile_name: parsed.name,
            config_path: display_path(&snapshot_path),
            provider_id: None,
            base_url: parsed.base_url,
            model: parsed.model,
            number: profile.display_number,
            auth_type: Some(parsed.auth_type),
        })
    }

    fn use_profile(&self, selector: &str) -> Result<ConfigSwitchReport> {
        let profile = self.resolve_profile(selector)?;
        let snapshot_path = PathBuf::from(&profile.secret_ref);
        let snapshot_bytes = read_file(&snapshot_path)?;
        if sha256_hex(&snapshot_bytes) != profile.config_hash {
            return Err(OpenMuxError::Message(format!(
                "stored Claude profile #{} failed hash verification",
                profile.display_number.unwrap_or_default()
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
                write_file_atomic_private(&path, current)?;
                prune_backup_files(
                    &self.backups_dir()?,
                    "settings.json.bak.",
                    BACKUP_RETENTION_PER_KIND,
                )?;
                Some(display_path(&path))
            } else {
                None
            }
        } else {
            None
        };

        if let Err(err) = write_file_atomic_private(&settings_path, &next_bytes) {
            return Err(OpenMuxError::Message(format!(
                "failed to apply Claude profile settings: {err}"
            )));
        }
        if let Err(err) = self.state_store()?.set_active_profile(
            CLAUDE_STATE_PROVIDER,
            &profile.local_id,
            unix_now(),
        ) {
            let rollback = match current_bytes {
                Some(bytes) => write_file_atomic_private(&settings_path, &bytes),
                None => fs::remove_file(&settings_path)
                    .map_err(|remove_err| io_error(&settings_path, remove_err)),
            };
            return match rollback {
                Ok(()) => Err(OpenMuxError::Message(format!(
                    "failed to update state store after applying profile; settings were rolled back: {err}"
                ))),
                Err(rollback_err) => Err(OpenMuxError::Message(format!(
                    "failed to update state store after applying profile and rollback failed: {err}; rollback error: {rollback_err}"
                ))),
            };
        }

        Ok(ConfigSwitchReport {
            platform: self.info(),
            profile: self.profile_status(&profile, Some(&profile.local_id)),
            config_path: display_path(&settings_path),
            backup_path,
        })
    }

    fn remove_profile(&self, selector: &str) -> Result<RemovedConfig> {
        let store = self.state_store()?;
        let profile = self.resolve_profile(selector)?;
        let was_active = store
            .active_profile(CLAUDE_STATE_PROVIDER)?
            .is_some_and(|active| active.local_id == profile.local_id);
        let profile_status =
            self.profile_status(&profile, was_active.then_some(profile.local_id.as_str()));
        let mut removed_paths = Vec::new();

        remove_file_if_exists(Path::new(&profile.secret_ref), &mut removed_paths)?;
        store.remove_profile(&profile.local_id)?;

        Ok(RemovedConfig {
            profile: profile_status,
            was_active,
            removed_paths,
        })
    }
}

impl ClaudePlugin {
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

    fn accounts_dir(&self) -> Result<PathBuf> {
        Ok(self.platform_state_dir()?.join("accounts"))
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

    fn account_snapshot_path(&self, snapshot_hash: &str) -> Result<PathBuf> {
        Ok(self
            .accounts_dir()?
            .join(format!("{snapshot_hash}.credentials.snapshot")))
    }

    fn oauth_account_path(&self, snapshot_hash: &str) -> Result<PathBuf> {
        Ok(self
            .accounts_dir()?
            .join(format!("{snapshot_hash}.oauth-account.json")))
    }

    fn account_snapshot_hash(account: &AccountRecord) -> Result<String> {
        Path::new(&account.secret_ref)
            .file_name()
            .and_then(|name| name.to_str())
            .and_then(|name| name.strip_suffix(".credentials.snapshot"))
            .filter(|value| !value.is_empty())
            .map(ToString::to_string)
            .ok_or_else(|| {
                OpenMuxError::Message(format!(
                    "invalid Claude account secret_ref `{}`",
                    account.secret_ref
                ))
            })
    }

    fn oauth_account_path_for_record(&self, account: &AccountRecord) -> Result<PathBuf> {
        self.oauth_account_path(&Self::account_snapshot_hash(account)?)
    }

    #[cfg(test)]
    fn account_snapshot_path_for_number(&self, number: u32) -> Result<PathBuf> {
        let account = self
            .state_store()?
            .account_by_selector(CLAUDE_STATE_PROVIDER, &number.to_string())?
            .ok_or_else(|| OpenMuxError::AccountNotFound {
                platform: self.id().to_string(),
                account: number.to_string(),
            })?;
        Ok(PathBuf::from(account.secret_ref))
    }

    #[cfg(test)]
    fn oauth_account_path_for_number(&self, number: u32) -> Result<PathBuf> {
        let account = self
            .state_store()?
            .account_by_selector(CLAUDE_STATE_PROVIDER, &number.to_string())?
            .ok_or_else(|| OpenMuxError::AccountNotFound {
                platform: self.id().to_string(),
                account: number.to_string(),
            })?;
        self.oauth_account_path_for_record(&account)
    }

    fn account_ref(&self, account: &AccountRecord) -> AccountRef {
        AccountRef {
            platform: self.id().to_string(),
            local_id: account.local_id.clone(),
            number: account.display_number,
            alias: account.alias.clone(),
        }
    }

    fn account_status(
        &self,
        account: &AccountRecord,
        active_local_id: Option<&str>,
    ) -> AccountStatus {
        AccountStatus {
            account: self.account_ref(account),
            active: active_local_id == Some(account.local_id.as_str()),
            account_label: account.account_label.clone(),
            plan_label: None,
            auth_type: account.auth_type.clone(),
            expires_at_unix: account.expires_at_unix,
            availability: Availability::unknown(),
            usage: None,
        }
    }

    fn resolve_account(&self, selector: &str) -> Result<AccountRecord> {
        let store = self.state_store()?;
        if let Some(account) = store.account_by_local_id(selector)?
            && account.provider == CLAUDE_STATE_PROVIDER
        {
            return Ok(account);
        }
        store
            .account_by_selector(CLAUDE_STATE_PROVIDER, selector)?
            .ok_or_else(|| OpenMuxError::AccountNotFound {
                platform: self.id().to_string(),
                account: selector.to_string(),
            })
    }

    fn resolve_target(&self, selector: &str) -> Result<TargetResolution> {
        let store = self.state_store()?;
        if let Some(account) = store.account_by_local_id(selector)?
            && account.provider == CLAUDE_STATE_PROVIDER
        {
            return Ok(TargetResolution {
                kind: TargetKind::Account,
                target_id: account.local_id,
            });
        }
        if let Some(profile) = store.profile_by_local_id(selector)?
            && profile.provider == CLAUDE_STATE_PROVIDER
        {
            return Ok(TargetResolution {
                kind: TargetKind::Profile,
                target_id: profile.local_id,
            });
        }
        TargetCatalog::new(self.list_accounts()?, self.list_configs()?).resolve(self.id(), selector)
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

        let status = run_cancellable_login(
            &mut command,
            LOGIN_TIMEOUT,
            &display_path(&self.claude_executable),
        )?;
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
        let snapshot_path = self.account_snapshot_path(&snapshot_hash)?;
        let oauth_path = self.oauth_account_path(&snapshot_hash)?;
        write_file_atomic_private(&snapshot_path, &credentials)?;
        let oauth_bytes = serde_json::to_vec_pretty(&oauth_account).map_err(|err| {
            OpenMuxError::Message(format!(
                "failed to encode Claude oauthAccount metadata: {err}"
            ))
        })?;
        write_file_atomic_private(&oauth_path, &oauth_bytes)?;

        let account_name = name
            .filter(|value| !value.trim().is_empty())
            .or(safe.email.clone())
            .unwrap_or_else(|| "claude".to_string());
        let account = self.state_store()?.upsert_account(UpsertAccount {
            provider: CLAUDE_STATE_PROVIDER.to_string(),
            alias: Some(sanitize_profile_name(&account_name)),
            provider_subject_kind: None,
            provider_subject_hash: None,
            provider_subject_label: None,
            account_label: safe.email,
            plan_label: None,
            auth_type: Some(if safe.partial_metadata {
                "oauth/partial".to_string()
            } else {
                "oauth/full".to_string()
            }),
            expires_at_unix: Some(parsed.expires_at_unix),
            auth_hash: duplicate_key,
            secret_ref: display_path(&snapshot_path),
            imported_at_unix: unix_now(),
        })?;
        Ok(self.account_ref(&account))
    }

    fn remove_account(&self, selector: &str) -> Result<RemovedAccount> {
        let store = self.state_store()?;
        let account = self.resolve_account(selector)?;
        let was_active = store
            .active_account(CLAUDE_STATE_PROVIDER)?
            .is_some_and(|active| active.local_id == account.local_id);
        let mut removed_paths = Vec::new();

        remove_file_if_exists(Path::new(&account.secret_ref), &mut removed_paths)?;
        remove_file_if_exists(
            &self.oauth_account_path_for_record(&account)?,
            &mut removed_paths,
        )?;
        store.remove_account(&account.local_id)?;

        Ok(RemovedAccount {
            account: self.account_ref(&account),
            was_active,
            removed_paths,
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
            accounts: true,
            account_login: true,
            account_import: true,
            profiles: true,
            profile_import: true,
            account_save: true,
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
        let store = self.state_store()?;
        let accounts = store.list_accounts(CLAUDE_STATE_PROVIDER)?;
        let active = store
            .active_account(CLAUDE_STATE_PROVIDER)?
            .map(|account| self.account_ref(&account));
        let profiles = store.list_profiles(CLAUDE_STATE_PROVIDER)?;
        let active_profile = store
            .active_profile(CLAUDE_STATE_PROVIDER)?
            .map(|profile| profile.name);
        Ok(PlatformPoolSummary {
            platform: self.info(),
            account_count: accounts.len(),
            active,
            profile_count: profiles.len(),
            active_profile,
            availability: Availability::unknown(),
        })
    }

    fn current(&self) -> Result<Option<AccountStatus>> {
        let Some(active) = self.state_store()?.active_account(CLAUDE_STATE_PROVIDER)? else {
            return Ok(None);
        };
        Ok(Some(self.account_status(&active, Some(&active.local_id))))
    }

    fn list_accounts(&self) -> Result<Vec<AccountStatus>> {
        let store = self.state_store()?;
        let active = store.active_account(CLAUDE_STATE_PROVIDER)?;
        let active_id = active.as_ref().map(|account| account.local_id.as_str());
        Ok(store
            .list_accounts(CLAUDE_STATE_PROVIDER)?
            .iter()
            .map(|account| self.account_status(account, active_id))
            .collect())
    }

    fn list_configs(&self) -> Result<Vec<ConfigProfile>> {
        let store = self.state_store()?;
        let active = store.active_profile(CLAUDE_STATE_PROVIDER)?;
        let active_id = active.as_ref().map(|profile| profile.local_id.as_str());
        Ok(store
            .list_profiles(CLAUDE_STATE_PROVIDER)?
            .iter()
            .map(|profile| self.profile_status(profile, active_id))
            .collect())
    }

    fn login(&self, options: LoginOptions) -> Result<AccountRef> {
        self.login_with_official_cli(options)
    }

    fn save_current(&self, _options: SaveOptions) -> Result<AccountRef> {
        self.import_account(None)
    }

    fn import_config(&self, options: ImportConfigOptions) -> Result<ImportedConfig> {
        if !options.content.trim().is_empty() {
            return self.import_profile(options);
        }
        let account = self.import_account(options.name)?;
        Ok(ImportedConfig {
            platform: self.info(),
            profile_name: account.alias.unwrap_or_else(|| account.number.to_string()),
            config_path: self
                .state_store()?
                .account_by_selector(CLAUDE_STATE_PROVIDER, &account.number.to_string())?
                .map(|record| record.secret_ref)
                .unwrap_or_default(),
            provider_id: Some("claude-ai-oauth".to_string()),
            base_url: None,
            model: None,
            number: Some(account.number),
            auth_type: Some("oauth".to_string()),
        })
    }

    fn use_target(&self, selector: &str) -> Result<UseReport> {
        let target = self.resolve_target(selector)?;
        match target.kind {
            TargetKind::Account => self.switch_to(&target.target_id).map(UseReport::Account),
            TargetKind::Profile => self.use_profile(&target.target_id).map(UseReport::Config),
        }
    }

    fn remove_target(&self, selector: &str) -> Result<RemoveReport> {
        let target = self.resolve_target(selector)?;
        match target.kind {
            TargetKind::Account => self
                .remove_account(&target.target_id)
                .map(RemoveReport::Account),
            TargetKind::Profile => self
                .remove_profile(&target.target_id)
                .map(RemoveReport::Config),
        }
    }

    fn switch_to(&self, selector: &str) -> Result<SwitchReport> {
        let store = self.state_store()?;
        let account = self.resolve_account(selector)?;
        let snapshot_path = PathBuf::from(&account.secret_ref);
        let snapshot = read_file(&snapshot_path)?;
        if sha256_hex(&snapshot) != Self::account_snapshot_hash(&account)? {
            return Err(OpenMuxError::Message(format!(
                "stored Claude account #{} failed hash verification",
                account.display_number
            )));
        }
        let oauth_account = read_file(&self.oauth_account_path_for_record(&account)?)?;
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
            prune_backup_files(
                &self.backups_dir()?,
                "credentials.snapshot.bak.",
                BACKUP_RETENTION_PER_KIND,
            )?;
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
            prune_backup_files(
                &self.backups_dir()?,
                "settings.json.bak.",
                BACKUP_RETENTION_PER_KIND,
            )?;
            Some(path)
        } else {
            None
        };

        if let Err(err) = backend.write(&snapshot) {
            return Err(OpenMuxError::Message(format!(
                "failed to write Claude credential: {err}"
            )));
        }
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
            let credential_rollback = backend.restore(current_credentials.as_deref());
            return Err(OpenMuxError::Message(format!(
                "failed to update Claude oauthAccount metadata; credential rollback: {}; error: {err}",
                rollback_status(credential_rollback)
            )));
        }

        let previous = store
            .active_account(CLAUDE_STATE_PROVIDER)?
            .filter(|current| current.local_id != account.local_id)
            .map(|stored| self.account_ref(&stored));
        if let Err(err) =
            store.set_active_account(CLAUDE_STATE_PROVIDER, &account.local_id, unix_now())
        {
            let credential_rollback = backend.restore(current_credentials.as_deref());
            let settings_rollback = rollback_file(&settings_path, current_settings.as_deref());
            return Err(OpenMuxError::Message(format!(
                "failed to update state store after switching Claude account; credential rollback: {}; settings rollback: {}; error: {}; backups: {}, {}",
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
        let settings_path = self.settings_path()?;
        let state_path = self.state_root()?.join("omx-state.sqlite");
        let state_store = self.state_store();
        Ok(DoctorReport {
            platform: self.id().to_string(),
            checks: vec![
                DoctorCheck {
                    name: format!("{}-credentials", backend.label()),
                    ok: backend.exists(),
                    message: display_path(backend.location()),
                },
                DoctorCheck {
                    name: "user-settings".to_string(),
                    ok: settings_path.exists(),
                    message: display_path(&settings_path),
                },
                DoctorCheck {
                    name: "state-store".to_string(),
                    ok: state_store.is_ok(),
                    message: display_path(&state_path),
                },
            ],
        })
    }
}

#[derive(Debug, Clone)]
struct ParsedCredentials {
    expires_at_unix: i64,
    refresh_token_hash: String,
}

#[derive(Debug, Clone)]
struct SafeAccountMetadata {
    email: Option<String>,
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
    Ok(ParsedCredentials {
        expires_at_unix,
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
    let has_account_uuid = oauth_account
        .get("accountUuid")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .is_some();
    let has_organization_uuid = oauth_account
        .get("organizationUuid")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .is_some();
    let partial_metadata = email.is_none() || !has_account_uuid || !has_organization_uuid;
    SafeAccountMetadata {
        email,
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

fn remove_file_if_exists(path: &Path, removed_paths: &mut Vec<String>) -> Result<()> {
    if path.exists() {
        fs::remove_file(path).map_err(|err| io_error(path, err))?;
        removed_paths.push(display_path(path));
    }
    Ok(())
}

fn mask_email(value: &str) -> String {
    let Some((name, domain)) = value.split_once('@') else {
        return "redacted".to_string();
    };
    let first = name.chars().next().unwrap_or('*');
    format!("{first}***@{domain}")
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
    let explicit_profile_name = vars
        .iter()
        .find(|(key, value)| {
            (key == "OMUX_PROFILE" || key == "name" || key == "NAME") && !value.is_empty()
        })
        .map(|(_, value)| value.as_str());
    let env: BTreeMap<String, String> = vars
        .iter()
        .filter(|(key, value)| MANAGED_ENV_KEYS.contains(&key.as_str()) && !value.is_empty())
        .map(|(key, value)| (key.clone(), value.clone()))
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
        explicit_profile_name,
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
