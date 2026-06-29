use omx_core::{CostStatus, UsagePeriod};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct MenubarQuery {
    pub provider: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MenubarSwitchCommand {
    pub provider: String,
    pub local_id: String,
    #[serde(default)]
    pub target_kind: Option<MenubarTargetKind>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MenubarRemoveCommand {
    pub provider: String,
    pub local_id: String,
    #[serde(default)]
    pub target_kind: Option<MenubarTargetKind>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MenubarConsumeResetCreditCommand {
    pub provider: String,
    pub local_id: String,
    pub idempotency_key: String,
    #[serde(default)]
    pub target_kind: Option<MenubarTargetKind>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MenubarRefreshCommand {
    pub provider: String,
    pub kind: RefreshKind,
    #[serde(default)]
    pub request_generation: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RefreshKind {
    Interactive,
    Background,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MenubarAccountsReport {
    pub control_plane_schema_version: u32,
    pub state_schema_version: u32,
    pub generated_at_unix: u64,
    pub providers: Vec<String>,
    pub accounts: Vec<MenubarAccount>,
    pub profiles: Vec<MenubarProfile>,
    pub active_local_id: Option<String>,
    pub active_target_key: Option<String>,
    pub active_target_kind: Option<MenubarTargetKind>,
    pub system_active_target: Option<MenubarActiveTarget>,
    pub selected_ui_target: Option<MenubarActiveTarget>,
    pub refresh_scope_target: Option<MenubarActiveTarget>,
    pub observed_target: Option<MenubarActiveTarget>,
    pub diagnostics: Vec<MenubarDiagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MenubarDashboardReport {
    pub control_plane_schema_version: u32,
    pub state_schema_version: u32,
    pub generated_at_unix: u64,
    pub accounts: MenubarAccountsReport,
    pub active: Option<MenubarAccount>,
    pub provider_views: Vec<MenubarProviderView>,
    pub usage: MenubarUsageSummary,
    pub provider_usage: Vec<MenubarProviderUsageSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MenubarProviderView {
    pub provider: String,
    pub display_label: String,
    pub status: MenubarAccountStatus,
    pub status_text: String,
    pub status_tone: MenubarViewTone,
    pub target_count: usize,
    pub diagnostics: Vec<MenubarDiagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MenubarSwitchReport {
    pub control_plane_schema_version: u32,
    pub state_schema_version: u32,
    pub generated_at_unix: u64,
    pub provider: String,
    pub requested_local_id: String,
    pub operation: MenubarOperationResult,
    pub dashboard: MenubarDashboardReport,
    pub active_local_id: Option<String>,
    pub accounts: MenubarAccountsReport,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MenubarRefreshReport {
    pub control_plane_schema_version: u32,
    pub state_schema_version: u32,
    pub generated_at_unix: u64,
    pub provider: String,
    pub kind: RefreshKind,
    pub generation: u64,
    pub operation: MenubarOperationResult,
    pub dashboard: MenubarDashboardReport,
    pub refreshed: bool,
    pub skipped_reason: Option<String>,
    pub accounts: MenubarAccountsReport,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MenubarRemoveReport {
    pub control_plane_schema_version: u32,
    pub state_schema_version: u32,
    pub generated_at_unix: u64,
    pub provider: String,
    pub requested_local_id: String,
    pub operation: MenubarOperationResult,
    pub dashboard: MenubarDashboardReport,
    pub accounts: MenubarAccountsReport,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MenubarConsumeResetCreditReport {
    pub control_plane_schema_version: u32,
    pub state_schema_version: u32,
    pub generated_at_unix: u64,
    pub provider: String,
    pub requested_local_id: String,
    pub operation: MenubarOperationResult,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outcome: Option<MenubarResetCreditOutcome>,
    pub dashboard: MenubarDashboardReport,
    pub accounts: MenubarAccountsReport,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum MenubarResetCreditOutcome {
    Reset { windows_reset: u32 },
    NothingToReset,
    NoCredit,
    AlreadyRedeemed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MenubarOperationResult {
    pub status: MenubarOperationStatus,
    pub changed: bool,
    pub active_before: Option<MenubarActiveTarget>,
    pub active_after: Option<MenubarActiveTarget>,
    pub message: String,
    pub diagnostics: Vec<MenubarDiagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MenubarOperationStatus {
    Success,
    Skipped,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MenubarActiveTarget {
    pub provider: String,
    pub target_kind: MenubarTargetKind,
    pub local_id: String,
    pub account_key: String,
    pub display_label: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MenubarTargetKind {
    Account,
    Profile,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MenubarAccount {
    pub provider: String,
    pub account_key: String,
    pub target_kind: MenubarTargetKind,
    pub display_number: u32,
    pub local_id: String,
    pub display_label: String,
    pub secondary_label: String,
    pub alias: Option<String>,
    pub account_label: Option<String>,
    pub plan: Option<String>,
    pub auth_type: Option<String>,
    pub active: bool,
    pub quota: Option<MenubarQuota>,
    pub status: MenubarAccountStatus,
    pub actions: MenubarTargetActions,
    pub diagnostic: Option<MenubarDiagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MenubarProfile {
    pub provider: String,
    pub account_key: String,
    pub target_kind: MenubarTargetKind,
    pub display_number: u32,
    pub local_id: String,
    pub display_label: String,
    pub secondary_label: String,
    pub name: String,
    pub active: bool,
    pub provider_id: Option<String>,
    pub base_url: Option<String>,
    pub model: Option<String>,
    pub auth_type: Option<String>,
    pub status: MenubarAccountStatus,
    pub actions: MenubarTargetActions,
    pub diagnostic: Option<MenubarDiagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MenubarTargetActions {
    pub can_activate: bool,
    pub can_remove: bool,
    pub primary_label: String,
    pub disabled_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MenubarQuota {
    pub summary: String,
    pub refreshed_at_unix: Option<i64>,
    pub primary_window: Option<MenubarQuotaWindow>,
    pub windows: Vec<MenubarQuotaWindow>,
    pub reset_credits: Option<MenubarResetCredits>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MenubarResetCredits {
    pub available_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MenubarQuotaWindow {
    pub id: String,
    pub label: String,
    pub window_seconds: Option<u64>,
    pub used_percent_x100: Option<u32>,
    pub remaining_percent_x100: Option<u32>,
    pub reset_at_unix: Option<i64>,
    pub exhausted: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MenubarAccountStatus {
    Healthy,
    Limited,
    Exhausted,
    Stale,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MenubarViewTone {
    Neutral,
    Success,
    Warning,
    Danger,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MenubarDiagnostic {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recovery_action: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MenubarUsageSummary {
    pub period: UsagePeriod,
    pub total_tokens: u64,
    pub top_client: Option<String>,
    pub top_model: Option<String>,
    pub model_breakdown: Vec<MenubarUsageModelBreakdown>,
    pub hourly_buckets: Vec<MenubarHourlyBucket>,
    #[serde(default)]
    pub series: Vec<MenubarUsageChartSeries>,
    pub cost_status: CostStatus,
    pub estimated_cost_usd: Option<String>,
    pub freshness: MenubarFreshness,
    pub coverage: MenubarCoverage,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MenubarUsageModelBreakdown {
    pub model: String,
    pub total_tokens: u64,
}

/// One local-hour bucket of token usage. The hour is the canonical unit: the
/// frontend renders today as 24 hourly bars and rolls hours up into days for the
/// 7d/30d views (a day is the `YYYY-MM-DD` prefix of `local_hour`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MenubarHourlyBucket {
    /// Local hour, ISO-like, e.g. "2026-06-27T14".
    pub local_hour: String,
    pub total_tokens: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MenubarUsageChartSeriesKind {
    Provider,
    Model,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MenubarUsageChartSeries {
    pub kind: MenubarUsageChartSeriesKind,
    pub key: String,
    pub label: String,
    pub hourly_buckets: Vec<MenubarHourlyBucket>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MenubarProviderUsageSummary {
    pub provider: String,
    pub usage: MenubarUsageSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MenubarFreshness {
    pub generated_at_unix: u64,
    pub stale: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MenubarCoverage {
    pub status: String,
    pub tone: MenubarViewTone,
    pub requested_clients: Vec<String>,
    pub available_clients: Vec<String>,
    pub missing_clients: Vec<String>,
}
