use std::{
    env, fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use omx_core::{
    AccountRecord, AccountRef, AccountStatus, Availability, AvailabilityState, ConfigProfile,
    ConfigSwitchReport, DoctorCheck, DoctorReport, ImportConfigOptions, ImportedConfig,
    LoginOptions, OpenMuxError, PlatformCapabilities, PlatformInfo, PlatformInstall,
    PlatformPlugin, PlatformPoolSummary, ProfileRecord, RemoveReport, RemovedAccount,
    RemovedConfig, Result, SaveOptions, StateStore, SwitchReport, UpsertAccount, UpsertProfile,
    UsageDiagnostic, UsageLimit, UsageLimitKind, UsageLimitScope, UsageSnapshot, UsageSource,
    UseReport, platform_info,
    storage::{
        create_dir_private, display_path, home_dir, io_error, read_file,
        set_private_file_permissions, sha256_hex, state_root as default_state_root, unix_now,
        unix_now_nanos, write_file_atomic_private,
    },
};

const AUTH_FILE_NAME: &str = "auth.json";

#[derive(Debug, Clone)]
pub struct CodexPlugin {
    codex_home: Option<PathBuf>,
    state_root: Option<PathBuf>,
    codex_executable: PathBuf,
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

        default_state_root()
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

    fn state_store(&self) -> Result<StateStore> {
        StateStore::open(&self.state_root()?)
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

    fn account_snapshot_path(&self, auth_hash: &str) -> Result<PathBuf> {
        Ok(self.accounts_dir()?.join(format!("{auth_hash}.auth.json")))
    }

    #[cfg(test)]
    fn account_snapshot_path_for_number(&self, number: u32) -> Result<PathBuf> {
        let account = self
            .state_store()?
            .account_by_selector(self.id(), &number.to_string())?
            .ok_or_else(|| OpenMuxError::AccountNotFound {
                platform: self.id().to_string(),
                account: number.to_string(),
            })?;
        Ok(PathBuf::from(account.secret_ref))
    }

    fn account_ref(&self, account: &AccountRecord) -> AccountRef {
        AccountRef {
            platform: self.id().to_string(),
            number: account.display_number,
            alias: account.alias.clone(),
        }
    }

    fn account_status(
        &self,
        account: &AccountRecord,
        active_local_id: Option<&str>,
    ) -> AccountStatus {
        let metadata = self.metadata_from_snapshot(account);
        let usage = self.usage_from_snapshot(account);
        let availability = usage.summary.clone();
        AccountStatus {
            active: active_local_id == Some(account.local_id.as_str()),
            account: self.account_ref(account),
            account_label: account.account_label.clone().or(metadata.account_label),
            plan_label: account.plan_label.clone().or(metadata.plan_label),
            auth_type: None,
            expires_at_unix: None,
            availability,
            usage: Some(usage),
        }
    }

    fn metadata_from_snapshot(&self, account: &AccountRecord) -> CodexAccountMetadata {
        read_file(Path::new(&account.secret_ref))
            .ok()
            .map(|bytes| extract_codex_account_metadata(&bytes))
            .unwrap_or_default()
    }

    fn usage_from_snapshot(&self, account: &AccountRecord) -> UsageSnapshot {
        let Some(auth) = read_file(Path::new(&account.secret_ref))
            .ok()
            .and_then(|bytes| parse_codex_usage_auth(&bytes))
        else {
            return self.usage_with_cached_fallback(
                account,
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
                return self.usage_with_cached_fallback(account, diagnostic);
            }
        };

        match parse_codex_usage_snapshot(&payload, unix_now() as i64) {
            Some(usage) => {
                if let Ok(store) = self.state_store() {
                    let _ = store.record_refresh_attempt(
                        &account.local_id,
                        self.id(),
                        "success",
                        None,
                        unix_now(),
                    );
                    let _ = store.save_quota_snapshot(&account.local_id, self.id(), &usage);
                }
                usage
            }
            None => self.usage_with_cached_fallback(
                account,
                UsageDiagnostic {
                    code: "schema".to_string(),
                    message: "Codex usage response did not include known quota fields".to_string(),
                },
            ),
        }
    }

    fn usage_with_cached_fallback(
        &self,
        account: &AccountRecord,
        diagnostic: UsageDiagnostic,
    ) -> UsageSnapshot {
        if let Ok(store) = self.state_store() {
            let _ = store.record_refresh_attempt(
                &account.local_id,
                self.id(),
                "error",
                Some(&diagnostic),
                unix_now(),
            );
        }
        if let Some(mut cached) = self.state_store().ok().and_then(|store| {
            store
                .latest_quota_snapshot(&account.local_id)
                .ok()
                .flatten()
        }) {
            cached.source = UsageSource::StoredSnapshot;
            cached.diagnostics = vec![diagnostic];
            return cached;
        }

        UsageSnapshot::unknown(UsageSource::RemoteApi, diagnostic)
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

    fn resolve_account(&self, selector: &str) -> Result<AccountRecord> {
        self.state_store()?
            .account_by_selector(self.id(), selector)?
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
        let provider_subject_kind = metadata
            .provider_subject
            .as_ref()
            .map(|subject| subject.kind.clone());
        let provider_subject_hash = metadata
            .provider_subject
            .as_ref()
            .map(|subject| provider_subject_hash(self.id(), subject));
        let provider_subject_label = metadata
            .provider_subject
            .as_ref()
            .map(|subject| subject.label.clone());
        let now = unix_now();
        let snapshot_path = self.account_snapshot_path(&auth_hash)?;
        write_file_atomic_private(&snapshot_path, auth_bytes)?;
        let store = self.state_store()?;
        if let Some(alias) = alias.as_deref() {
            let existing = store
                .account_by_selector(self.id(), alias)?
                .map(|account| account.local_id);
            let same_auth = store
                .list_accounts(self.id())?
                .into_iter()
                .find(|account| {
                    account.auth_hash == auth_hash
                        || (provider_subject_kind.is_some()
                            && account.provider_subject_kind == provider_subject_kind
                            && account.provider_subject_hash == provider_subject_hash)
                })
                .map(|account| account.local_id);
            if existing.is_some() && existing != same_auth {
                return Err(OpenMuxError::Message(format!(
                    "alias `{alias}` is already used by another account"
                )));
            }
        }
        let account = store.upsert_account(UpsertAccount {
            provider: self.id().to_string(),
            alias,
            provider_subject_kind,
            provider_subject_hash,
            provider_subject_label,
            account_label: metadata.account_label,
            plan_label: metadata.plan_label,
            auth_type: None,
            expires_at_unix: None,
            auth_hash,
            secret_ref: display_path(&snapshot_path),
            imported_at_unix: now,
        })?;
        let account_ref = self.account_ref(&account);

        if mark_active {
            store.set_active_account(self.id(), &account.local_id, now)?;
        }
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
        let profile = self.state_store()?.upsert_profile(UpsertProfile {
            provider: self.id().to_string(),
            name: imported.profile_name.clone(),
            label: None,
            profile_kind: "config".to_string(),
            provider_id: imported.provider_id.clone(),
            base_url: imported.base_url.clone(),
            model: imported.model.clone(),
            auth_type: imported.auth_type.clone(),
            config_hash: sha256_hex(imported.config_toml.as_bytes()),
            secret_ref: display_path(&profile_path),
            imported_at_unix: unix_now(),
        })?;

        Ok(ImportedConfig {
            platform: self.info(),
            profile_name: imported.profile_name,
            config_path: display_path(&profile_path),
            provider_id: imported.provider_id,
            base_url: imported.base_url,
            model: imported.model,
            number: profile.display_number,
            auth_type: imported.auth_type,
        })
    }

    fn list_codex_profiles(&self) -> Result<Vec<ConfigProfile>> {
        let store = self.state_store()?;
        let active = store.active_profile(self.id())?;
        Ok(store
            .list_profiles(self.id())?
            .into_iter()
            .map(|profile| {
                let is_active = active
                    .as_ref()
                    .is_some_and(|active| active.local_id == profile.local_id);
                profile.to_config_profile(self.info(), is_active)
            })
            .collect())
    }

    fn config_profile_by_selector(&self, selector: &str) -> Result<Option<ProfileRecord>> {
        self.state_store()?.profile_by_selector(self.id(), selector)
    }

    fn switch_to_config_profile(&self, selector: &str) -> Result<ConfigSwitchReport> {
        let profile = self.config_profile_by_selector(selector)?.ok_or_else(|| {
            OpenMuxError::AccountNotFound {
                platform: self.id().to_string(),
                account: selector.to_string(),
            }
        })?;
        let source_path = PathBuf::from(&profile.secret_ref);
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
        if let Err(err) =
            self.state_store()?
                .set_active_profile(self.id(), &profile.local_id, unix_now())
        {
            let rollback = match current_bytes {
                Some(bytes) => write_file_atomic_private(&config_path, &bytes),
                None => fs::remove_file(&config_path)
                    .map_err(|remove_err| io_error(&config_path, remove_err)),
            };
            return match rollback {
                Ok(()) => Err(OpenMuxError::Message(format!(
                    "failed to update state store after switching profile; config was rolled back: {err}"
                ))),
                Err(rollback_err) => Err(OpenMuxError::Message(format!(
                    "failed to update state store after switching profile and rollback failed: {err}; rollback error: {rollback_err}; backup: {}",
                    backup_path.as_deref().unwrap_or("none")
                ))),
            };
        }
        let active_profile = profile.to_config_profile(self.info(), true);
        Ok(ConfigSwitchReport {
            platform: self.info(),
            profile: active_profile,
            config_path: display_path(&config_path),
            backup_path,
        })
    }

    fn remove_account(&self, selector: &str) -> Result<RemovedAccount> {
        let store = self.state_store()?;
        let account = self.resolve_account(selector)?;
        let was_active = store
            .active_account(self.id())?
            .is_some_and(|active| active.local_id == account.local_id);
        let mut removed_paths = Vec::new();

        remove_file_if_exists(Path::new(&account.secret_ref), &mut removed_paths)?;
        store.remove_account(&account.local_id)?;

        Ok(RemovedAccount {
            account: self.account_ref(&account),
            was_active,
            removed_paths,
        })
    }

    fn remove_config_profile(&self, selector: &str) -> Result<RemovedConfig> {
        let store = self.state_store()?;
        let profile = self.config_profile_by_selector(selector)?.ok_or_else(|| {
            OpenMuxError::AccountNotFound {
                platform: self.id().to_string(),
                account: selector.to_string(),
            }
        })?;
        let was_active = store
            .active_profile(self.id())?
            .is_some_and(|active| active.local_id == profile.local_id);
        let mut removed_paths = Vec::new();
        remove_file_if_exists(Path::new(&profile.secret_ref), &mut removed_paths)?;
        store.remove_profile(&profile.local_id)?;

        Ok(RemovedConfig {
            was_active,
            profile: profile.to_config_profile(self.info(), was_active),
            removed_paths,
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
        let store = self.state_store()?;
        let accounts = store.list_accounts(self.id())?;
        let active = store
            .active_account(self.id())?
            .map(|account| self.account_ref(&account));
        let availability = summarize_usage_availability(
            accounts
                .iter()
                .map(|account| self.usage_from_snapshot(account))
                .collect(),
        );

        Ok(PlatformPoolSummary {
            platform: self.info(),
            account_count: accounts.len(),
            active,
            profile_count: store.list_profiles(self.id())?.len(),
            active_profile: store.active_profile(self.id())?.map(|profile| profile.name),
            availability,
        })
    }

    fn current(&self) -> Result<Option<AccountStatus>> {
        let Some(active) = self.state_store()?.active_account(self.id())? else {
            return Ok(None);
        };
        Ok(Some(self.account_status(&active, Some(&active.local_id))))
    }

    fn list_accounts(&self) -> Result<Vec<AccountStatus>> {
        let store = self.state_store()?;
        let active = store.active_account(self.id())?;
        let active_id = active.as_ref().map(|account| account.local_id.as_str());
        Ok(store
            .list_accounts(self.id())?
            .iter()
            .map(|account| self.account_status(account, active_id))
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
        let account_match = self.resolve_account(selector).ok();
        let profile_match = self.config_profile_by_selector(selector)?;

        match (account_match, profile_match) {
            (Some(account), Some(profile)) => Err(OpenMuxError::Message(format!(
                "`{selector}` matches both account #{} and profile `{}`; use a unique alias or profile name",
                account.display_number, profile.name
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

    fn remove_target(&self, selector: &str) -> Result<RemoveReport> {
        let account_match = self.resolve_account(selector).ok();
        let profile_match = self.config_profile_by_selector(selector)?;

        match (account_match, profile_match) {
            (Some(account), Some(profile)) => Err(OpenMuxError::Message(format!(
                "`{selector}` matches both account #{} and profile `{}`; use a unique alias or profile name",
                account.display_number, profile.name
            ))),
            (Some(_), None) => self.remove_account(selector).map(RemoveReport::Account),
            (None, Some(_)) => self
                .remove_config_profile(selector)
                .map(RemoveReport::Config),
            (None, None) => Err(OpenMuxError::AccountNotFound {
                platform: self.id().to_string(),
                account: selector.to_string(),
            }),
        }
    }

    fn switch_to(&self, selector: &str) -> Result<SwitchReport> {
        let store = self.state_store()?;
        let account = self.resolve_account(selector)?;
        let snapshot_path = PathBuf::from(&account.secret_ref);
        if !snapshot_path.exists() {
            return Err(OpenMuxError::Message(format!(
                "stored auth snapshot for account #{} is missing at {}",
                account.display_number,
                display_path(&snapshot_path)
            )));
        }

        let auth_path = self.active_auth_path()?;
        let next_bytes = read_file(&snapshot_path)?;
        let next_hash = sha256_hex(&next_bytes);
        if next_hash != account.auth_hash {
            return Err(OpenMuxError::Message(format!(
                "stored auth snapshot for account #{} failed hash verification",
                account.display_number
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

        let previous = store
            .active_account(self.id())?
            .filter(|current| current.local_id != account.local_id)
            .map(|stored| self.account_ref(&stored));
        if let Err(err) = store.set_active_account(self.id(), &account.local_id, unix_now()) {
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
                    "failed to update state store after switching auth; active auth and config were rolled back: {err}"
                )));
            }
            return Err(OpenMuxError::Message(format!(
                "failed to update state store after switching auth and rollback was incomplete: {err}; auth rollback: {}; config rollback: {}; backup: {}",
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

        let store = self.state_store()?;
        let account = self.resolve_account(selector)?;
        ensure_alias_available(
            &store.list_accounts(self.id())?,
            alias,
            Some(&account.local_id),
        )?;
        store.set_account_alias(&account.local_id, alias, unix_now())?;
        Ok(self.account_ref(
            &store
                .account_by_selector(self.id(), alias)?
                .expect("updated alias should resolve"),
        ))
    }

    fn doctor(&self) -> Result<DoctorReport> {
        let codex_home = self.codex_home()?;
        let state_dir = self.platform_state_dir()?;
        let auth_path = self.active_auth_path()?;
        let state_path = self.state_root()?.join("omx-state.sqlite");
        let state_store = self.state_store();

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
                    name: "state-store".to_string(),
                    ok: state_store.is_ok(),
                    message: display_path(&state_path),
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
    accounts: &[AccountRecord],
    alias: &str,
    allowed_local_id: Option<&str>,
) -> Result<()> {
    if let Some(existing) = accounts.iter().find(|account| {
        account.alias.as_deref() == Some(alias)
            && Some(account.local_id.as_str()) != allowed_local_id
    }) {
        return Err(OpenMuxError::Message(format!(
            "alias `{alias}` is already used by account #{}",
            existing.display_number
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

fn remove_file_if_exists(path: &Path, removed_paths: &mut Vec<String>) -> Result<()> {
    if path.exists() {
        fs::remove_file(path).map_err(|err| io_error(path, err))?;
        removed_paths.push(display_path(path));
    }
    Ok(())
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
    provider_subject: Option<CodexAccountSubject>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CodexAccountSubject {
    kind: String,
    value: String,
    label: String,
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
        let account_id = auth
            .pointer("/tokens/account_id")
            .and_then(serde_json::Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .map(ToString::to_string);
        metadata.account_label = account_id.clone();
        if metadata.provider_subject.is_none() {
            metadata.provider_subject = account_id.map(|value| CodexAccountSubject {
                kind: "codex_account_id".to_string(),
                label: "account_id".to_string(),
                value,
            });
        }
    }

    metadata
}

fn provider_subject_hash(provider: &str, subject: &CodexAccountSubject) -> String {
    sha256_hex(format!("{provider}:{}:{}", subject.kind, subject.value).as_bytes())
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

    let provider_subject = auth
        .and_then(|auth| {
            auth.get("chatgpt_account_id")
                .and_then(serde_json::Value::as_str)
                .filter(|value| !value.trim().is_empty())
                .map(|value| CodexAccountSubject {
                    kind: "codex_chatgpt_account".to_string(),
                    label: "chatgpt_account_id".to_string(),
                    value: value.to_string(),
                })
        })
        .or_else(|| {
            let issuer = claims
                .get("iss")
                .and_then(serde_json::Value::as_str)
                .filter(|value| !value.trim().is_empty())?;
            let subject = claims
                .get("sub")
                .and_then(serde_json::Value::as_str)
                .filter(|value| !value.trim().is_empty())?;
            Some(CodexAccountSubject {
                kind: "oidc_subject".to_string(),
                label: "iss_sub".to_string(),
                value: format!("{issuer}\n{subject}"),
            })
        })
        .or_else(|| {
            auth.and_then(|auth| {
                ["chatgpt_user_id", "user_id"]
                    .into_iter()
                    .find_map(|field| {
                        auth.get(field)
                            .and_then(serde_json::Value::as_str)
                            .filter(|value| !value.trim().is_empty())
                            .map(|value| CodexAccountSubject {
                                kind: "codex_chatgpt_user".to_string(),
                                label: field.to_string(),
                                value: value.to_string(),
                            })
                    })
            })
        });

    Some(CodexAccountMetadata {
        account_label,
        plan_label: plan,
        provider_subject,
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
