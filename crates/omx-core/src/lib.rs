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
    pub alias: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AccountStatus {
    pub account: AccountRef,
    pub active: bool,
    pub display_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SwitchReport {
    pub previous: Option<AccountRef>,
    pub current: AccountRef,
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
    fn current(&self) -> Result<Option<AccountStatus>>;
    fn list_accounts(&self) -> Result<Vec<AccountStatus>>;
    fn import_current(&self, alias: &str) -> Result<AccountRef>;
    fn switch_to(&self, alias: &str) -> Result<SwitchReport>;
    fn doctor(&self) -> Result<DoctorReport>;
}

pub fn platform_info(id: impl Into<String>, name: impl Into<String>) -> PlatformInfo {
    PlatformInfo {
        id: id.into(),
        name: name.into(),
    }
}
