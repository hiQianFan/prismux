use serde::{Deserialize, Serialize};

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
    pub reset_credits: Option<UsageResetCredits>,
    pub diagnostics: Vec<UsageDiagnostic>,
}

impl UsageSnapshot {
    pub fn unknown(source: UsageSource, diagnostic: UsageDiagnostic) -> Self {
        Self {
            source,
            refreshed_at_unix: None,
            summary: Availability::unknown(),
            limits: Vec::new(),
            reset_credits: None,
            diagnostics: vec![diagnostic],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UsageResetCredits {
    pub available_count: u32,
    #[serde(default)]
    pub credits: Vec<UsageResetCredit>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UsageResetCredit {
    pub status: Option<String>,
    pub reset_type: Option<String>,
    pub granted_at_unix: Option<i64>,
    pub expires_at_unix: Option<i64>,
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
