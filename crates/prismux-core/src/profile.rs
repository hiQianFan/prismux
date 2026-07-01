use crate::PlatformInfo;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImportConfigOptions {
    pub name: Option<String>,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProfileRef {
    pub platform: String,
    pub local_id: String,
    pub number: Option<u32>,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImportedConfig {
    pub platform: PlatformInfo,
    pub profile_name: String,
    pub config_path: String,
    pub provider_id: Option<String>,
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub number: Option<u32>,
    pub auth_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConfigProfile {
    pub platform: PlatformInfo,
    pub local_id: String,
    pub name: String,
    pub active: bool,
    pub config_path: String,
    pub provider_id: Option<String>,
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub number: Option<u32>,
    pub auth_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConfigSwitchReport {
    pub platform: PlatformInfo,
    pub profile: ConfigProfile,
    pub config_path: String,
    pub backup_path: Option<String>,
}
