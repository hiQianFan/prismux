use std::{
    env, fs, io,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

use omx_core::{
    AccountRef, AccountStatus, Availability, AvailabilityState, ConfigProfile, ConfigSwitchReport,
    DoctorCheck, DoctorReport, ImportConfigOptions, ImportedConfig, LoginOptions, OpenMuxError,
    PlatformInfo, PlatformInstall, PlatformPlugin, PlatformPoolSummary, Result, SaveOptions,
    SwitchReport, UsageDiagnostic, UsageLimit, UsageLimitKind, UsageLimitScope, UsageSnapshot,
    UsageSource, UseReport, platform_info,
};

const REGISTRY_SCHEMA_VERSION: u32 = 1;
const AUTH_FILE_NAME: &str = "auth.json";

#[derive(Debug, Clone)]
pub struct CodexPlugin {
    codex_home: Option<PathBuf>,
    state_root: Option<PathBuf>,
    codex_executable: PathBuf,
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

impl Default for CodexPlugin {
    fn default() -> Self {
        Self {
            codex_home: None,
            state_root: None,
            codex_executable: env::var_os("OMUX_CODEX_BIN")
                .filter(|value| !value.is_empty())
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("codex")),
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

        if let Some(index) = registry
            .accounts
            .iter()
            .position(|account| account.auth_hash == auth_hash)
        {
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
        })
    }

    fn list_codex_profiles(&self) -> Result<Vec<ConfigProfile>> {
        let codex_home = self.codex_home()?;
        if !codex_home.exists() {
            return Ok(Vec::new());
        }
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
                active: active_config
                    .as_ref()
                    .is_some_and(|active| active.as_slice() == profile_bytes),
                config_path: display_path(&path),
                provider_id: parsed.provider_id,
                base_url: parsed.base_url,
                model: parsed.model,
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
                let active = self
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
        let mut backup_path = None;
        if config_path.exists() {
            let current_bytes = read_file(&config_path)?;
            let default_snapshot_path = self.default_config_snapshot_path()?;
            if !default_snapshot_path.exists() {
                write_file_atomic_private(&default_snapshot_path, &current_bytes)?;
            }
            if current_bytes != next_bytes {
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
        let _ = fs::remove_dir_all(&login_home);
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
        match self.switch_to(selector) {
            Ok(report) => Ok(UseReport::Account(report)),
            Err(OpenMuxError::AccountNotFound { .. }) => self
                .switch_to_config_profile(selector)
                .map(UseReport::Config),
            Err(err) => Err(err),
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
        let changed = auth_path.exists() && read_file(&auth_path)? != next_bytes;
        if changed {
            let backup_path = self
                .backups_dir()?
                .join(format!("auth.json.bak.{}", unix_now_nanos()));
            if let Some(parent) = backup_path.parent() {
                create_dir_private(parent)?;
            }
            fs::copy(&auth_path, &backup_path).map_err(|err| io_error(&backup_path, err))?;
            set_private_file_permissions(&backup_path)?;
        }

        write_file_atomic_private(&auth_path, &next_bytes)?;

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
        self.save_registry(&registry)?;

        Ok(SwitchReport {
            previous,
            current: self.account_ref(&account),
        })
    }

    fn set_alias(&self, selector: &str, alias: &str) -> Result<AccountRef> {
        validate_alias(alias)?;

        let mut registry = self.load_registry()?;
        let number = self.resolve_account(&registry, selector)?.number;
        if let Some(existing) = registry
            .accounts
            .iter()
            .find(|account| account.number != number && account.alias.as_deref() == Some(alias))
        {
            return Err(OpenMuxError::Message(format!(
                "alias `{alias}` is already used by account #{}",
                existing.number
            )));
        }

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

#[derive(Debug, Clone)]
struct ParsedCodexImportConfig {
    profile_name: String,
    provider_id: Option<String>,
    base_url: Option<String>,
    model: Option<String>,
    config_toml: String,
}

#[derive(Debug, Clone)]
struct CodexProfileMetadata {
    provider_id: Option<String>,
    base_url: Option<String>,
    model: Option<String>,
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

fn create_dir_private(path: &Path) -> Result<()> {
    fs::create_dir_all(path).map_err(|err| io_error(path, err))?;
    set_private_dir_permissions(path)
}

fn read_file(path: &Path) -> Result<Vec<u8>> {
    fs::read(path).map_err(|err| io_error(path, err))
}

fn write_file_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        create_dir_private(parent)?;
    }

    let tmp_path = path.with_extension(format!("tmp.{}.{}", std::process::id(), unix_now_nanos()));
    fs::write(&tmp_path, bytes).map_err(|err| io_error(&tmp_path, err))?;
    fs::rename(&tmp_path, path).map_err(|err| {
        let _ = fs::remove_file(&tmp_path);
        io_error(path, err)
    })?;
    Ok(())
}

fn write_file_atomic_private(path: &Path, bytes: &[u8]) -> Result<()> {
    write_file_atomic(path, bytes)?;
    set_private_file_permissions(path)
}

#[cfg(unix)]
fn set_private_file_permissions(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(|err| io_error(path, err))
}

#[cfg(not(unix))]
fn set_private_file_permissions(_path: &Path) -> Result<()> {
    Ok(())
}

#[cfg(unix)]
fn set_private_dir_permissions(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(|err| io_error(path, err))
}

#[cfg(not(unix))]
fn set_private_dir_permissions(_path: &Path) -> Result<()> {
    Ok(())
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}

fn unix_now_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default()
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn encode_registry(registry: &Registry) -> String {
    let mut output = String::new();
    output.push_str(&format!("schema_version\t{}\n", registry.schema_version));
    if let Some(number) = registry.active_number {
        output.push_str(&format!("active_number\t{number}\n"));
    }
    if let Some(number) = registry.previous_active_number {
        output.push_str(&format!("previous_active_number\t{number}\n"));
    }
    output.push_str(&format!("next_number\t{}\n", registry.next_number));
    for account in &registry.accounts {
        output.push_str(&format!(
            "account\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            account.number,
            escape_field(account.alias.as_deref().unwrap_or("")),
            escape_field(account.account_label.as_deref().unwrap_or("")),
            escape_field(account.plan_label.as_deref().unwrap_or("")),
            escape_field(&account.auth_hash),
            escape_field(&account.snapshot_path),
            account.imported_at_unix,
            account
                .last_activated_at_unix
                .map(|value| value.to_string())
                .unwrap_or_default()
        ));
    }
    output
}

fn parse_registry(path: &Path, text: &str) -> Result<Registry> {
    let mut registry = Registry::default();
    let mut saw_schema = false;

    for (line_number, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }

        let fields: Vec<_> = line.split('\t').collect();
        match fields.as_slice() {
            ["schema_version", value] => {
                registry.schema_version = parse_number(path, line_number, value, "schema version")?;
                saw_schema = true;
            }
            ["active_number", value] => {
                registry.active_number =
                    Some(parse_number(path, line_number, value, "active number")?);
            }
            ["previous_active_number", value] => {
                registry.previous_active_number = Some(parse_number(
                    path,
                    line_number,
                    value,
                    "previous active number",
                )?);
            }
            ["next_number", value] => {
                registry.next_number = parse_number(path, line_number, value, "next number")?;
            }
            [
                "account",
                number,
                alias,
                account_label,
                plan_label,
                auth_hash,
                snapshot_path,
                imported_at,
                last_activated_at,
            ] => {
                let imported_at_unix =
                    parse_number(path, line_number, imported_at, "import timestamp")?;
                let last_activated_at_unix = if last_activated_at.is_empty() {
                    None
                } else {
                    Some(parse_number(
                        path,
                        line_number,
                        last_activated_at,
                        "activation timestamp",
                    )?)
                };
                let alias = unescape_field(alias)?;
                let alias = if alias.is_empty() { None } else { Some(alias) };
                validate_alias_option(alias.as_deref())?;
                let account_label = unescape_field(account_label)?;
                let account_label = if account_label.is_empty() {
                    None
                } else {
                    Some(account_label)
                };
                let plan_label = unescape_field(plan_label)?;
                let plan_label = if plan_label.is_empty() {
                    None
                } else {
                    Some(plan_label)
                };

                registry.accounts.push(StoredAccount {
                    number: parse_number(path, line_number, number, "account number")?,
                    alias,
                    account_label,
                    plan_label,
                    auth_hash: unescape_field(auth_hash)?,
                    snapshot_path: unescape_field(snapshot_path)?,
                    imported_at_unix,
                    last_activated_at_unix,
                });
            }
            [
                "account",
                number,
                alias,
                auth_hash,
                snapshot_path,
                imported_at,
                last_activated_at,
            ] => {
                let imported_at_unix =
                    parse_number(path, line_number, imported_at, "import timestamp")?;
                let last_activated_at_unix = if last_activated_at.is_empty() {
                    None
                } else {
                    Some(parse_number(
                        path,
                        line_number,
                        last_activated_at,
                        "activation timestamp",
                    )?)
                };
                let alias = unescape_field(alias)?;
                let alias = if alias.is_empty() { None } else { Some(alias) };
                validate_alias_option(alias.as_deref())?;

                registry.accounts.push(StoredAccount {
                    number: parse_number(path, line_number, number, "account number")?,
                    alias,
                    account_label: None,
                    plan_label: None,
                    auth_hash: unescape_field(auth_hash)?,
                    snapshot_path: unescape_field(snapshot_path)?,
                    imported_at_unix,
                    last_activated_at_unix,
                });
            }
            _ => {
                return Err(OpenMuxError::Message(format!(
                    "{}:{}: unrecognized registry line",
                    display_path(path),
                    line_number + 1
                )));
            }
        }
    }

    if !saw_schema {
        return Err(OpenMuxError::Message(format!(
            "{}: missing schema_version",
            display_path(path)
        )));
    }

    Ok(registry)
}

fn parse_number<T>(path: &Path, line_number: usize, value: &str, label: &str) -> Result<T>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    value.parse().map_err(|err| {
        OpenMuxError::Message(format!(
            "{}:{}: invalid {label}: {err}",
            display_path(path),
            line_number + 1
        ))
    })
}

fn escape_field(value: &str) -> String {
    value
        .replace('%', "%25")
        .replace('\t', "%09")
        .replace('\n', "%0A")
}

fn unescape_field(value: &str) -> Result<String> {
    let mut output = String::new();
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch != '%' {
            output.push(ch);
            continue;
        }

        let code = [
            chars.next().ok_or_else(|| {
                OpenMuxError::Message("invalid percent escape in registry".into())
            })?,
            chars.next().ok_or_else(|| {
                OpenMuxError::Message("invalid percent escape in registry".into())
            })?,
        ];
        match code {
            ['2', '5'] => output.push('%'),
            ['0', '9'] => output.push('\t'),
            ['0', 'A'] => output.push('\n'),
            _ => {
                return Err(OpenMuxError::Message(
                    "invalid percent escape in registry".into(),
                ));
            }
        }
    }
    Ok(output)
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

fn sha256_hex(bytes: &[u8]) -> String {
    let mut state = [
        0x6a09e667u32,
        0xbb67ae85,
        0x3c6ef372,
        0xa54ff53a,
        0x510e527f,
        0x9b05688c,
        0x1f83d9ab,
        0x5be0cd19,
    ];
    let bit_len = (bytes.len() as u64).wrapping_mul(8);
    let mut padded = bytes.to_vec();
    padded.push(0x80);
    while (padded.len() % 64) != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in padded.chunks_exact(64) {
        let mut schedule = [0u32; 64];
        for (index, word) in schedule.iter_mut().take(16).enumerate() {
            let offset = index * 4;
            *word = u32::from_be_bytes([
                chunk[offset],
                chunk[offset + 1],
                chunk[offset + 2],
                chunk[offset + 3],
            ]);
        }
        for index in 16..64 {
            let s0 = schedule[index - 15].rotate_right(7)
                ^ schedule[index - 15].rotate_right(18)
                ^ (schedule[index - 15] >> 3);
            let s1 = schedule[index - 2].rotate_right(17)
                ^ schedule[index - 2].rotate_right(19)
                ^ (schedule[index - 2] >> 10);
            schedule[index] = schedule[index - 16]
                .wrapping_add(s0)
                .wrapping_add(schedule[index - 7])
                .wrapping_add(s1);
        }

        let mut a = state[0];
        let mut b = state[1];
        let mut c = state[2];
        let mut d = state[3];
        let mut e = state[4];
        let mut f = state[5];
        let mut g = state[6];
        let mut h = state[7];

        for index in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(SHA256_K[index])
                .wrapping_add(schedule[index]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        state[0] = state[0].wrapping_add(a);
        state[1] = state[1].wrapping_add(b);
        state[2] = state[2].wrapping_add(c);
        state[3] = state[3].wrapping_add(d);
        state[4] = state[4].wrapping_add(e);
        state[5] = state[5].wrapping_add(f);
        state[6] = state[6].wrapping_add(g);
        state[7] = state[7].wrapping_add(h);
    }

    state.iter().map(|word| format!("{word:08x}")).collect()
}

const SHA256_K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

fn home_dir() -> Option<PathBuf> {
    env::var_os("HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            env::var_os("USERPROFILE")
                .filter(|value| !value.is_empty())
                .map(PathBuf::from)
        })
}

fn data_local_dir() -> Option<PathBuf> {
    if let Some(path) = env::var_os("XDG_DATA_HOME").filter(|value| !value.is_empty()) {
        return Some(PathBuf::from(path));
    }

    #[cfg(target_os = "macos")]
    {
        home_dir().map(|path| path.join("Library").join("Application Support"))
    }

    #[cfg(target_os = "windows")]
    {
        return env::var_os("LOCALAPPDATA")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from);
    }

    #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
    {
        home_dir().map(|path| path.join(".local").join("share"))
    }
}

fn io_error(path: &Path, err: io::Error) -> OpenMuxError {
    OpenMuxError::Message(format!("{}: {err}", display_path(path)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saves_lists_and_switches_codex_auth_snapshots_by_number_and_alias() {
        let temp = test_temp_dir("save-switch");
        let codex_home = temp.join("codex-home");
        let state_root = temp.join("openmux-state");
        fs::create_dir_all(&codex_home).unwrap();
        fs::write(codex_home.join(AUTH_FILE_NAME), br#"{"account":"work"}"#).unwrap();

        let plugin = CodexPlugin::with_paths(&codex_home, &state_root);
        let first = plugin
            .save_current(SaveOptions {
                alias: Some("work".to_string()),
            })
            .unwrap();
        assert_eq!(first.number, 1);
        assert_eq!(first.alias.as_deref(), Some("work"));

        fs::write(
            codex_home.join(AUTH_FILE_NAME),
            br#"{"account":"personal"}"#,
        )
        .unwrap();
        let second = plugin.save_current(SaveOptions::default()).unwrap();
        assert_eq!(second.number, 2);
        assert_eq!(second.alias, None);

        let accounts = plugin.list_accounts().unwrap();
        assert_eq!(accounts.len(), 2);
        assert_eq!(accounts[0].account.number, 1);
        assert_eq!(accounts[1].account.number, 2);

        let report = plugin.switch_to("1").unwrap();
        assert_eq!(report.current.number, 1);
        assert_eq!(
            fs::read(codex_home.join(AUTH_FILE_NAME)).unwrap(),
            br#"{"account":"work"}"#
        );
        assert_eq!(plugin.current().unwrap().unwrap().account.number, 1);
        assert!(plugin.backups_dir().unwrap().exists());

        let report = plugin.switch_to("work").unwrap();
        assert_eq!(report.current.number, 1);
    }

    #[test]
    fn duplicate_save_updates_existing_account_instead_of_appending() {
        let temp = test_temp_dir("duplicate-save");
        let codex_home = temp.join("codex-home");
        let state_root = temp.join("openmux-state");
        fs::create_dir_all(&codex_home).unwrap();
        fs::write(codex_home.join(AUTH_FILE_NAME), br#"{"account":"same"}"#).unwrap();

        let plugin = CodexPlugin::with_paths(&codex_home, &state_root);
        let first = plugin.save_current(SaveOptions::default()).unwrap();
        let second = plugin
            .save_current(SaveOptions {
                alias: Some("same".to_string()),
            })
            .unwrap();

        assert_eq!(first.number, second.number);
        assert_eq!(plugin.list_accounts().unwrap().len(), 1);
        assert_eq!(second.alias.as_deref(), Some("same"));
    }

    #[cfg(unix)]
    #[test]
    fn duplicate_login_updates_existing_account_instead_of_appending() {
        let temp = test_temp_dir("duplicate-login");
        let codex_home = temp.join("codex-home");
        let state_root = temp.join("openmux-state");
        let fake_codex = fake_codex_static_executable(&temp);
        let plugin =
            CodexPlugin::with_paths_and_codex_executable(&codex_home, &state_root, &fake_codex);

        let first = plugin.login(LoginOptions::default()).unwrap();
        let second = plugin
            .login(LoginOptions {
                alias: Some("same".to_string()),
                ..LoginOptions::default()
            })
            .unwrap();

        assert_eq!(first.number, 1);
        assert_eq!(second.number, 1);
        assert_eq!(second.alias.as_deref(), Some("same"));
        assert_eq!(plugin.list_accounts().unwrap().len(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn login_assigns_numbers_supports_alias_device_auth_and_use() {
        let temp = test_temp_dir("login");
        let codex_home = temp.join("codex-home");
        let state_root = temp.join("openmux-state");
        let fake_codex = fake_codex_executable(&temp);
        let plugin =
            CodexPlugin::with_paths_and_codex_executable(&codex_home, &state_root, &fake_codex);

        let first = plugin.login(LoginOptions::default()).unwrap();
        assert_eq!(first.number, 1);
        assert!(plugin.current().unwrap().is_none());

        let second = plugin
            .login(LoginOptions {
                device_auth: true,
                alias: Some("work".to_string()),
                activate: true,
            })
            .unwrap();
        assert_eq!(second.number, 2);
        assert_eq!(second.alias.as_deref(), Some("work"));
        assert_eq!(plugin.current().unwrap().unwrap().account.number, 2);
        assert_eq!(
            fs::read(codex_home.join(AUTH_FILE_NAME)).unwrap(),
            br#"{"account":"2"}"#
        );

        let args_log = fs::read_to_string(fake_codex.with_extension("args")).unwrap();
        assert!(args_log.contains("login\n"));
        assert!(args_log.contains("login --device-auth\n"));
    }

    #[test]
    fn alias_set_rejects_all_digit_aliases() {
        let temp = test_temp_dir("alias");
        let codex_home = temp.join("codex-home");
        let state_root = temp.join("openmux-state");
        fs::create_dir_all(&codex_home).unwrap();
        fs::write(codex_home.join(AUTH_FILE_NAME), br#"{"account":"work"}"#).unwrap();

        let plugin = CodexPlugin::with_paths(&codex_home, &state_root);
        plugin.save_current(SaveOptions::default()).unwrap();
        let err = plugin.set_alias("1", "123").unwrap_err();
        assert!(err.to_string().contains("all digits"));
    }

    #[test]
    fn imports_codex_toml_gateway_config_as_profile_file() {
        let temp = test_temp_dir("import-codex-toml-config");
        let codex_home = temp.join("codex-home");
        let state_root = temp.join("openmux-state");
        let plugin = CodexPlugin::with_paths(&codex_home, &state_root);

        let imported = plugin
            .import_config(ImportConfigOptions {
                name: None,
                content: r#"
model_provider = "codex"
model = "gpt-5.5"
review_model = "gpt-5.5"
disable_response_storage = true

[model_providers.codex]
name = "codex"
base_url = "https://api.apikey.fun"
wire_api = "responses"
requires_openai_auth = true

[features]
goals = true
"#
                .to_string(),
            })
            .unwrap();

        assert_eq!(imported.profile_name, "api-apikey-fun");
        assert_eq!(imported.provider_id.as_deref(), Some("codex"));
        assert_eq!(imported.model.as_deref(), Some("gpt-5.5"));
        let profile = fs::read_to_string(codex_home.join("api-apikey-fun.config.toml")).unwrap();
        assert!(profile.contains("requires_openai_auth = true"));
        assert!(profile.contains("[features]"));

        let profiles = plugin.list_configs().unwrap();
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].name, "api-apikey-fun");
        assert_eq!(profiles[0].provider_id.as_deref(), Some("codex"));
        assert_eq!(
            profiles[0].base_url.as_deref(),
            Some("https://api.apikey.fun")
        );
        assert_eq!(profiles[0].model.as_deref(), Some("gpt-5.5"));
    }

    #[test]
    fn imports_openai_compatible_kv_without_storing_raw_api_key() {
        let temp = test_temp_dir("import-codex-kv-config");
        let codex_home = temp.join("codex-home");
        let state_root = temp.join("openmux-state");
        let plugin = CodexPlugin::with_paths(&codex_home, &state_root);

        let imported = plugin
            .import_config(ImportConfigOptions {
                name: Some("api-key-fun".to_string()),
                content: r#"
export OPENAI_API_KEY=sk-secret
OPENAI_BASE_URL=https://api.apikey.fun/v1
OPENAI_MODEL=gpt-5.5
"#
                .to_string(),
            })
            .unwrap();

        assert_eq!(imported.profile_name, "api-key-fun");
        assert_eq!(imported.provider_id.as_deref(), Some("api-key-fun"));
        let profile = fs::read_to_string(codex_home.join("api-key-fun.config.toml")).unwrap();
        assert!(profile.contains("env_key = \"OPENAI_API_KEY\""));
        assert!(profile.contains("base_url = \"https://api.apikey.fun/v1\""));
        assert!(!profile.contains("sk-secret"));
    }

    #[test]
    fn sha256_matches_known_vector() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn extracts_account_and_plan_from_codex_id_token_like_official_codex() {
        let token = fake_jwt(
            r#"{"alg":"none"}"#,
            r#"{"https://api.openai.com/profile":{"email":"profile@example.com"},"https://api.openai.com/auth":{"chatgpt_plan_type":"pro","chatgpt_user_id":"user-123","chatgpt_account_id":"account-456"}}"#,
        );
        let auth =
            format!(r#"{{"tokens":{{"id_token":"{token}","account_id":"fallback-account"}}}}"#);

        let metadata = extract_codex_account_metadata(auth.as_bytes());
        assert_eq!(
            metadata.account_label.as_deref(),
            Some("profile@example.com")
        );
        assert_eq!(metadata.plan_label.as_deref(), Some("Pro"));
    }

    #[test]
    fn extracts_account_from_account_id_when_jwt_has_no_account_claims() {
        let auth = br#"{"tokens":{"account_id":"account-456"}}"#;
        let metadata = extract_codex_account_metadata(auth);
        assert_eq!(metadata.account_label.as_deref(), Some("account-456"));
        assert_eq!(metadata.plan_label, None);
    }

    #[test]
    fn parses_codex_usage_auth_without_exposing_tokens() {
        let auth = br#"{"tokens":{"access_token":"access-secret","account_id":"account-456"}}"#;
        assert_eq!(
            parse_codex_usage_auth(auth),
            Some(CodexUsageAuth {
                access_token: "access-secret".to_string(),
                account_id: "account-456".to_string(),
            })
        );
    }

    #[test]
    fn parses_curl_http_output_status_without_touching_body() {
        let output = br#"{"ok":true}
200"#;

        let (status, body) = parse_curl_http_output(output).unwrap();
        assert_eq!(status, 200);
        assert_eq!(body, br#"{"ok":true}"#);
    }

    #[test]
    fn parses_curl_http_output_error_status() {
        let output = br#"{"error":{"code":"rate_limited"}}
429"#;

        let (status, body) = parse_curl_http_output(output).unwrap();
        assert_eq!(status, 429);
        assert_eq!(body, br#"{"error":{"code":"rate_limited"}}"#);
    }

    #[test]
    fn parses_legacy_registry_account_lines_without_identity_metadata() {
        let text = concat!(
            "schema_version\t1\n",
            "active_number\t1\n",
            "next_number\t2\n",
            "account\t1\t\tabc123\t/tmp/1.auth.json\t1781516517\t1781516517\n",
        );

        let registry = parse_registry(Path::new("/tmp/registry.omx"), text).unwrap();
        assert_eq!(registry.accounts.len(), 1);
        assert_eq!(registry.accounts[0].number, 1);
        assert_eq!(registry.accounts[0].alias, None);
        assert_eq!(registry.accounts[0].account_label, None);
        assert_eq!(registry.accounts[0].plan_label, None);
        assert_eq!(registry.accounts[0].auth_hash, "abc123");
        assert_eq!(registry.accounts[0].snapshot_path, "/tmp/1.auth.json");
        assert_eq!(registry.accounts[0].imported_at_unix, 1781516517);
        assert_eq!(
            registry.accounts[0].last_activated_at_unix,
            Some(1781516517)
        );
    }

    #[test]
    fn parses_codex_usage_windows_as_structured_limits() {
        let payload = serde_json::json!({
            "rate_limit": {
                "primary_window": {
                    "used_percent": 42,
                    "limit_window_seconds": 18000,
                    "reset_at": 1_725_000_000
                },
                "secondary_window": {
                    "used_percent": 81,
                    "limit_window_seconds": 604800
                }
            }
        });

        let usage = parse_codex_usage_snapshot(&payload, 1_785_000_000).unwrap();
        assert_eq!(usage.summary.display, "19%");
        assert_eq!(usage.summary.state, AvailabilityState::Limited);
        assert_eq!(usage.refreshed_at_unix, Some(1_785_000_000));
        assert_eq!(usage.limits.len(), 2);
        assert_eq!(usage.limits[0].label, "5h");
        assert_eq!(usage.limits[0].remaining_percent_x100, Some(5_800));
        assert_eq!(usage.limits[0].reset_at_unix, Some(1_725_000_000));
        assert_eq!(usage.limits[1].label, "weekly");
        assert_eq!(usage.limits[1].remaining_percent_x100, Some(1_900));
    }

    #[test]
    fn summarizes_known_account_availability_by_tightest_remaining_capacity() {
        let summary = summarize_availability(vec![
            Availability {
                state: AvailabilityState::Available,
                display: "80%".to_string(),
            },
            Availability {
                state: AvailabilityState::Limited,
                display: "20%".to_string(),
            },
        ]);

        assert_eq!(summary.display, "20%");
        assert_eq!(summary.state, AvailabilityState::Limited);
    }

    #[cfg(unix)]
    fn fake_codex_executable(temp: &Path) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;

        let script = temp.join("codex");
        fs::write(
            &script,
            r#"#!/bin/sh
set -eu
count_file="$0.count"
args_file="$0.args"
count=0
if [ -f "$count_file" ]; then
  count="$(cat "$count_file")"
fi
count=$((count + 1))
printf '%s' "$count" > "$count_file"
printf '%s\n' "$*" >> "$args_file"
mkdir -p "$CODEX_HOME"
printf '{"account":"%s"}' "$count" > "$CODEX_HOME/auth.json"
"#,
        )
        .unwrap();
        fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
        script
    }

    #[cfg(unix)]
    fn fake_codex_static_executable(temp: &Path) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;

        let script = temp.join("codex-static");
        fs::write(
            &script,
            r#"#!/bin/sh
set -eu
mkdir -p "$CODEX_HOME"
printf '{"account":"same"}' > "$CODEX_HOME/auth.json"
"#,
        )
        .unwrap();
        fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
        script
    }

    fn fake_jwt(header: &str, payload: &str) -> String {
        format!(
            "{}.{}.signature",
            base64_url_encode(header.as_bytes()),
            base64_url_encode(payload.as_bytes())
        )
    }

    fn base64_url_encode(bytes: &[u8]) -> String {
        const TABLE: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
        let mut output = String::new();
        let mut index = 0;
        while index < bytes.len() {
            let first = bytes[index];
            let second = bytes.get(index + 1).copied();
            let third = bytes.get(index + 2).copied();

            output.push(TABLE[(first >> 2) as usize] as char);
            output.push(
                TABLE[(((first & 0b0000_0011) << 4) | second.unwrap_or(0) >> 4) as usize] as char,
            );
            if let Some(second) = second {
                output.push(
                    TABLE[(((second & 0b0000_1111) << 2) | third.unwrap_or(0) >> 6) as usize]
                        as char,
                );
            }
            if let Some(third) = third {
                output.push(TABLE[(third & 0b0011_1111) as usize] as char);
            }

            index += 3;
        }
        output
    }

    fn test_temp_dir(name: &str) -> PathBuf {
        let path = env::temp_dir().join(format!("openmux-test-{name}-{}", unix_now_nanos()));
        fs::create_dir_all(&path).unwrap();
        path
    }
}
