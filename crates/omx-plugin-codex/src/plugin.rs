use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use omx_core::{
    AccountRef, AccountStatus, Availability, AvailabilityState, ConfigProfile, ConfigSwitchReport,
    DoctorCheck, DoctorReport, ImportConfigOptions, ImportedConfig, LoginOptions, OpenMuxError,
    PlatformCapabilities, PlatformInfo, PlatformInstall, PlatformPlugin, PlatformPoolSummary,
    Result, SaveOptions, SwitchReport, UsageDiagnostic, UsageLimit, UsageLimitKind,
    UsageLimitScope, UsageSnapshot, UsageSource, UseReport, platform_info,
    storage::{
        create_dir_private, data_local_dir, display_path, home_dir, io_error, read_file,
        set_private_file_permissions, sha256_hex, unix_now, unix_now_nanos,
        write_file_atomic_private,
    },
};
use registry_io::{encode_registry, parse_registry};

#[path = "registry_io.rs"]
mod registry_io;

const REGISTRY_SCHEMA_VERSION: u32 = 1;
const AUTH_FILE_NAME: &str = "auth.json";

#[derive(Debug, Clone)]
pub struct CodexPlugin {
    codex_home: Option<PathBuf>,
    state_root: Option<PathBuf>,
    codex_executable: PathBuf,
    #[cfg(test)]
    fail_registry_save: bool,
}

#[derive(Debug, Clone)]
struct Registry {
    schema_version: u32,
    active_number: Option<u32>,
    previous_active_number: Option<u32>,
    next_number: u32,
    accounts: Vec<StoredAccount>,
}

#[derive(Debug, Clone)]
struct StoredAccount {
    number: u32,
    alias: Option<String>,
    account_label: Option<String>,
    plan_label: Option<String>,
    auth_hash: String,
    snapshot_path: String,
    imported_at_unix: u64,
    last_activated_at_unix: Option<u64>,
}

struct TempDirCleanup {
    path: PathBuf,
}

impl TempDirCleanup {
    fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl Drop for TempDirCleanup {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

impl Default for CodexPlugin {
    fn default() -> Self {
        Self {
            codex_home: None,
            state_root: None,
            codex_executable: env::var_os("OMUX_CODEX_BIN")
                .filter(|value| !value.is_empty())
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("codex")),
            #[cfg(test)]
            fail_registry_save: false,
        }
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self {
            schema_version: REGISTRY_SCHEMA_VERSION,
            active_number: None,
            previous_active_number: None,
            next_number: 1,
            accounts: Vec::new(),
        }
    }
}

impl CodexPlugin {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_paths(codex_home: impl Into<PathBuf>, state_root: impl Into<PathBuf>) -> Self {
        Self {
            codex_home: Some(codex_home.into()),
            state_root: Some(state_root.into()),
            ..Self::default()
        }
    }

    pub fn with_paths_and_codex_executable(
        codex_home: impl Into<PathBuf>,
        state_root: impl Into<PathBuf>,
        codex_executable: impl Into<PathBuf>,
    ) -> Self {
        Self {
            codex_home: Some(codex_home.into()),
            state_root: Some(state_root.into()),
            codex_executable: codex_executable.into(),
            #[cfg(test)]
            fail_registry_save: false,
        }
    }

    #[cfg(test)]
    fn with_paths_and_registry_save_failure(
        codex_home: impl Into<PathBuf>,
        state_root: impl Into<PathBuf>,
    ) -> Self {
        Self {
            codex_home: Some(codex_home.into()),
            state_root: Some(state_root.into()),
            fail_registry_save: true,
            ..Self::default()
        }
    }

    fn info(&self) -> PlatformInfo {
        platform_info(self.id(), self.name())
    }

    fn codex_home(&self) -> Result<PathBuf> {
        if let Some(path) = &self.codex_home {
            return Ok(path.clone());
        }

        if let Some(path) = env::var_os("CODEX_HOME").filter(|value| !value.is_empty()) {
            return Ok(PathBuf::from(path));
        }

        home_dir()
            .map(|path| path.join(".codex"))
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

    fn accounts_dir(&self) -> Result<PathBuf> {
        Ok(self.platform_state_dir()?.join("accounts"))
    }

    fn backups_dir(&self) -> Result<PathBuf> {
        Ok(self.platform_state_dir()?.join("backups"))
    }

    fn config_snapshots_dir(&self) -> Result<PathBuf> {
        Ok(self.platform_state_dir()?.join("configs"))
    }

    fn login_dir(&self) -> Result<PathBuf> {
        Ok(self.platform_state_dir()?.join("login"))
    }

    fn registry_path(&self) -> Result<PathBuf> {
        Ok(self.platform_state_dir()?.join("registry.omx"))
    }

    fn active_auth_path(&self) -> Result<PathBuf> {
        Ok(self.codex_home()?.join(AUTH_FILE_NAME))
    }

    fn config_path(&self) -> Result<PathBuf> {
        Ok(self.codex_home()?.join("config.toml"))
    }

    fn default_config_snapshot_path(&self) -> Result<PathBuf> {
        Ok(self.config_snapshots_dir()?.join("default.config.toml"))
    }

    fn profile_config_path(&self, profile_name: &str) -> Result<PathBuf> {
        Ok(self
            .codex_home()?
            .join(format!("{profile_name}.config.toml")))
    }

    fn account_snapshot_path(&self, number: u32) -> Result<PathBuf> {
        Ok(self.accounts_dir()?.join(format!("{number}.auth.json")))
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
        registry.accounts.sort_by_key(|account| account.number);
        registry.next_number = registry.next_number.max(
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

    fn save_registry(&self, registry: &Registry) -> Result<()> {
        #[cfg(test)]
        if self.fail_registry_save {
            return Err(OpenMuxError::Message(
                "injected Codex registry save failure".to_string(),
            ));
        }
        let path = self.registry_path()?;
        write_file_atomic_private(&path, encode_registry(registry).as_bytes())
    }

    fn account_ref(&self, account: &StoredAccount) -> AccountRef {
        AccountRef {
            platform: self.id().to_string(),
            number: account.number,
            alias: account.alias.clone(),
        }
    }

    fn account_status(&self, account: &StoredAccount, active_number: Option<u32>) -> AccountStatus {
        let metadata = self.metadata_from_snapshot(account);
        let usage = self.usage_from_snapshot(account);
        let availability = usage.summary.clone();
        AccountStatus {
            active: active_number == Some(account.number),
            account: self.account_ref(account),
            account_label: account.account_label.clone().or(metadata.account_label),
            plan_label: account.plan_label.clone().or(metadata.plan_label),
            auth_type: None,
            expires_at_unix: None,
            availability,
            usage: Some(usage),
        }
    }

    fn metadata_from_snapshot(&self, account: &StoredAccount) -> CodexAccountMetadata {
        read_file(Path::new(&account.snapshot_path))
            .ok()
            .map(|bytes| extract_codex_account_metadata(&bytes))
            .unwrap_or_default()
    }

    fn usage_from_snapshot(&self, account: &StoredAccount) -> UsageSnapshot {
        let Some(auth) = read_file(Path::new(&account.snapshot_path))
            .ok()
            .and_then(|bytes| parse_codex_usage_auth(&bytes))
        else {
            return UsageSnapshot::unknown(
                UsageSource::RemoteApi,
                UsageDiagnostic {
                    code: "auth".to_string(),
                    message: "stored auth snapshot is missing ChatGPT access token or account id"
                        .to_string(),
                },
            );
        };

        let payload = match self.fetch_codex_usage(&auth) {
            Ok(payload) => payload,
            Err(diagnostic) => {
                return UsageSnapshot::unknown(UsageSource::RemoteApi, diagnostic);
            }
        };

        parse_codex_usage_snapshot(&payload, unix_now() as i64).unwrap_or_else(|| {
            UsageSnapshot::unknown(
                UsageSource::RemoteApi,
                UsageDiagnostic {
                    code: "schema".to_string(),
                    message: "Codex usage response did not include known quota fields".to_string(),
                },
            )
        })
    }

    fn fetch_codex_usage(
        &self,
        auth: &CodexUsageAuth,
    ) -> std::result::Result<serde_json::Value, UsageDiagnostic> {
        let config_path = self
            .platform_state_dir()
            .map_err(usage_diagnostic_from_error)?
            .join(format!(
                ".usage-curl-{}-{}.conf",
                std::process::id(),
                unix_now_nanos()
            ));
        let config = format!(
            "header = \"Authorization: Bearer {}\"\nheader = \"ChatGPT-Account-Id: {}\"\nheader = \"User-Agent: codex-cli\"\nmax-time = 4\nsilent\nshow-error\n",
            escape_curl_config(&auth.access_token),
            escape_curl_config(&auth.account_id)
        );
        write_file_atomic_private(&config_path, config.as_bytes())
            .map_err(usage_diagnostic_from_error)?;

        let output = Command::new("curl")
            .arg("--config")
            .arg(&config_path)
            .arg("--write-out")
            .arg("\n%{http_code}")
            .arg("https://chatgpt.com/backend-api/wham/usage")
            .output()
            .map_err(|err| {
                let _ = fs::remove_file(&config_path);
                UsageDiagnostic {
                    code: "network".to_string(),
                    message: format!("failed to run curl for Codex usage: {err}"),
                }
            })?;
        let _ = fs::remove_file(&config_path);

        if !output.status.success() {
            return Err(usage_diagnostic_from_curl_status(output.status.code()));
        }

        let (status, body) =
            parse_curl_http_output(&output.stdout).ok_or_else(|| UsageDiagnostic {
                code: "network".to_string(),
                message: "Codex usage request did not include an HTTP status".to_string(),
            })?;
        if !(200..=299).contains(&status) {
            return Err(UsageDiagnostic {
                code: status.to_string(),
                message: format!("Codex usage request returned HTTP {status}"),
            });
        }

        serde_json::from_slice(body).map_err(|err| UsageDiagnostic {
            code: "json".to_string(),
            message: format!("invalid Codex usage response: {err}"),
        })
    }

    fn detect_install(&self) -> Result<PlatformInstall> {
        let config_path = self.config_path()?;
        let auth_path = self.active_auth_path()?;

        Ok(PlatformInstall {
            platform: self.info(),
            config_path: config_path.exists().then(|| display_path(&config_path)),
            auth_path: auth_path.exists().then(|| display_path(&auth_path)),
        })
    }

    fn resolve_account<'a>(
        &self,
        registry: &'a Registry,
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
            .find(|account| account.alias.as_deref() == Some(selector))
            .ok_or_else(|| OpenMuxError::AccountNotFound {
                platform: self.id().to_string(),
                account: selector.to_string(),
            })
    }

    fn import_auth_bytes(
        &self,
        auth_bytes: &[u8],
        alias: Option<String>,
        mark_active: bool,
    ) -> Result<AccountRef> {
        if auth_bytes.is_empty() {
            return Err(OpenMuxError::Message(
                "refusing to import empty auth payload".into(),
            ));
        }
        validate_alias_option(alias.as_deref())?;

        let auth_hash = sha256_hex(auth_bytes);
        let metadata = extract_codex_account_metadata(auth_bytes);
        let mut registry = self.load_registry()?;
        let now = unix_now();
        let account_ref;

        let existing_index = registry
            .accounts
            .iter()
            .position(|account| account.auth_hash == auth_hash);
        let existing_number = existing_index.map(|index| registry.accounts[index].number);
        ensure_alias_available(&registry, alias.as_deref(), existing_number)?;

        if let Some(index) = existing_index {
            let number = registry.accounts[index].number;
            let snapshot_path = self.account_snapshot_path(number)?;
            write_file_atomic_private(&snapshot_path, auth_bytes)?;

            {
                let account = &mut registry.accounts[index];
                if alias.is_some() {
                    account.alias = alias;
                }
                account.account_label = metadata
                    .account_label
                    .or_else(|| account.account_label.clone());
                account.plan_label = metadata.plan_label.or_else(|| account.plan_label.clone());
                account.snapshot_path = display_path(&snapshot_path);
                account.imported_at_unix = now;
            }
            account_ref = self.account_ref(&registry.accounts[index]);
        } else {
            let number = registry.next_number;
            registry.next_number += 1;
            let snapshot_path = self.account_snapshot_path(number)?;
            write_file_atomic_private(&snapshot_path, auth_bytes)?;
            registry.accounts.push(StoredAccount {
                number,
                alias,
                account_label: metadata.account_label,
                plan_label: metadata.plan_label,
                auth_hash,
                snapshot_path: display_path(&snapshot_path),
                imported_at_unix: now,
                last_activated_at_unix: None,
            });
            registry.accounts.sort_by_key(|account| account.number);
            let account = registry
                .accounts
                .iter()
                .find(|account| account.number == number)
                .expect("new account should exist");
            account_ref = self.account_ref(account);
        }

        if mark_active {
            if registry.active_number != Some(account_ref.number) {
                registry.previous_active_number = registry.active_number;
            }
            registry.active_number = Some(account_ref.number);
            if let Some(account) = registry
                .accounts
                .iter_mut()
                .find(|account| account.number == account_ref.number)
            {
                account.last_activated_at_unix = Some(now);
            }
        }

        self.save_registry(&registry)?;
        Ok(account_ref)
    }

    fn build_imported_config(&self, options: ImportConfigOptions) -> Result<ImportedConfig> {
        let raw_content = options.content.trim();
        if raw_content.is_empty() {
            return Err(OpenMuxError::Message(
                "missing Codex config content to import".into(),
            ));
        }

        let imported = parse_codex_import_config(raw_content, options.name.as_deref())?;
        let profile_path = self.profile_config_path(&imported.profile_name)?;
        write_file_atomic_private(&profile_path, imported.config_toml.as_bytes())?;

        Ok(ImportedConfig {
            platform: self.info(),
            profile_name: imported.profile_name,
            config_path: display_path(&profile_path),
            provider_id: imported.provider_id,
            base_url: imported.base_url,
            model: imported.model,
            number: None,
            auth_type: imported.auth_type,
        })
    }

    fn list_codex_profiles(&self) -> Result<Vec<ConfigProfile>> {
        let codex_home = self.codex_home()?;
        if !codex_home.exists() {
            return Ok(Vec::new());
        }
        let account_active = self.load_registry()?.active_number.is_some();
        let active_config = self
            .config_path()
            .ok()
            .and_then(|path| read_file(&path).ok());

        let mut profiles = Vec::new();
        for entry in fs::read_dir(&codex_home).map_err(|err| io_error(&codex_home, err))? {
            let entry = entry.map_err(|err| io_error(&codex_home, err))?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
                continue;
            };
            let Some(profile_name) = file_name.strip_suffix(".config.toml") else {
                continue;
            };
            if profile_name.is_empty() || profile_name == "config" {
                continue;
            }

            let text = String::from_utf8(read_file(&path)?)
                .map_err(|err| OpenMuxError::Message(format!("{}: {err}", display_path(&path))))?;
            let profile_bytes = text.as_bytes();
            let parsed = parse_codex_profile_file(profile_name, &text);
            profiles.push(ConfigProfile {
                platform: self.info(),
                name: profile_name.to_string(),
                active: !account_active
                    && active_config
                        .as_ref()
                        .is_some_and(|active| active.as_slice() == profile_bytes),
                config_path: display_path(&path),
                provider_id: parsed.provider_id,
                base_url: parsed.base_url,
                model: parsed.model,
                number: None,
                auth_type: parsed.auth_type,
            });
        }

        profiles.sort_by(|left, right| left.name.cmp(&right.name));
        Ok(profiles)
    }

    fn config_profile_by_name(&self, selector: &str) -> Result<Option<ConfigProfile>> {
        if selector == "default" {
            let path = self.default_config_snapshot_path()?;
            if path.exists() {
                let text = String::from_utf8(read_file(&path)?).map_err(|err| {
                    OpenMuxError::Message(format!("{}: {err}", display_path(&path)))
                })?;
                let parsed = parse_codex_profile_file("default", &text);
                let active = self.load_registry()?.active_number.is_none()
                    && self
                        .config_path()
                        .ok()
                        .and_then(|path| read_file(&path).ok())
                        .is_some_and(|active| active == text.as_bytes());
                return Ok(Some(ConfigProfile {
                    platform: self.info(),
                    name: "default".to_string(),
                    active,
                    config_path: display_path(&path),
                    provider_id: parsed.provider_id,
                    base_url: parsed.base_url,
                    model: parsed.model,
                    number: None,
                    auth_type: parsed.auth_type,
                }));
            }
        }

        Ok(self
            .list_codex_profiles()?
            .into_iter()
            .find(|profile| profile.name == selector))
    }

    fn switch_to_config_profile(&self, selector: &str) -> Result<ConfigSwitchReport> {
        let profile = self.config_profile_by_name(selector)?.ok_or_else(|| {
            OpenMuxError::AccountNotFound {
                platform: self.id().to_string(),
                account: selector.to_string(),
            }
        })?;
        let source_path = PathBuf::from(&profile.config_path);
        if !source_path.exists() {
            return Err(OpenMuxError::Message(format!(
                "stored config profile `{}` is missing at {}",
                profile.name,
                display_path(&source_path)
            )));
        }

        let config_path = self.config_path()?;
        let next_bytes = read_file(&source_path)?;
        let current_bytes = config_path
            .exists()
            .then(|| read_file(&config_path))
            .transpose()?;
        let mut backup_path = None;
        if let Some(current_bytes) = current_bytes.as_ref() {
            let default_snapshot_path = self.default_config_snapshot_path()?;
            if !default_snapshot_path.exists() {
                write_file_atomic_private(&default_snapshot_path, current_bytes)?;
            }
            if current_bytes != &next_bytes {
                let path = self
                    .backups_dir()?
                    .join(format!("config.toml.bak.{}", unix_now_nanos()));
                if let Some(parent) = path.parent() {
                    create_dir_private(parent)?;
                }
                fs::copy(&config_path, &path).map_err(|err| io_error(&path, err))?;
                set_private_file_permissions(&path)?;
                backup_path = Some(display_path(&path));
            }
        }

        write_file_atomic_private(&config_path, &next_bytes)?;
        let mut registry = self.load_registry()?;
        let previous_active_number = registry.active_number;
        registry.previous_active_number = previous_active_number;
        registry.active_number = None;
        if let Err(err) = self.save_registry(&registry) {
            let rollback = match current_bytes {
                Some(bytes) => write_file_atomic_private(&config_path, &bytes),
                None => fs::remove_file(&config_path)
                    .map_err(|remove_err| io_error(&config_path, remove_err)),
            };
            return match rollback {
                Ok(()) => Err(OpenMuxError::Message(format!(
                    "failed to update registry after switching profile; config was rolled back: {err}"
                ))),
                Err(rollback_err) => Err(OpenMuxError::Message(format!(
                    "failed to update registry after switching profile and rollback failed: {err}; rollback error: {rollback_err}; backup: {}",
                    backup_path.as_deref().unwrap_or("none")
                ))),
            };
        }
        let mut active_profile = profile;
        active_profile.active = true;
        Ok(ConfigSwitchReport {
            platform: self.info(),
            profile: active_profile,
            config_path: display_path(&config_path),
            backup_path,
        })
    }
}

impl PlatformPlugin for CodexPlugin {
    fn id(&self) -> &'static str {
        "codex"
    }

    fn name(&self) -> &'static str {
        "Codex"
    }

    fn capabilities(&self) -> PlatformCapabilities {
        PlatformCapabilities {
            profiles: true,
            profile_import: true,
            ..PlatformCapabilities::account_pool()
        }
    }

    fn detect(&self) -> Result<PlatformInstall> {
        self.detect_install()
    }

    fn pool_summary(&self) -> Result<PlatformPoolSummary> {
        let registry = self.load_registry()?;
        let active = registry
            .active_number
            .and_then(|number| {
                registry
                    .accounts
                    .iter()
                    .find(|account| account.number == number)
            })
            .map(|account| self.account_ref(account));
        let availability = summarize_usage_availability(
            registry
                .accounts
                .iter()
                .map(|account| self.usage_from_snapshot(account))
                .collect(),
        );

        Ok(PlatformPoolSummary {
            platform: self.info(),
            account_count: registry.accounts.len(),
            active,
            profile_count: self
                .list_codex_profiles()
                .map(|profiles| profiles.len())
                .unwrap_or(0),
            active_profile: self
                .list_codex_profiles()
                .ok()
                .and_then(|profiles| profiles.into_iter().find(|profile| profile.active))
                .map(|profile| profile.name),
            availability,
        })
    }

    fn current(&self) -> Result<Option<AccountStatus>> {
        let registry = self.load_registry()?;
        let Some(active_number) = registry.active_number else {
            return Ok(None);
        };

        Ok(registry
            .accounts
            .iter()
            .find(|account| account.number == active_number)
            .map(|account| self.account_status(account, Some(active_number))))
    }

    fn list_accounts(&self) -> Result<Vec<AccountStatus>> {
        let registry = self.load_registry()?;
        Ok(registry
            .accounts
            .iter()
            .map(|account| self.account_status(account, registry.active_number))
            .collect())
    }

    fn list_configs(&self) -> Result<Vec<ConfigProfile>> {
        self.list_codex_profiles()
    }

    fn login(&self, options: LoginOptions) -> Result<AccountRef> {
        validate_alias_option(options.alias.as_deref())?;

        let login_home = self.login_dir()?.join(format!(
            "codex-login-{}-{}",
            std::process::id(),
            unix_now_nanos()
        ));
        create_dir_private(&login_home)?;
        let _cleanup = TempDirCleanup::new(login_home.clone());

        let mut command = Command::new(&self.codex_executable);
        command
            .arg("login")
            .env("CODEX_HOME", &login_home)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());
        if options.device_auth {
            command.arg("--device-auth");
        }

        let status = command.status().map_err(|err| {
            OpenMuxError::Message(format!(
                "failed to run {} login: {err}",
                display_path(&self.codex_executable)
            ))
        });
        if let Err(err) = status {
            let _ = fs::remove_dir_all(&login_home);
            return Err(err);
        }
        if !status.expect("status checked").success() {
            let _ = fs::remove_dir_all(&login_home);
            return Err(OpenMuxError::Message(
                "codex login did not complete successfully".into(),
            ));
        }

        let auth_path = login_home.join(AUTH_FILE_NAME);
        let auth_bytes = read_file(&auth_path)?;
        let account = self.import_auth_bytes(&auth_bytes, options.alias, false)?;
        let account = if options.activate {
            self.switch_to(&account.number.to_string())?.current
        } else {
            account
        };
        Ok(account)
    }

    fn save_current(&self, options: SaveOptions) -> Result<AccountRef> {
        validate_alias_option(options.alias.as_deref())?;

        let auth_path = self.active_auth_path()?;
        if !auth_path.exists() {
            return Err(OpenMuxError::PlatformNotDetected(format!(
                "{} auth file at {}",
                self.name(),
                display_path(&auth_path)
            )));
        }

        let auth_bytes = read_file(&auth_path)?;
        self.import_auth_bytes(&auth_bytes, options.alias, true)
    }

    fn import_config(&self, options: ImportConfigOptions) -> Result<ImportedConfig> {
        self.build_imported_config(options)
    }

    fn use_target(&self, selector: &str) -> Result<UseReport> {
        let registry = self.load_registry()?;
        let account_match = self.resolve_account(&registry, selector).ok().cloned();
        let profile_match = self.config_profile_by_name(selector)?;

        match (account_match, profile_match) {
            (Some(account), Some(profile)) => Err(OpenMuxError::Message(format!(
                "`{selector}` matches both account #{} and profile `{}`; use a unique alias or profile name",
                account.number, profile.name
            ))),
            (Some(_), None) => self.switch_to(selector).map(UseReport::Account),
            (None, Some(_)) => self
                .switch_to_config_profile(selector)
                .map(UseReport::Config),
            (None, None) => Err(OpenMuxError::AccountNotFound {
                platform: self.id().to_string(),
                account: selector.to_string(),
            }),
        }
    }

    fn switch_to(&self, selector: &str) -> Result<SwitchReport> {
        let mut registry = self.load_registry()?;
        let account = self.resolve_account(&registry, selector)?.clone();
        let snapshot_path = PathBuf::from(&account.snapshot_path);
        if !snapshot_path.exists() {
            return Err(OpenMuxError::Message(format!(
                "stored auth snapshot for account #{} is missing at {}",
                account.number,
                display_path(&snapshot_path)
            )));
        }

        let auth_path = self.active_auth_path()?;
        let next_bytes = read_file(&snapshot_path)?;
        let next_hash = sha256_hex(&next_bytes);
        if next_hash != account.auth_hash {
            return Err(OpenMuxError::Message(format!(
                "stored auth snapshot for account #{} failed hash verification",
                account.number
            )));
        }

        let current_bytes = if auth_path.exists() {
            Some(read_file(&auth_path)?)
        } else {
            None
        };
        let changed = current_bytes
            .as_ref()
            .is_some_and(|current| current != &next_bytes);
        let config_path = self.config_path()?;
        let current_config_bytes = config_path
            .exists()
            .then(|| read_file(&config_path))
            .transpose()?;
        let default_config_path = self.default_config_snapshot_path()?;
        let default_config_bytes = default_config_path
            .exists()
            .then(|| read_file(&default_config_path))
            .transpose()?;
        let backup_path = if changed {
            let backup_path = self
                .backups_dir()?
                .join(format!("auth.json.bak.{}", unix_now_nanos()));
            if let Some(parent) = backup_path.parent() {
                create_dir_private(parent)?;
            }
            fs::copy(&auth_path, &backup_path).map_err(|err| io_error(&backup_path, err))?;
            set_private_file_permissions(&backup_path)?;
            Some(backup_path)
        } else {
            None
        };

        write_file_atomic_private(&auth_path, &next_bytes)?;
        match default_config_bytes.as_ref() {
            Some(default_bytes) if current_config_bytes.as_ref() != Some(default_bytes) => {
                if let Err(err) = write_file_atomic_private(&config_path, default_bytes) {
                    let rollback = match current_bytes.as_ref() {
                        Some(bytes) => write_file_atomic_private(&auth_path, bytes),
                        None => fs::remove_file(&auth_path)
                            .map_err(|remove_err| io_error(&auth_path, remove_err)),
                    };
                    return match rollback {
                        Ok(()) => Err(OpenMuxError::Message(format!(
                            "failed to restore default Codex config after switching account; active auth was rolled back: {err}"
                        ))),
                        Err(rollback_err) => Err(OpenMuxError::Message(format!(
                            "failed to restore default Codex config after switching account and auth rollback failed: {err}; rollback error: {rollback_err}; backup: {}",
                            backup_path
                                .as_deref()
                                .map(display_path)
                                .unwrap_or_else(|| "none".to_string())
                        ))),
                    };
                }
            }
            _ => {}
        }

        let previous = registry
            .active_number
            .filter(|current| *current != account.number)
            .and_then(|current| {
                registry
                    .accounts
                    .iter()
                    .find(|stored| stored.number == current)
                    .map(|stored| self.account_ref(stored))
            });
        registry.previous_active_number = previous.as_ref().map(|account| account.number);
        registry.active_number = Some(account.number);
        if let Some(stored) = registry
            .accounts
            .iter_mut()
            .find(|stored| stored.number == account.number)
        {
            stored.last_activated_at_unix = Some(unix_now());
        }
        if let Err(err) = self.save_registry(&registry) {
            let auth_rollback = match current_bytes {
                Some(bytes) => write_file_atomic_private(&auth_path, &bytes),
                None => fs::remove_file(&auth_path)
                    .map_err(|remove_err| io_error(&auth_path, remove_err)),
            };
            let config_rollback = match current_config_bytes {
                Some(bytes) => write_file_atomic_private(&config_path, &bytes),
                None if default_config_bytes.is_some() => fs::remove_file(&config_path)
                    .map_err(|remove_err| io_error(&config_path, remove_err)),
                None => Ok(()),
            };
            if auth_rollback.is_ok() && config_rollback.is_ok() {
                return Err(OpenMuxError::Message(format!(
                    "failed to update registry after switching auth; active auth and config were rolled back: {err}"
                )));
            }
            return Err(OpenMuxError::Message(format!(
                "failed to update registry after switching auth and rollback was incomplete: {err}; auth rollback: {}; config rollback: {}; backup: {}",
                rollback_status(auth_rollback),
                rollback_status(config_rollback),
                backup_path
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

    fn set_alias(&self, selector: &str, alias: &str) -> Result<AccountRef> {
        validate_alias(alias)?;

        let mut registry = self.load_registry()?;
        let number = self.resolve_account(&registry, selector)?.number;
        ensure_alias_available(&registry, Some(alias), Some(number))?;

        let account_ref = {
            let account = registry
                .accounts
                .iter_mut()
                .find(|account| account.number == number)
                .expect("resolved account should exist");
            account.alias = Some(alias.to_string());
            self.account_ref(account)
        };
        self.save_registry(&registry)?;
        Ok(account_ref)
    }

    fn doctor(&self) -> Result<DoctorReport> {
        let codex_home = self.codex_home()?;
        let state_dir = self.platform_state_dir()?;
        let auth_path = self.active_auth_path()?;
        let registry_path = self.registry_path()?;
        let registry = self.load_registry();

        Ok(DoctorReport {
            platform: self.id().to_string(),
            checks: vec![
                DoctorCheck {
                    name: "codex-home".to_string(),
                    ok: codex_home.exists(),
                    message: display_path(&codex_home),
                },
                DoctorCheck {
                    name: "active-auth".to_string(),
                    ok: auth_path.exists(),
                    message: display_path(&auth_path),
                },
                DoctorCheck {
                    name: "state-dir".to_string(),
                    ok: state_dir.parent().is_some_and(Path::exists) || state_dir.exists(),
                    message: display_path(&state_dir),
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

fn validate_alias_option(alias: Option<&str>) -> Result<()> {
    if let Some(alias) = alias {
        validate_alias(alias)?;
    }
    Ok(())
}

fn validate_alias(alias: &str) -> Result<()> {
    if alias.is_empty() {
        return Err(OpenMuxError::Message(
            "account alias cannot be empty".into(),
        ));
    }

    if alias.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(OpenMuxError::Message(
            "account alias cannot be all digits because numbers select accounts".into(),
        ));
    }

    let valid = alias
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'));
    if !valid {
        return Err(OpenMuxError::Message(
            "account alias may only contain letters, numbers, dash, underscore, or dot".into(),
        ));
    }

    Ok(())
}

fn ensure_alias_available(
    registry: &Registry,
    alias: Option<&str>,
    allowed_number: Option<u32>,
) -> Result<()> {
    let Some(alias) = alias else {
        return Ok(());
    };

    if let Some(existing) = registry.accounts.iter().find(|account| {
        account.alias.as_deref() == Some(alias) && Some(account.number) != allowed_number
    }) {
        return Err(OpenMuxError::Message(format!(
            "alias `{alias}` is already used by account #{}",
            existing.number
        )));
    }

    Ok(())
}

fn rollback_status(result: Result<()>) -> &'static str {
    match result {
        Ok(()) => "ok",
        Err(_) => "failed",
    }
}

#[derive(Debug, Clone)]
struct ParsedCodexImportConfig {
    profile_name: String,
    provider_id: Option<String>,
    base_url: Option<String>,
    model: Option<String>,
    auth_type: Option<String>,
    config_toml: String,
}

#[derive(Debug, Clone)]
struct CodexProfileMetadata {
    provider_id: Option<String>,
    base_url: Option<String>,
    model: Option<String>,
    auth_type: Option<String>,
}

fn parse_codex_import_config(
    content: &str,
    requested_name: Option<&str>,
) -> Result<ParsedCodexImportConfig> {
    if looks_like_toml_config(content) {
        parse_codex_toml_import(content, requested_name)
    } else {
        parse_codex_kv_import(content, requested_name)
    }
}

fn looks_like_toml_config(content: &str) -> bool {
    content
        .lines()
        .map(str::trim)
        .any(|line| line.starts_with('[') || line.contains(" = "))
}

fn parse_codex_toml_import(
    content: &str,
    requested_name: Option<&str>,
) -> Result<ParsedCodexImportConfig> {
    let value: toml::Value = toml::from_str(content).map_err(|err| {
        OpenMuxError::Message(format!("invalid Codex TOML config fragment: {err}"))
    })?;

    let metadata = codex_profile_metadata(&value);
    let provider_id = metadata.provider_id;
    let base_url = metadata.base_url;
    let model = metadata.model;

    if provider_id.is_none() && model.is_none() && base_url.is_none() {
        return Err(OpenMuxError::Message(
            "Codex TOML import must include model_provider, model, openai_base_url, or [model_providers.<id>]".into(),
        ));
    }

    let profile_name = resolve_profile_name(
        requested_name,
        base_url.as_deref(),
        provider_id.as_deref(),
        "codex-import",
    )?;
    let mut config_toml = content.trim().to_string();
    config_toml.push('\n');

    Ok(ParsedCodexImportConfig {
        profile_name,
        provider_id,
        base_url,
        model,
        auth_type: codex_profile_auth_type(&value),
        config_toml,
    })
}

fn parse_codex_kv_import(
    content: &str,
    requested_name: Option<&str>,
) -> Result<ParsedCodexImportConfig> {
    let vars = parse_shell_like_kv(content)?;
    let base_url = find_var(&vars, &["OPENAI_BASE_URL", "OPENAI_API_BASE", "BASE_URL"])
        .ok_or_else(|| {
            OpenMuxError::Message("Codex KV import needs OPENAI_BASE_URL or OPENAI_API_BASE".into())
        })?;
    let model = find_var(&vars, &["OPENAI_MODEL", "MODEL"]).unwrap_or("gpt-5");
    let key_var = first_present_key(&vars, &["OPENAI_API_KEY", "API_KEY"]);
    let provider_id = resolve_profile_name(requested_name, Some(base_url), None, "codex")?;
    let profile_name = provider_id.clone();

    let mut config = String::new();
    config.push_str(&format!(
        "model_provider = \"{}\"\n",
        escape_toml_string(&provider_id)
    ));
    config.push_str(&format!("model = \"{}\"\n\n", escape_toml_string(model)));
    config.push_str(&format!("[model_providers.{}]\n", provider_id));
    config.push_str(&format!(
        "name = \"{}\"\n",
        escape_toml_string(&provider_id)
    ));
    config.push_str(&format!(
        "base_url = \"{}\"\n",
        escape_toml_string(base_url)
    ));
    config.push_str("wire_api = \"responses\"\n");
    match key_var {
        Some(key_var) => {
            config.push_str(&format!("env_key = \"{}\"\n", escape_toml_string(key_var)));
        }
        None => config.push_str("requires_openai_auth = true\n"),
    }

    Ok(ParsedCodexImportConfig {
        profile_name,
        provider_id: Some(provider_id),
        base_url: Some(base_url.to_string()),
        model: Some(model.to_string()),
        auth_type: Some(
            key_var
                .map(|_| "api-key")
                .unwrap_or("codex-auth")
                .to_string(),
        ),
        config_toml: config,
    })
}

fn parse_codex_profile_file(_profile_name: &str, content: &str) -> CodexProfileMetadata {
    toml::from_str(content)
        .map(|value| codex_profile_metadata(&value))
        .unwrap_or(CodexProfileMetadata {
            provider_id: None,
            base_url: None,
            model: None,
            auth_type: None,
        })
}

fn codex_profile_metadata(value: &toml::Value) -> CodexProfileMetadata {
    let provider_id = value
        .get("model_provider")
        .and_then(toml::Value::as_str)
        .map(str::to_string)
        .or_else(|| first_model_provider_id(value));
    let model = value
        .get("model")
        .and_then(toml::Value::as_str)
        .map(str::to_string);
    let base_url = provider_id
        .as_deref()
        .and_then(|provider_id| provider_base_url(value, provider_id))
        .or_else(|| {
            value
                .get("openai_base_url")
                .and_then(toml::Value::as_str)
                .map(str::to_string)
        });

    CodexProfileMetadata {
        provider_id,
        base_url,
        model,
        auth_type: codex_profile_auth_type(value),
    }
}

fn codex_profile_auth_type(value: &toml::Value) -> Option<String> {
    let provider_id = value
        .get("model_provider")
        .and_then(toml::Value::as_str)
        .map(str::to_string)
        .or_else(|| first_model_provider_id(value))?;
    let provider = value.get("model_providers")?.get(provider_id)?;
    if provider.get("env_key").is_some() {
        Some("api-key".to_string())
    } else if provider
        .get("requires_openai_auth")
        .and_then(toml::Value::as_bool)
        .unwrap_or(false)
    {
        Some("codex-auth".to_string())
    } else {
        None
    }
}

fn first_model_provider_id(value: &toml::Value) -> Option<String> {
    value
        .get("model_providers")
        .and_then(toml::Value::as_table)
        .and_then(|table| table.keys().next())
        .cloned()
}

fn provider_base_url(value: &toml::Value, provider_id: &str) -> Option<String> {
    value
        .get("model_providers")?
        .get(provider_id)?
        .get("base_url")?
        .as_str()
        .map(str::to_string)
}

fn resolve_profile_name(
    requested_name: Option<&str>,
    base_url: Option<&str>,
    provider_id: Option<&str>,
    fallback: &str,
) -> Result<String> {
    let raw = requested_name
        .or_else(|| base_url.and_then(host_from_url))
        .or(provider_id)
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

fn find_var<'a>(vars: &'a [(String, String)], keys: &[&str]) -> Option<&'a str> {
    keys.iter().find_map(|key| {
        vars.iter()
            .find(|(candidate, _)| candidate == key)
            .map(|(_, value)| value.as_str())
    })
}

fn first_present_key<'a>(vars: &[(String, String)], keys: &'a [&'a str]) -> Option<&'a str> {
    keys.iter().find_map(|key| {
        vars.iter()
            .any(|(candidate, value)| candidate == key && !value.is_empty())
            .then_some(*key)
    })
}

fn escape_toml_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct CodexAccountMetadata {
    account_label: Option<String>,
    plan_label: Option<String>,
}

fn extract_codex_account_metadata(auth_bytes: &[u8]) -> CodexAccountMetadata {
    let Ok(auth) = serde_json::from_slice::<serde_json::Value>(auth_bytes) else {
        return CodexAccountMetadata::default();
    };
    let mut metadata = auth
        .pointer("/tokens/id_token")
        .and_then(serde_json::Value::as_str)
        .and_then(metadata_from_jwt)
        .unwrap_or_default();

    if metadata.account_label.is_none() {
        metadata.account_label = auth
            .pointer("/tokens/account_id")
            .and_then(serde_json::Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .map(ToString::to_string);
    }

    metadata
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CodexUsageAuth {
    access_token: String,
    account_id: String,
}

fn parse_codex_usage_auth(auth_bytes: &[u8]) -> Option<CodexUsageAuth> {
    let auth: serde_json::Value = serde_json::from_slice(auth_bytes).ok()?;
    Some(CodexUsageAuth {
        access_token: auth
            .pointer("/tokens/access_token")
            .and_then(serde_json::Value::as_str)
            .filter(|value| !value.trim().is_empty())?
            .to_string(),
        account_id: auth
            .pointer("/tokens/account_id")
            .and_then(serde_json::Value::as_str)
            .filter(|value| !value.trim().is_empty())?
            .to_string(),
    })
}

fn parse_curl_http_output(output: &[u8]) -> Option<(u16, &[u8])> {
    let status_separator = output.iter().rposition(|byte| *byte == b'\n')?;
    let body = &output[..status_separator];
    let status = std::str::from_utf8(&output[status_separator + 1..])
        .ok()?
        .trim()
        .parse()
        .ok()?;
    Some((status, body))
}

fn usage_diagnostic_from_curl_status(exit_code: Option<i32>) -> UsageDiagnostic {
    let code = match exit_code {
        Some(28) => "timeout",
        Some(5 | 6 | 7 | 35 | 52 | 56) => "network",
        _ => "curl",
    };
    UsageDiagnostic {
        code: code.to_string(),
        message: match exit_code {
            Some(value) => format!("curl exited with status {value}"),
            None => "curl was terminated before completing".to_string(),
        },
    }
}

fn usage_diagnostic_from_error(error: OpenMuxError) -> UsageDiagnostic {
    UsageDiagnostic {
        code: "state".to_string(),
        message: error.to_string(),
    }
}

fn parse_codex_usage_snapshot(
    payload: &serde_json::Value,
    refreshed_at_unix: i64,
) -> Option<UsageSnapshot> {
    let mut limits = Vec::new();
    if let Some(limit) = parse_codex_usage_window(
        payload.pointer("/rate_limit/primary_window"),
        "codex-primary",
        "primary",
        Some("primary_window"),
    ) {
        limits.push(limit);
    }
    if let Some(limit) = parse_codex_usage_window(
        payload.pointer("/rate_limit/secondary_window"),
        "codex-secondary",
        "secondary",
        Some("secondary_window"),
    ) {
        limits.push(limit);
    }
    if let Some(additional) = payload
        .pointer("/additional_rate_limits")
        .and_then(serde_json::Value::as_array)
    {
        for (index, item) in additional.iter().enumerate() {
            let id = item
                .pointer("/limit_id")
                .and_then(serde_json::Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .map(ToString::to_string)
                .unwrap_or_else(|| format!("codex-additional-{index}"));
            let label = item
                .pointer("/name")
                .and_then(serde_json::Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .unwrap_or(&id);
            if let Some(limit) =
                parse_codex_usage_window(Some(item), &id, label, Some("additional_rate_limits"))
            {
                limits.push(limit);
            }
        }
    }

    if limits.is_empty() {
        return None;
    }

    Some(UsageSnapshot {
        source: UsageSource::RemoteApi,
        refreshed_at_unix: Some(refreshed_at_unix),
        summary: summarize_limits(&limits),
        limits,
        diagnostics: Vec::new(),
    })
}

fn parse_codex_usage_window(
    value: Option<&serde_json::Value>,
    id: &str,
    fallback_label: &str,
    raw_provider_key: Option<&str>,
) -> Option<UsageLimit> {
    let value = value?;
    let used_percent_x100 = value
        .pointer("/used_percent")
        .and_then(json_number_as_percent_x100)?;
    let remaining_percent_x100 = 10_000_u32.saturating_sub(used_percent_x100.min(10_000));
    let window_seconds = value
        .pointer("/limit_window_seconds")
        .and_then(json_number_as_u64)
        .or_else(|| {
            value
                .pointer("/window_minutes")
                .and_then(json_number_as_u64)
                .and_then(|minutes| minutes.checked_mul(60))
        });

    Some(UsageLimit {
        id: id.to_string(),
        label: usage_window_label(window_seconds, fallback_label),
        scope: UsageLimitScope::Account,
        kind: UsageLimitKind::RollingWindow,
        window_seconds,
        used_percent_x100: Some(used_percent_x100),
        remaining_percent_x100: Some(remaining_percent_x100),
        reset_at_unix: value.pointer("/reset_at").and_then(json_number_as_i64),
        exhausted: Some(remaining_percent_x100 == 0),
        raw_provider_key: raw_provider_key.map(ToString::to_string),
    })
}

fn summarize_usage_availability(values: Vec<UsageSnapshot>) -> Availability {
    let summaries: Vec<Availability> = values.into_iter().map(|usage| usage.summary).collect();
    summarize_availability(summaries)
}

fn summarize_availability(values: Vec<Availability>) -> Availability {
    let percentages: Vec<u32> = values
        .iter()
        .filter_map(|value| parse_display_percent_x100(&value.display))
        .collect();

    if percentages.is_empty() {
        return Availability::unknown();
    }

    let minimum = percentages.into_iter().min().unwrap_or(0);
    Availability {
        state: availability_state_from_percent_x100(minimum),
        display: format_percent_x100(minimum),
    }
}

fn summarize_limits(limits: &[UsageLimit]) -> Availability {
    let remaining: Vec<u32> = limits
        .iter()
        .filter_map(|limit| limit.remaining_percent_x100)
        .collect();
    if let Some(minimum) = remaining.into_iter().min() {
        return Availability {
            state: availability_state_from_percent_x100(minimum),
            display: format_percent_x100(minimum),
        };
    }
    Availability::unknown()
}

fn availability_state_from_percent_x100(percent_x100: u32) -> AvailabilityState {
    if percent_x100 == 0 {
        AvailabilityState::Exhausted
    } else if percent_x100 <= 2_000 {
        AvailabilityState::Limited
    } else {
        AvailabilityState::Available
    }
}

fn usage_window_label(window_seconds: Option<u64>, fallback: &str) -> String {
    match window_seconds {
        Some(18_000) => "5h".to_string(),
        Some(604_800) => "weekly".to_string(),
        Some(seconds) if seconds % 86_400 == 0 => format!("{}d", seconds / 86_400),
        Some(seconds) if seconds % 3_600 == 0 => format!("{}h", seconds / 3_600),
        Some(seconds) if seconds % 60 == 0 => format!("{}m", seconds / 60),
        Some(seconds) => format!("{seconds}s"),
        None => fallback.to_string(),
    }
}

fn format_percent_x100(percent_x100: u32) -> String {
    let percent_x100 = percent_x100.min(10_000);
    if percent_x100.is_multiple_of(100) {
        format!("{}%", percent_x100 / 100)
    } else {
        format!("{:.1}%", percent_x100 as f64 / 100.0)
    }
}

fn parse_display_percent_x100(display: &str) -> Option<u32> {
    let percent = display
        .split_whitespace()
        .next()
        .unwrap_or(display)
        .strip_suffix('%')?;
    let value = percent.parse::<f64>().ok()?;
    Some(percent_to_x100(value))
}

fn percent_to_x100(value: f64) -> u32 {
    if !value.is_finite() {
        return 0;
    }
    ((value.clamp(0.0, 100.0) * 100.0).round()) as u32
}

fn json_number_as_i64(value: &serde_json::Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_u64().and_then(|value| i64::try_from(value).ok()))
        .or_else(|| {
            value
                .as_f64()
                .filter(|value| value.is_finite())
                .map(|value| value.round() as i64)
        })
}

fn json_number_as_u64(value: &serde_json::Value) -> Option<u64> {
    value.as_u64().or_else(|| {
        value
            .as_i64()
            .and_then(|value| u64::try_from(value).ok())
            .or_else(|| {
                value
                    .as_f64()
                    .filter(|value| value.is_finite() && *value >= 0.0)
                    .map(|value| value.round() as u64)
            })
    })
}

fn json_number_as_percent_x100(value: &serde_json::Value) -> Option<u32> {
    let percent = value
        .as_f64()
        .or_else(|| value.as_i64().map(|value| value as f64))?;
    Some(percent_to_x100(percent))
}

fn escape_curl_config(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn metadata_from_jwt(token: &str) -> Option<CodexAccountMetadata> {
    let payload = token.split('.').nth(1)?;
    let bytes = base64_url_decode(payload)?;
    let claims: serde_json::Value = serde_json::from_slice(&bytes).ok()?;

    let email = claims
        .get("email")
        .and_then(serde_json::Value::as_str)
        .or_else(|| {
            claims
                .get("https://api.openai.com/profile")
                .and_then(|profile| profile.get("email"))
                .and_then(serde_json::Value::as_str)
        })
        .filter(|value| !value.trim().is_empty());

    let auth = claims
        .get("https://api.openai.com/auth")
        .and_then(serde_json::Value::as_object);
    let plan = auth
        .and_then(|auth| auth.get("chatgpt_plan_type"))
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(display_plan_type);

    let account_label = email.map(ToString::to_string).or_else(|| {
        auth.and_then(|auth| {
            ["chatgpt_user_id", "user_id", "chatgpt_account_id"]
                .into_iter()
                .find_map(|field| {
                    auth.get(field)
                        .and_then(serde_json::Value::as_str)
                        .filter(|value| !value.trim().is_empty())
                        .map(ToString::to_string)
                })
        })
        .or_else(|| {
            claims
                .get("sub")
                .and_then(serde_json::Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .map(ToString::to_string)
        })
    });

    Some(CodexAccountMetadata {
        account_label,
        plan_label: plan,
    })
}

fn display_plan_type(plan: &str) -> String {
    match plan {
        "free" => "Free".to_string(),
        "go" => "Go".to_string(),
        "plus" => "Plus".to_string(),
        "pro" => "Pro".to_string(),
        "pro_lite" => "Pro Lite".to_string(),
        "team" => "Team".to_string(),
        "business" => "Business".to_string(),
        "enterprise" => "Enterprise".to_string(),
        "edu" | "education" => "Edu".to_string(),
        "self_serve_business_usage_based" => "Business Usage Based".to_string(),
        "enterprise_cbp_usage_based" => "Enterprise Usage Based".to_string(),
        other => other.to_string(),
    }
}

fn base64_url_decode(input: &str) -> Option<Vec<u8>> {
    let mut bits = 0u32;
    let mut bit_count = 0u8;
    let mut output = Vec::new();

    for byte in input.bytes() {
        if byte == b'=' {
            break;
        }
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'-' => 62,
            b'_' => 63,
            _ => return None,
        } as u32;

        bits = (bits << 6) | value;
        bit_count += 6;
        if bit_count >= 8 {
            bit_count -= 8;
            output.push((bits >> bit_count) as u8);
            bits &= (1 << bit_count) - 1;
        }
    }

    Some(output)
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
