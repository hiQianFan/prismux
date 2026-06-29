use omx_core::{CostStatus, UsagePeriod};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct DashboardQuery {
    pub provider: Option<String>,
    #[serde(default)]
    pub usage_period: Option<UsagePeriod>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SwitchCommand {
    pub provider: String,
    pub local_id: String,
    #[serde(default)]
    pub target_kind: Option<TargetKindView>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemoveCommand {
    pub provider: String,
    pub local_id: String,
    #[serde(default)]
    pub target_kind: Option<TargetKindView>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConsumeResetCreditCommand {
    pub provider: String,
    pub local_id: String,
    pub idempotency_key: String,
    #[serde(default)]
    pub target_kind: Option<TargetKindView>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RefreshCommand {
    pub provider: String,
    pub kind: RefreshKind,
    #[serde(default)]
    pub local_id: Option<String>,
    #[serde(default)]
    pub target_kind: Option<TargetKindView>,
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
pub struct AccountsReport {
    pub control_plane_schema_version: u32,
    pub state_schema_version: u32,
    pub generated_at_unix: u64,
    pub providers: Vec<String>,
    pub accounts: Vec<TargetAccount>,
    pub profiles: Vec<TargetProfile>,
    pub active_local_id: Option<String>,
    pub active_target_key: Option<String>,
    pub active_target_kind: Option<TargetKindView>,
    pub system_active_target: Option<ActiveTarget>,
    pub selected_ui_target: Option<ActiveTarget>,
    pub refresh_scope_target: Option<ActiveTarget>,
    pub observed_target: Option<ActiveTarget>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DashboardReport {
    pub control_plane_schema_version: u32,
    pub state_schema_version: u32,
    pub generated_at_unix: u64,
    pub accounts: AccountsReport,
    pub active: Option<TargetAccount>,
    pub provider_views: Vec<ProviderView>,
    pub aggregate: DashboardAggregateView,
    pub usage: UsageSummaryView,
    pub provider_usage: Vec<ProviderUsageSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderView {
    pub provider: String,
    pub display_label: String,
    pub status: TargetStatus,
    pub status_text: String,
    pub status_tone: ViewTone,
    pub target_count: usize,
    pub aggregate: ProviderAggregateView,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DashboardAggregateView {
    pub quota_health: QuotaHealthRollup,
    pub provider_aggregates: Vec<ProviderAggregateView>,
    pub usage_headline: UsageHeadline,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderAggregateView {
    pub provider_id: String,
    pub provider_display_label: String,
    pub account_count: u32,
    pub profile_count: u32,
    pub target_count: u32,
    pub active_target: Option<ActiveTarget>,
    pub quota_health: QuotaHealthRollup,
    /// This provider's token/cost headline for the selected period. Lets the
    /// Overview render a per-provider usage line without re-joining provider_usage.
    pub usage_headline: UsageHeadline,
    pub status: TargetStatus,
    pub status_tone: ViewTone,
    pub status_text: String,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct QuotaFactsRollup {
    pub account_count: u32,
    pub reporting_count: u32,
    pub avg_remaining_percent_x100: Option<u32>,
    pub min_remaining_percent_x100: Option<u32>,
    pub max_remaining_percent_x100: Option<u32>,
    pub soonest_reset_at_unix: Option<i64>,
    pub reset_credit_total: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QuotaHealthRollup {
    pub facts: QuotaFactsRollup,
    pub healthy_count: u32,
    pub low_count: u32,
    pub exhausted_count: u32,
    pub worst_target: Option<ActiveTarget>,
    pub best_alternative: Option<TargetRecommendation>,
    pub status: TargetStatus,
    pub status_tone: ViewTone,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TargetRecommendation {
    pub target: ActiveTarget,
    pub reason: String,
    pub action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UsageHeadline {
    pub period: UsagePeriod,
    pub total_tokens: u64,
    pub estimated_cost_usd: Option<String>,
    pub cost_status: CostStatus,
    pub top_client: Option<String>,
    pub top_model: Option<String>,
    pub breakdown: Vec<UsageModelBreakdown>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SwitchReport {
    pub control_plane_schema_version: u32,
    pub state_schema_version: u32,
    pub generated_at_unix: u64,
    pub provider: String,
    pub requested_local_id: String,
    pub operation: OperationResult,
    pub dashboard: DashboardReport,
    pub active_local_id: Option<String>,
    pub accounts: AccountsReport,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RefreshReport {
    pub control_plane_schema_version: u32,
    pub state_schema_version: u32,
    pub generated_at_unix: u64,
    pub provider: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requested_local_id: Option<String>,
    pub kind: RefreshKind,
    pub generation: u64,
    pub operation: OperationResult,
    pub dashboard: DashboardReport,
    pub refreshed: bool,
    pub skipped_reason: Option<String>,
    pub accounts: AccountsReport,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemoveReportView {
    pub control_plane_schema_version: u32,
    pub state_schema_version: u32,
    pub generated_at_unix: u64,
    pub provider: String,
    pub requested_local_id: String,
    pub operation: OperationResult,
    pub dashboard: DashboardReport,
    pub accounts: AccountsReport,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConsumeResetCreditReport {
    pub control_plane_schema_version: u32,
    pub state_schema_version: u32,
    pub generated_at_unix: u64,
    pub provider: String,
    pub requested_local_id: String,
    pub operation: OperationResult,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outcome: Option<ResetCreditOutcomeView>,
    pub dashboard: DashboardReport,
    pub accounts: AccountsReport,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ResetCreditOutcomeView {
    Reset { windows_reset: u32 },
    NothingToReset,
    NoCredit,
    AlreadyRedeemed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OperationResult {
    pub status: OperationStatus,
    pub changed: bool,
    pub active_before: Option<ActiveTarget>,
    pub active_after: Option<ActiveTarget>,
    pub message: String,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OperationStatus {
    Success,
    Skipped,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ActiveTarget {
    pub provider: String,
    pub target_kind: TargetKindView,
    pub local_id: String,
    pub account_key: String,
    pub display_label: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TargetKindView {
    Account,
    Profile,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TargetAccount {
    pub provider: String,
    pub account_key: String,
    pub target_kind: TargetKindView,
    pub display_number: u32,
    pub local_id: String,
    pub display_label: String,
    pub secondary_label: String,
    pub alias: Option<String>,
    pub account_label: Option<String>,
    pub plan: Option<String>,
    pub auth_type: Option<String>,
    pub active: bool,
    pub quota: Option<QuotaView>,
    pub status: TargetStatus,
    pub actions: TargetActions,
    pub diagnostic: Option<Diagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TargetProfile {
    pub provider: String,
    pub account_key: String,
    pub target_kind: TargetKindView,
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
    pub status: TargetStatus,
    pub actions: TargetActions,
    pub diagnostic: Option<Diagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TargetActions {
    pub can_activate: bool,
    pub can_remove: bool,
    pub primary_label: String,
    pub disabled_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QuotaView {
    pub summary: String,
    pub refreshed_at_unix: Option<i64>,
    pub primary_window: Option<QuotaWindow>,
    pub windows: Vec<QuotaWindow>,
    pub reset_credits: Option<ResetCreditsView>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResetCreditsView {
    pub available_count: u32,
    #[serde(default)]
    pub credits: Vec<ResetCreditView>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResetCreditView {
    pub status: Option<String>,
    pub reset_type: Option<String>,
    pub granted_at_unix: Option<i64>,
    pub expires_at_unix: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QuotaWindow {
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
pub enum TargetStatus {
    Healthy,
    Limited,
    Exhausted,
    Stale,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ViewTone {
    Neutral,
    Success,
    Warning,
    Danger,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recovery_action: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UsageSummaryView {
    pub period: UsagePeriod,
    pub total_tokens: u64,
    pub top_client: Option<String>,
    pub top_model: Option<String>,
    pub model_breakdown: Vec<UsageModelBreakdown>,
    pub hourly_buckets: Vec<HourlyBucket>,
    #[serde(default)]
    pub series: Vec<UsageChartSeries>,
    pub cost_status: CostStatus,
    pub estimated_cost_usd: Option<String>,
    pub freshness: Freshness,
    pub coverage: Coverage,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UsageModelBreakdown {
    pub model: String,
    pub total_tokens: u64,
}

/// One local-hour bucket of token usage. The hour is the canonical unit: the
/// frontend renders today as 24 hourly bars and rolls hours up into days for the
/// 7d/30d views (a day is the `YYYY-MM-DD` prefix of `local_hour`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HourlyBucket {
    /// Local hour, ISO-like, e.g. "2026-06-27T14".
    pub local_hour: String,
    pub total_tokens: u64,
    pub estimated_cost_usd: Option<String>,
    pub cost_status: CostStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum UsageChartSeriesKind {
    Provider,
    Model,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UsageChartSeries {
    pub kind: UsageChartSeriesKind,
    pub key: String,
    pub label: String,
    pub hourly_buckets: Vec<HourlyBucket>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderUsageSummary {
    pub provider: String,
    pub usage: UsageSummaryView,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Freshness {
    pub generated_at_unix: u64,
    pub stale: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Coverage {
    pub status: String,
    pub tone: ViewTone,
    pub requested_clients: Vec<String>,
    pub available_clients: Vec<String>,
    pub missing_clients: Vec<String>,
}
