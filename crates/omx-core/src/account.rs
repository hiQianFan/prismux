use crate::{Availability, UsageSnapshot};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AccountRef {
    pub platform: String,
    pub local_id: String,
    pub number: u32,
    pub alias: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AccountStatus {
    pub account: AccountRef,
    pub active: bool,
    pub account_label: Option<String>,
    pub plan_label: Option<String>,
    pub auth_type: Option<String>,
    pub expires_at_unix: Option<i64>,
    pub availability: Availability,
    pub usage: Option<UsageSnapshot>,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SwitchReport {
    pub previous: Option<AccountRef>,
    pub current: AccountRef,
}
