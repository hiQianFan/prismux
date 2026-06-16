use serde::{Deserialize, Serialize};

pub type Result<T> = std::result::Result<T, OpenMuxError>;

#[derive(Debug, thiserror::Error)]
pub enum OpenMuxError {
    #[error("platform `{0}` is not installed or could not be detected")]
    PlatformNotDetected(String),
    #[error("account `{account}` was not found for platform `{platform}`")]
    AccountNotFound { platform: String, account: String },
    #[error("{0}")]
    Message(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlatformInfo {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlatformInstall {
    pub platform: PlatformInfo,
    pub config_path: Option<String>,
    pub auth_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AccountRef {
    pub platform: String,
    pub number: u32,
    pub alias: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AccountStatus {
    pub account: AccountRef,
    pub active: bool,
    pub account_label: Option<String>,
    pub plan_label: Option<String>,
    pub availability: Availability,
    pub usage: Option<UsageSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlatformPoolSummary {
    pub platform: PlatformInfo,
    pub account_count: usize,
    pub active: Option<AccountRef>,
    pub availability: Availability,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Availability {
    pub state: AvailabilityState,
    pub display: String,
}

impl Availability {
    pub fn unknown() -> Self {
        Self {
            state: AvailabilityState::Unknown,
            display: "unknown".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AvailabilityState {
    Unknown,
    Available,
    Limited,
    Exhausted,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UsageSnapshot {
    pub source: UsageSource,
    pub refreshed_at_unix: Option<i64>,
    pub summary: Availability,
    pub limits: Vec<UsageLimit>,
    pub diagnostics: Vec<UsageDiagnostic>,
}

impl UsageSnapshot {
    pub fn unknown(source: UsageSource, diagnostic: UsageDiagnostic) -> Self {
        Self {
            source,
            refreshed_at_unix: None,
            summary: Availability::unknown(),
            limits: Vec::new(),
            diagnostics: vec![diagnostic],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum UsageSource {
    RemoteApi,
    LocalSession,
    StoredSnapshot,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UsageLimit {
    pub id: String,
    pub label: String,
    pub scope: UsageLimitScope,
    pub kind: UsageLimitKind,
    pub window_seconds: Option<u64>,
    pub used_percent_x100: Option<u32>,
    pub remaining_percent_x100: Option<u32>,
    pub reset_at_unix: Option<i64>,
    pub exhausted: Option<bool>,
    pub raw_provider_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum UsageLimitScope {
    Account,
    Workspace,
    Project,
    Model,
    Feature,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum UsageLimitKind {
    RollingWindow,
    CalendarWindow,
    CreditBalance,
    RequestRate,
    TokenRate,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UsageDiagnostic {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct LoginOptions {
    pub device_auth: bool,
    pub alias: Option<String>,
    pub activate: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SaveOptions {
    pub alias: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImportConfigOptions {
    pub name: Option<String>,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImportedConfig {
    pub platform: PlatformInfo,
    pub profile_name: String,
    pub config_path: String,
    pub provider_id: Option<String>,
    pub base_url: Option<String>,
    pub model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConfigProfile {
    pub platform: PlatformInfo,
    pub name: String,
    pub active: bool,
    pub config_path: String,
    pub provider_id: Option<String>,
    pub base_url: Option<String>,
    pub model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SwitchReport {
    pub previous: Option<AccountRef>,
    pub current: AccountRef,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConfigSwitchReport {
    pub platform: PlatformInfo,
    pub profile: ConfigProfile,
    pub config_path: String,
    pub backup_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum UseReport {
    Account(SwitchReport),
    Config(ConfigSwitchReport),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DoctorReport {
    pub platform: String,
    pub checks: Vec<DoctorCheck>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DoctorCheck {
    pub name: String,
    pub ok: bool,
    pub message: String,
}

pub trait PlatformPlugin {
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;

    fn detect(&self) -> Result<PlatformInstall>;
    fn pool_summary(&self) -> Result<PlatformPoolSummary>;
    fn current(&self) -> Result<Option<AccountStatus>>;
    fn list_accounts(&self) -> Result<Vec<AccountStatus>>;
    fn list_configs(&self) -> Result<Vec<ConfigProfile>> {
        Ok(Vec::new())
    }
    fn login(&self, options: LoginOptions) -> Result<AccountRef>;
    fn save_current(&self, options: SaveOptions) -> Result<AccountRef>;
    fn import_config(&self, options: ImportConfigOptions) -> Result<ImportedConfig>;
    fn use_target(&self, selector: &str) -> Result<UseReport> {
        self.switch_to(selector).map(UseReport::Account)
    }
    fn switch_to(&self, selector: &str) -> Result<SwitchReport>;
    fn set_alias(&self, selector: &str, alias: &str) -> Result<AccountRef>;
    fn doctor(&self) -> Result<DoctorReport>;
}

pub fn platform_info(id: impl Into<String>, name: impl Into<String>) -> PlatformInfo {
    PlatformInfo {
        id: id.into(),
        name: name.into(),
    }
}
