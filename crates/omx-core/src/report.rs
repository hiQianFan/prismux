use crate::{AccountRef, Availability, ConfigSwitchReport, PlatformInfo, SwitchReport};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlatformPoolSummary {
    pub platform: PlatformInfo,
    pub account_count: usize,
    pub active: Option<AccountRef>,
    pub profile_count: usize,
    pub active_profile: Option<String>,
    pub availability: Availability,
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
