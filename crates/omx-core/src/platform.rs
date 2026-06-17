use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlatformCapabilities {
    pub accounts: bool,
    pub account_login: bool,
    pub account_save: bool,
    pub account_import: bool,
    pub profiles: bool,
    pub profile_import: bool,
}

impl PlatformCapabilities {
    pub fn account_pool() -> Self {
        Self {
            accounts: true,
            account_login: true,
            account_save: true,
            account_import: false,
            profiles: false,
            profile_import: false,
        }
    }
}

pub fn platform_info(id: impl Into<String>, name: impl Into<String>) -> PlatformInfo {
    PlatformInfo {
        id: id.into(),
        name: name.into(),
    }
}
