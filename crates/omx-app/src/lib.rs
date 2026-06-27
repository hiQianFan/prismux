use chrono::{Datelike, Local, TimeZone};
use omx_core::{
    AccountStatus, AvailabilityState, ConfigProfile, CostStatus, OpenMuxError, PlatformPlugin,
    RemoveReport, Result, TargetCatalog, TargetKind, TargetResolution, UsageLimit, UsagePeriod,
    UsageSnapshot, UsageSummary, UsageSummaryQuery, UseReport, storage::unix_now,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex},
};

static OPERATION_LOCK: Mutex<()> = Mutex::new(());
static REFRESH_STATE: LazyLock<Mutex<HashMap<String, RefreshState>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

const INTERACTIVE_REFRESH_FLOOR_SECONDS: u64 = 30;
const BACKGROUND_REFRESH_FLOOR_SECONDS: u64 = 300;
const REFRESH_ERROR_BACKOFF_SECONDS: u64 = 120;

#[doc(hidden)]
pub fn reset_menubar_refresh_state_for_tests() {
    REFRESH_STATE
        .lock()
        .unwrap_or_else(|err| err.into_inner())
        .clear();
}

#[derive(Debug, Clone, Default)]
struct RefreshState {
    last_attempt_unix: Option<u64>,
    last_success_unix: Option<u64>,
    last_error_unix: Option<u64>,
}

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
pub struct MenubarRefreshCommand {
    pub provider: String,
    pub kind: RefreshKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RefreshKind {
    Interactive,
    Background,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MenubarAccountsReport {
    pub generated_at_unix: u64,
    pub providers: Vec<String>,
    pub accounts: Vec<MenubarAccount>,
    pub profiles: Vec<MenubarProfile>,
    pub active_local_id: Option<String>,
    pub active_target_key: Option<String>,
    pub active_target_kind: Option<MenubarTargetKind>,
    pub diagnostics: Vec<MenubarDiagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MenubarDashboardReport {
    pub generated_at_unix: u64,
    pub accounts: MenubarAccountsReport,
    pub active: Option<MenubarAccount>,
    pub usage: MenubarUsageSummary,
    pub provider_usage: Vec<MenubarProviderUsageSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MenubarSwitchReport {
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
    pub generated_at_unix: u64,
    pub provider: String,
    pub kind: RefreshKind,
    pub operation: MenubarOperationResult,
    pub dashboard: MenubarDashboardReport,
    pub refreshed: bool,
    pub skipped_reason: Option<String>,
    pub accounts: MenubarAccountsReport,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MenubarRemoveReport {
    pub generated_at_unix: u64,
    pub provider: String,
    pub requested_local_id: String,
    pub operation: MenubarOperationResult,
    pub dashboard: MenubarDashboardReport,
    pub accounts: MenubarAccountsReport,
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
    pub diagnostic: Option<MenubarDiagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MenubarQuota {
    pub summary: String,
    pub refreshed_at_unix: Option<i64>,
    pub primary_window: Option<MenubarQuotaWindow>,
    pub windows: Vec<MenubarQuotaWindow>,
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
pub struct MenubarDiagnostic {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MenubarUsageSummary {
    pub period: UsagePeriod,
    pub total_tokens: u64,
    pub top_client: Option<String>,
    pub top_model: Option<String>,
    pub model_breakdown: Vec<MenubarUsageModelBreakdown>,
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
    pub requested_clients: Vec<String>,
    pub available_clients: Vec<String>,
    pub missing_clients: Vec<String>,
}

pub fn menubar_accounts(
    plugins: &[Box<dyn PlatformPlugin>],
    query: MenubarQuery,
) -> Result<MenubarAccountsReport> {
    let mut diagnostics = Vec::new();
    let mut accounts = Vec::new();
    let mut profiles = Vec::new();
    let mut providers = Vec::new();
    for plugin in selected_plugins(plugins, query.provider.as_deref())? {
        providers.push(plugin.id().to_string());
        match plugin.list_accounts() {
            Ok(statuses) => accounts.extend(statuses.iter().map(account_from_status)),
            Err(err) => diagnostics.push(MenubarDiagnostic {
                code: "provider_unavailable".to_string(),
                message: sanitize_diagnostic(&err.to_string()),
            }),
        }
        match plugin.list_configs() {
            Ok(configs) => profiles.extend(configs.iter().map(profile_from_config)),
            Err(err) => diagnostics.push(MenubarDiagnostic {
                code: "profiles_unavailable".to_string(),
                message: sanitize_diagnostic(&err.to_string()),
            }),
        }
    }
    sort_accounts(&mut accounts);
    sort_profiles(&mut profiles);
    for provider in &providers {
        let active_accounts = accounts
            .iter()
            .filter(|account| account.provider == *provider && account.active)
            .count();
        let active_profiles = profiles
            .iter()
            .filter(|profile| profile.provider == *provider && profile.active)
            .count();
        if active_accounts + active_profiles > 1 {
            diagnostics.push(MenubarDiagnostic {
                code: "multiple_active_targets".to_string(),
                message: format!(
                    "`{provider}` reported multiple active targets; using one active target"
                ),
            });
        }
    }
    normalize_active_targets(&mut accounts, &mut profiles);
    let active_account = accounts.iter().find(|account| account.active);
    let active_profile = profiles.iter().find(|profile| profile.active);
    let active_local_id = active_account
        .map(|account| account.local_id.clone())
        .or_else(|| active_profile.map(|profile| profile.local_id.clone()));
    let active_target_key = active_account
        .map(|account| account.account_key.clone())
        .or_else(|| active_profile.map(|profile| profile.account_key.clone()));
    let active_target_kind = active_account
        .map(|_| MenubarTargetKind::Account)
        .or_else(|| active_profile.map(|_| MenubarTargetKind::Profile));
    Ok(MenubarAccountsReport {
        generated_at_unix: unix_now(),
        providers,
        accounts,
        profiles,
        active_local_id,
        active_target_key,
        active_target_kind,
        diagnostics,
    })
}

pub fn account_statuses(plugin: &dyn PlatformPlugin) -> Result<Vec<AccountStatus>> {
    plugin.list_accounts()
}

pub fn config_profiles(plugin: &dyn PlatformPlugin) -> Result<Vec<ConfigProfile>> {
    plugin.list_configs()
}

pub fn active_account_status(plugin: &dyn PlatformPlugin) -> Result<Option<AccountStatus>> {
    plugin.current()
}

pub fn target_catalog(plugin: &dyn PlatformPlugin) -> Result<TargetCatalog> {
    Ok(TargetCatalog::new(
        account_statuses(plugin)?,
        config_profiles(plugin)?,
    ))
}

pub fn resolve_target(plugin: &dyn PlatformPlugin, selector: &str) -> Result<TargetResolution> {
    target_catalog(plugin)?.resolve(plugin.id(), selector)
}

pub fn use_resolved_target(plugin: &dyn PlatformPlugin, selector: &str) -> Result<UseReport> {
    let target = resolve_target(plugin, selector)?;
    plugin.use_target(&target.target_id)
}

pub fn remove_resolved_target(plugin: &dyn PlatformPlugin, selector: &str) -> Result<RemoveReport> {
    let target = resolve_target(plugin, selector)?;
    plugin.remove_target(&target.target_id)
}

pub fn menubar_switch(
    plugins: &[Box<dyn PlatformPlugin>],
    command: MenubarSwitchCommand,
    store: Option<&omx_core::StateStore>,
) -> Result<MenubarSwitchReport> {
    let _guard = OPERATION_LOCK
        .try_lock()
        .map_err(|_| OpenMuxError::Message("menubar operation already in progress".to_string()))?;
    let plugin = find_plugin(plugins, &command.provider)?;
    let before_catalog = target_catalog(plugin)?;
    let active_before = active_target(plugin.id(), &before_catalog);
    let target = resolve_menubar_target(plugin.id(), &before_catalog, &command)?;
    plugin.use_target(&target.target_id)?;
    let dashboard = menubar_dashboard(plugins, MenubarQuery { provider: None }, store)?;
    let active_after = active_target_for_provider_from_report(plugin.id(), &dashboard.accounts);
    let changed = active_before.as_ref().map(|target| &target.account_key)
        != active_after.as_ref().map(|target| &target.account_key);
    Ok(MenubarSwitchReport {
        generated_at_unix: unix_now(),
        provider: command.provider,
        requested_local_id: command.local_id,
        operation: MenubarOperationResult {
            status: MenubarOperationStatus::Success,
            changed,
            active_before,
            active_after,
            message: if changed {
                "Active target switched.".to_string()
            } else {
                "Target was already active.".to_string()
            },
            diagnostics: Vec::new(),
        },
        active_local_id: dashboard.accounts.active_local_id.clone(),
        accounts: dashboard.accounts.clone(),
        dashboard,
    })
}

pub fn menubar_remove(
    plugins: &[Box<dyn PlatformPlugin>],
    command: MenubarRemoveCommand,
    store: Option<&omx_core::StateStore>,
) -> Result<MenubarRemoveReport> {
    let _guard = OPERATION_LOCK
        .try_lock()
        .map_err(|_| OpenMuxError::Message("menubar operation already in progress".to_string()))?;
    let plugin = find_plugin(plugins, &command.provider)?;
    let before_catalog = target_catalog(plugin)?;
    let active_before = active_target(plugin.id(), &before_catalog);
    let target = resolve_menubar_target(
        plugin.id(),
        &before_catalog,
        &MenubarSwitchCommand {
            provider: command.provider.clone(),
            local_id: command.local_id.clone(),
            target_kind: command.target_kind,
        },
    )?;
    plugin.remove_target(&target.target_id)?;
    let dashboard = menubar_dashboard(plugins, MenubarQuery { provider: None }, store)?;
    let active_after = active_target_for_provider_from_report(plugin.id(), &dashboard.accounts);
    let accounts = dashboard.accounts.clone();
    Ok(MenubarRemoveReport {
        generated_at_unix: unix_now(),
        provider: command.provider,
        requested_local_id: command.local_id,
        operation: MenubarOperationResult {
            status: MenubarOperationStatus::Success,
            changed: active_before.as_ref().map(|target| &target.account_key)
                != active_after.as_ref().map(|target| &target.account_key),
            active_before,
            active_after,
            message: "Target deleted.".to_string(),
            diagnostics: Vec::new(),
        },
        dashboard,
        accounts,
    })
}

pub fn menubar_refresh(
    plugins: &[Box<dyn PlatformPlugin>],
    command: MenubarRefreshCommand,
    store: Option<&omx_core::StateStore>,
) -> Result<MenubarRefreshReport> {
    let _guard = OPERATION_LOCK
        .try_lock()
        .map_err(|_| OpenMuxError::Message("menubar operation already in progress".to_string()))?;
    let now = unix_now();
    let plugin = find_plugin(plugins, &command.provider)?;
    let skipped_reason = refresh_skip_reason(&command.provider, &command.kind, now);
    let refreshed = skipped_reason.is_none();
    let mut operation_status = if refreshed {
        MenubarOperationStatus::Success
    } else {
        MenubarOperationStatus::Skipped
    };
    let mut operation_message = skipped_reason
        .as_ref()
        .map(|reason| format!("Refresh skipped: {reason}."))
        .unwrap_or_else(|| "Provider refreshed.".to_string());
    let mut operation_diagnostics = Vec::new();
    if refreshed {
        let result = plugin.refresh_accounts();
        record_refresh_result(&command.provider, now, result.is_ok());
        if let Err(err) = result {
            operation_status = MenubarOperationStatus::Failed;
            operation_message = "Refresh failed; showing last known data.".to_string();
            operation_diagnostics.push(MenubarDiagnostic {
                code: "refresh_failed".to_string(),
                message: sanitize_diagnostic(&err.to_string()),
            });
        }
    }
    let dashboard = menubar_dashboard(plugins, MenubarQuery { provider: None }, store)?;
    let active = active_target_for_provider_from_report(&command.provider, &dashboard.accounts);
    let refreshed = refreshed && operation_status == MenubarOperationStatus::Success;
    let operation = MenubarOperationResult {
        status: operation_status,
        changed: false,
        active_before: active.clone(),
        active_after: active,
        message: operation_message,
        diagnostics: operation_diagnostics,
    };
    let accounts = dashboard.accounts.clone();
    Ok(MenubarRefreshReport {
        generated_at_unix: unix_now(),
        provider: command.provider,
        kind: command.kind,
        operation,
        dashboard,
        refreshed,
        skipped_reason,
        accounts,
    })
}

fn resolve_menubar_target(
    provider: &str,
    catalog: &TargetCatalog,
    command: &MenubarSwitchCommand,
) -> Result<TargetResolution> {
    let matched_account = catalog
        .accounts
        .iter()
        .find(|status| status.account.local_id == command.local_id);
    let matched_profile = catalog
        .profiles
        .iter()
        .find(|profile| profile.local_id == command.local_id);

    match command.target_kind {
        Some(MenubarTargetKind::Account) => matched_account
            .map(|status| TargetResolution {
                kind: TargetKind::Account,
                target_id: status.account.local_id.clone(),
            })
            .ok_or_else(|| missing_target(provider, &command.local_id, "account")),
        Some(MenubarTargetKind::Profile) => matched_profile
            .map(|profile| TargetResolution {
                kind: TargetKind::Profile,
                target_id: profile.local_id.clone(),
            })
            .ok_or_else(|| missing_target(provider, &command.local_id, "profile")),
        None => match (matched_account, matched_profile) {
            (Some(status), None) => Ok(TargetResolution {
                kind: TargetKind::Account,
                target_id: status.account.local_id.clone(),
            }),
            (None, Some(profile)) => Ok(TargetResolution {
                kind: TargetKind::Profile,
                target_id: profile.local_id.clone(),
            }),
            (Some(_), Some(_)) => Err(OpenMuxError::Message(format!(
                "`{}` is ambiguous for `{provider}`: matched account and profile",
                command.local_id
            ))),
            (None, None) => Err(OpenMuxError::Message(format!(
                "`{}` did not match any account or profile for `{provider}`",
                command.local_id
            ))),
        },
    }
}

fn missing_target(provider: &str, local_id: &str, kind: &str) -> OpenMuxError {
    OpenMuxError::Message(format!(
        "`{local_id}` did not match any {kind} for `{provider}`"
    ))
}

fn active_target(provider: &str, catalog: &TargetCatalog) -> Option<MenubarActiveTarget> {
    catalog
        .accounts
        .iter()
        .find(|status| status.active)
        .map(|status| MenubarActiveTarget {
            provider: provider.to_string(),
            target_kind: MenubarTargetKind::Account,
            local_id: status.account.local_id.clone(),
            account_key: target_key(
                provider,
                MenubarTargetKind::Account,
                &status.account.local_id,
            ),
            display_label: status
                .account
                .alias
                .clone()
                .or_else(|| status.account_label.clone())
                .unwrap_or_else(|| status.account.local_id.clone()),
        })
        .or_else(|| {
            catalog
                .profiles
                .iter()
                .find(|profile| profile.active)
                .map(|profile| MenubarActiveTarget {
                    provider: provider.to_string(),
                    target_kind: MenubarTargetKind::Profile,
                    local_id: profile.local_id.clone(),
                    account_key: target_key(
                        provider,
                        MenubarTargetKind::Profile,
                        &profile.local_id,
                    ),
                    display_label: profile.name.clone(),
                })
        })
}

fn active_target_for_provider_from_report(
    provider: &str,
    report: &MenubarAccountsReport,
) -> Option<MenubarActiveTarget> {
    report
        .accounts
        .iter()
        .find(|account| account.provider == provider && account.active)
        .map(|account| MenubarActiveTarget {
            provider: account.provider.clone(),
            target_kind: MenubarTargetKind::Account,
            local_id: account.local_id.clone(),
            account_key: account.account_key.clone(),
            display_label: account.display_label.clone(),
        })
        .or_else(|| {
            report
                .profiles
                .iter()
                .find(|profile| profile.provider == provider && profile.active)
                .map(|profile| MenubarActiveTarget {
                    provider: profile.provider.clone(),
                    target_kind: MenubarTargetKind::Profile,
                    local_id: profile.local_id.clone(),
                    account_key: profile.account_key.clone(),
                    display_label: profile.display_label.clone(),
                })
        })
}

pub fn menubar_dashboard(
    plugins: &[Box<dyn PlatformPlugin>],
    query: MenubarQuery,
    store: Option<&omx_core::StateStore>,
) -> Result<MenubarDashboardReport> {
    let accounts = menubar_accounts(plugins, query)?;
    let active = accounts
        .accounts
        .iter()
        .find(|account| account.active)
        .cloned();
    Ok(MenubarDashboardReport {
        generated_at_unix: unix_now(),
        active,
        usage: menubar_today_usage(store)?,
        provider_usage: menubar_provider_usage(store, &accounts.providers)?,
        accounts,
    })
}

pub fn menubar_today_usage(store: Option<&omx_core::StateStore>) -> Result<MenubarUsageSummary> {
    menubar_usage_for_client(store, None)
}

fn menubar_provider_usage(
    store: Option<&omx_core::StateStore>,
    providers: &[String],
) -> Result<Vec<MenubarProviderUsageSummary>> {
    providers
        .iter()
        .map(|provider| {
            Ok(MenubarProviderUsageSummary {
                provider: provider.clone(),
                usage: menubar_usage_for_client(store, Some(provider))?,
            })
        })
        .collect()
}

fn menubar_usage_for_client(
    store: Option<&omx_core::StateStore>,
    client: Option<&str>,
) -> Result<MenubarUsageSummary> {
    let generated_at_unix = unix_now();
    let Some(store) = store else {
        return Ok(empty_usage(generated_at_unix, "unavailable"));
    };
    let (since_unix, until_unix) = today_window()?;
    let summaries = store.usage_summaries_by(UsageSummaryQuery {
        client: client.map(str::to_string),
        since_unix: Some(since_unix),
        until_unix: Some(until_unix),
        ..UsageSummaryQuery::default()
    })?;
    let models = store.usage_summaries_by(UsageSummaryQuery {
        client: client.map(str::to_string),
        since_unix: Some(since_unix),
        until_unix: Some(until_unix),
        group_by_model: true,
        ..UsageSummaryQuery::default()
    })?;
    let mut total = UsageSummary::empty("all");
    for summary in &summaries {
        total.add(summary);
    }
    let top_client = summaries
        .iter()
        .max_by_key(|summary| summary.normalized_total_tokens)
        .map(|summary| summary.client.clone());
    let top_model = models
        .iter()
        .max_by_key(|summary| summary.normalized_total_tokens)
        .and_then(|summary| summary.model.clone());
    let available_clients = summaries
        .iter()
        .map(|summary| summary.client.clone())
        .collect();
    Ok(MenubarUsageSummary {
        period: UsagePeriod::Today,
        total_tokens: total.normalized_total_tokens,
        top_client,
        top_model,
        model_breakdown: usage_model_breakdown(&models),
        cost_status: total.cost_status,
        estimated_cost_usd: total.estimated_cost_usd.map(|value| format!("{value:.4}")),
        freshness: MenubarFreshness {
            generated_at_unix,
            stale: true,
        },
        coverage: MenubarCoverage {
            status: if total.event_count == 0 {
                "empty".to_string()
            } else {
                "complete".to_string()
            },
            requested_clients: Vec::new(),
            available_clients,
            missing_clients: Vec::new(),
        },
    })
}

fn usage_model_breakdown(models: &[UsageSummary]) -> Vec<MenubarUsageModelBreakdown> {
    let mut breakdown = models
        .iter()
        .map(|summary| MenubarUsageModelBreakdown {
            model: summary
                .model
                .clone()
                .unwrap_or_else(|| "unknown".to_string()),
            total_tokens: summary.normalized_total_tokens,
        })
        .filter(|entry| entry.total_tokens > 0)
        .collect::<Vec<_>>();
    breakdown.sort_by_key(|entry| std::cmp::Reverse(entry.total_tokens));
    breakdown.truncate(6);
    breakdown
}

fn selected_plugins<'a>(
    plugins: &'a [Box<dyn PlatformPlugin>],
    provider: Option<&str>,
) -> Result<Vec<&'a dyn PlatformPlugin>> {
    if let Some(provider) = provider {
        return Ok(vec![find_plugin(plugins, provider)?]);
    }
    Ok(plugins.iter().map(|plugin| plugin.as_ref()).collect())
}

fn find_plugin<'a>(
    plugins: &'a [Box<dyn PlatformPlugin>],
    provider: &str,
) -> Result<&'a dyn PlatformPlugin> {
    plugins
        .iter()
        .map(|plugin| plugin.as_ref())
        .find(|plugin| plugin.id() == provider)
        .ok_or_else(|| OpenMuxError::Message(format!("unknown provider `{provider}`")))
}

fn refresh_skip_reason(provider: &str, kind: &RefreshKind, now: u64) -> Option<String> {
    let states = REFRESH_STATE.lock().unwrap_or_else(|err| err.into_inner());
    let state = states.get(provider)?;
    if let Some(last_error) = state.last_error_unix
        && now.saturating_sub(last_error) < REFRESH_ERROR_BACKOFF_SECONDS
    {
        return Some("error_backoff".to_string());
    }
    let floor = match kind {
        RefreshKind::Interactive => INTERACTIVE_REFRESH_FLOOR_SECONDS,
        RefreshKind::Background => BACKGROUND_REFRESH_FLOOR_SECONDS,
    };
    if let Some(last_success) = state.last_success_unix
        && now.saturating_sub(last_success) < floor
    {
        return Some("fresh_enough".to_string());
    }
    None
}

fn record_refresh_result(provider: &str, now: u64, success: bool) {
    let mut states = REFRESH_STATE.lock().unwrap_or_else(|err| err.into_inner());
    let state = states.entry(provider.to_string()).or_default();
    state.last_attempt_unix = Some(now);
    if success {
        state.last_success_unix = Some(now);
        state.last_error_unix = None;
    } else {
        state.last_error_unix = Some(now);
    }
}

fn account_from_status(status: &AccountStatus) -> MenubarAccount {
    let diagnostic = status
        .usage
        .as_ref()
        .and_then(|usage| usage.diagnostics.first())
        .map(|diagnostic| MenubarDiagnostic {
            code: diagnostic.code.clone(),
            message: sanitize_diagnostic(&diagnostic.message),
        });
    let display_label = status
        .account
        .alias
        .clone()
        .or_else(|| status.account_label.clone())
        .unwrap_or_else(|| status.account.local_id.clone());
    let secondary_label = [
        status.plan_label.as_deref(),
        status.auth_type.as_deref(),
        Some(status.availability.display.as_str()),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" · ");
    MenubarAccount {
        provider: status.account.platform.clone(),
        account_key: target_key(
            &status.account.platform,
            MenubarTargetKind::Account,
            &status.account.local_id,
        ),
        target_kind: MenubarTargetKind::Account,
        display_number: status.account.number,
        local_id: status.account.local_id.clone(),
        display_label,
        secondary_label,
        alias: status.account.alias.clone(),
        account_label: status.account_label.clone(),
        plan: status.plan_label.clone(),
        auth_type: status.auth_type.clone(),
        active: status.active,
        quota: status.usage.as_ref().map(quota_from_usage),
        status: account_state(status),
        diagnostic,
    }
}

fn profile_from_config(profile: &ConfigProfile) -> MenubarProfile {
    let secondary_label = [
        profile.provider_id.as_deref(),
        profile.model.as_deref(),
        profile.auth_type.as_deref(),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" · ");
    MenubarProfile {
        provider: profile.platform.id.clone(),
        account_key: target_key(
            &profile.platform.id,
            MenubarTargetKind::Profile,
            &profile.local_id,
        ),
        target_kind: MenubarTargetKind::Profile,
        display_number: profile.number.unwrap_or_default(),
        local_id: profile.local_id.clone(),
        display_label: profile.name.clone(),
        secondary_label: if secondary_label.is_empty() {
            profile.config_path.clone()
        } else {
            secondary_label
        },
        name: profile.name.clone(),
        active: profile.active,
        provider_id: profile.provider_id.clone(),
        base_url: profile.base_url.clone(),
        model: profile.model.clone(),
        auth_type: profile.auth_type.clone(),
        status: MenubarAccountStatus::Healthy,
        diagnostic: None,
    }
}

fn target_key(provider: &str, kind: MenubarTargetKind, local_id: &str) -> String {
    let kind = match kind {
        MenubarTargetKind::Account => "account",
        MenubarTargetKind::Profile => "profile",
    };
    format!("{provider}/{kind}/{local_id}")
}

fn account_state(status: &AccountStatus) -> MenubarAccountStatus {
    if status
        .usage
        .as_ref()
        .is_some_and(|usage| !usage.diagnostics.is_empty() && usage.refreshed_at_unix.is_some())
    {
        return MenubarAccountStatus::Stale;
    }
    match status.availability.state {
        AvailabilityState::Available => MenubarAccountStatus::Healthy,
        AvailabilityState::Limited => MenubarAccountStatus::Limited,
        AvailabilityState::Exhausted => MenubarAccountStatus::Exhausted,
        AvailabilityState::Unknown => MenubarAccountStatus::Unavailable,
    }
}

fn quota_from_usage(usage: &UsageSnapshot) -> MenubarQuota {
    let windows = usage
        .limits
        .iter()
        .map(quota_window_from_limit)
        .collect::<Vec<_>>();
    let primary_window = usage
        .limits
        .iter()
        .min_by_key(|limit| limit.remaining_percent_x100.unwrap_or(u32::MAX))
        .map(quota_window_from_limit);
    MenubarQuota {
        summary: usage.summary.display.clone(),
        refreshed_at_unix: usage.refreshed_at_unix,
        primary_window,
        windows,
    }
}

fn quota_window_from_limit(limit: &UsageLimit) -> MenubarQuotaWindow {
    MenubarQuotaWindow {
        id: limit.id.clone(),
        label: limit.label.clone(),
        window_seconds: limit.window_seconds,
        used_percent_x100: limit.used_percent_x100,
        remaining_percent_x100: limit.remaining_percent_x100,
        reset_at_unix: limit.reset_at_unix,
        exhausted: limit.exhausted,
    }
}

fn sort_accounts(accounts: &mut [MenubarAccount]) {
    accounts.sort_by_key(|account| account.display_number);
}

fn sort_profiles(profiles: &mut [MenubarProfile]) {
    profiles.sort_by_key(|profile| (profile.display_number, profile.name.clone()));
}

fn normalize_active_targets(accounts: &mut [MenubarAccount], profiles: &mut [MenubarProfile]) {
    let providers = accounts
        .iter()
        .map(|account| account.provider.clone())
        .chain(profiles.iter().map(|profile| profile.provider.clone()))
        .collect::<std::collections::HashSet<_>>();

    for provider in providers {
        if let Some(active_profile_key) = profiles
            .iter()
            .find(|profile| profile.provider == provider && profile.active)
            .map(|profile| profile.account_key.clone())
        {
            for account in accounts
                .iter_mut()
                .filter(|account| account.provider == provider)
            {
                account.active = false;
            }
            for profile in profiles
                .iter_mut()
                .filter(|profile| profile.provider == provider)
            {
                profile.active = profile.account_key == active_profile_key;
            }
            continue;
        }

        if let Some(active_account_key) = accounts
            .iter()
            .find(|account| account.provider == provider && account.active)
            .map(|account| account.account_key.clone())
        {
            for account in accounts
                .iter_mut()
                .filter(|account| account.provider == provider)
            {
                account.active = account.account_key == active_account_key;
            }
        }
    }
}

fn empty_usage(generated_at_unix: u64, status: &str) -> MenubarUsageSummary {
    MenubarUsageSummary {
        period: UsagePeriod::Today,
        total_tokens: 0,
        top_client: None,
        top_model: None,
        model_breakdown: Vec::new(),
        cost_status: CostStatus::Missing,
        estimated_cost_usd: None,
        freshness: MenubarFreshness {
            generated_at_unix,
            stale: true,
        },
        coverage: MenubarCoverage {
            status: status.to_string(),
            requested_clients: Vec::new(),
            available_clients: Vec::new(),
            missing_clients: Vec::new(),
        },
    }
}

fn today_window() -> Result<(i64, i64)> {
    let today = Local::now().date_naive();
    let tomorrow = today
        .succ_opt()
        .ok_or_else(|| OpenMuxError::Message("invalid local date".to_string()))?;
    Ok((
        local_date_start_unix(today)?,
        local_date_start_unix(tomorrow)?,
    ))
}

fn local_date_start_unix(date: chrono::NaiveDate) -> Result<i64> {
    Local
        .with_ymd_and_hms(date.year(), date.month(), date.day(), 0, 0, 0)
        .single()
        .map(|time| time.timestamp())
        .ok_or_else(|| OpenMuxError::Message("local date boundary is ambiguous".to_string()))
}

fn sanitize_diagnostic(message: &str) -> String {
    let lower = message.to_ascii_lowercase();
    let sensitive = [
        "access_token",
        "refresh_token",
        "api_key",
        "authorization:",
        "bearer ",
        "auth payload",
        "raw response",
        "raw log",
        "sk-",
    ];
    if sensitive.iter().any(|marker| lower.contains(marker)) {
        "[redacted sensitive diagnostic]".to_string()
    } else {
        message.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use omx_core::{
        AccountRef, Availability, ConfigProfile, DoctorReport, ImportConfigOptions, ImportedConfig,
        LoginOptions, PlatformCapabilities, PlatformInfo, PlatformInstall, PlatformPoolSummary,
        SaveOptions, SwitchReport, UsageDataQuality, UsageEvent, UsageEventSource,
        UsageTokenBreakdown,
    };
    use std::sync::{
        Arc, Mutex as StdMutex,
        atomic::{AtomicUsize, Ordering},
    };

    static TEST_OPERATION_LOCK: StdMutex<()> = StdMutex::new(());

    #[test]
    fn default_menubar_query_selects_all_providers() {
        assert_eq!(MenubarQuery::default().provider, None);
    }

    #[test]
    fn accounts_marks_single_active_and_redacts_diagnostics() {
        let plugins = vec![Box::new(FakePlugin::new(vec![
            account(1, true, None),
            account(2, false, Some("bearer token leaked")),
        ])) as Box<dyn PlatformPlugin>];

        let report = menubar_accounts(&plugins, MenubarQuery::default()).unwrap();

        assert_eq!(report.active_local_id.as_deref(), Some("codex-account-1"));
        assert!(report.accounts[0].active);
        assert_eq!(
            report.accounts[1].diagnostic.as_ref().unwrap().message,
            "[redacted sensitive diagnostic]"
        );
    }

    #[test]
    fn accounts_report_normalizes_to_one_active_target() {
        let plugins = vec![Box::new(
            FakePlugin::new(vec![account(1, true, None)]).with_profiles(vec![profile(1, true)]),
        ) as Box<dyn PlatformPlugin>];

        let report = menubar_accounts(&plugins, MenubarQuery::default()).unwrap();

        assert_eq!(
            report.active_target_key.as_deref(),
            Some("codex/profile/codex-profile-1")
        );
        assert!(!report.accounts[0].active);
        assert!(report.profiles[0].active);
    }

    #[test]
    fn accounts_report_keeps_one_active_target_per_provider() {
        let plugins = vec![
            Box::new(FakePlugin::new(vec![account(1, true, None)])) as Box<dyn PlatformPlugin>,
            Box::new(
                FakePlugin::new(vec![account_for_provider("claude", 1, true, None)])
                    .with_provider("claude"),
            ) as Box<dyn PlatformPlugin>,
        ];

        let report = menubar_accounts(&plugins, MenubarQuery::default()).unwrap();

        assert_eq!(
            report
                .accounts
                .iter()
                .filter(|account| account.active)
                .count(),
            2
        );
        assert!(
            report
                .accounts
                .iter()
                .any(|account| account.provider == "codex" && account.active)
        );
        assert!(
            report
                .accounts
                .iter()
                .any(|account| account.provider == "claude" && account.active)
        );
        assert!(report.diagnostics.is_empty());
    }

    #[test]
    fn switch_re_resolves_stable_local_id() {
        let _guard = TEST_OPERATION_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        let switched = Arc::new(StdMutex::new(None));
        let plugins = vec![Box::new(
            FakePlugin::new(vec![account(1, true, None), account(2, false, None)])
                .with_switched(switched.clone()),
        ) as Box<dyn PlatformPlugin>];

        let report = menubar_switch(&plugins, switch_command("codex-account-2"), None).unwrap();

        assert_eq!(switched.lock().unwrap().as_deref(), Some("codex-account-2"));
        assert_eq!(report.requested_local_id, "codex-account-2");
    }

    #[test]
    fn switch_rejects_removed_target_before_plugin_write() {
        let _guard = TEST_OPERATION_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        let plugins = vec![
            Box::new(FakePlugin::new(vec![account(1, true, None)])) as Box<dyn PlatformPlugin>
        ];

        let err = menubar_switch(&plugins, switch_command("codex-account-404"), None).unwrap_err();

        assert!(err.to_string().contains("did not match"));
    }

    #[test]
    fn switch_failure_does_not_mark_target_switched() {
        let _guard = TEST_OPERATION_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        let switched = Arc::new(StdMutex::new(None));
        let plugins = vec![Box::new(
            FakePlugin::new(vec![account(1, true, None), account(2, false, None)])
                .with_switched(switched.clone())
                .with_switch_error("atomic replacement failed"),
        ) as Box<dyn PlatformPlugin>];

        let err = menubar_switch(&plugins, switch_command("codex-account-2"), None).unwrap_err();

        assert!(err.to_string().contains("atomic replacement failed"));
        assert!(switched.lock().unwrap().is_none());
    }

    #[test]
    fn refresh_is_rejected_while_an_operation_is_in_progress() {
        let _test_guard = TEST_OPERATION_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        let _operation_guard = OPERATION_LOCK.lock().unwrap_or_else(|err| err.into_inner());
        let plugins = vec![
            Box::new(FakePlugin::new(vec![account(1, true, None)])) as Box<dyn PlatformPlugin>
        ];

        let err = menubar_refresh(
            &plugins,
            MenubarRefreshCommand {
                provider: "codex".to_string(),
                kind: RefreshKind::Interactive,
            },
            None,
        )
        .unwrap_err();

        assert!(err.to_string().contains("operation already in progress"));
    }

    #[test]
    fn background_refresh_respects_backend_floor() {
        let _guard = TEST_OPERATION_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        reset_refresh_state();
        let refresh_count = Arc::new(AtomicUsize::new(0));
        let plugins = vec![Box::new(
            FakePlugin::new(vec![account(1, true, None)]).with_refresh_count(refresh_count.clone()),
        ) as Box<dyn PlatformPlugin>];

        let first = menubar_refresh(
            &plugins,
            MenubarRefreshCommand {
                provider: "codex".to_string(),
                kind: RefreshKind::Background,
            },
            None,
        )
        .unwrap();
        let second = menubar_refresh(
            &plugins,
            MenubarRefreshCommand {
                provider: "codex".to_string(),
                kind: RefreshKind::Background,
            },
            None,
        )
        .unwrap();

        assert!(first.refreshed);
        assert!(!second.refreshed);
        assert_eq!(second.skipped_reason.as_deref(), Some("fresh_enough"));
        assert_eq!(refresh_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn refresh_error_backoff_skips_followup_background_request() {
        let _guard = TEST_OPERATION_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        reset_refresh_state();
        let refresh_count = Arc::new(AtomicUsize::new(0));
        let failing = vec![Box::new(
            FakePlugin::new(vec![account(1, true, None)])
                .with_refresh_error("network timeout")
                .with_refresh_count(refresh_count.clone()),
        ) as Box<dyn PlatformPlugin>];
        let working = vec![Box::new(
            FakePlugin::new(vec![account(1, true, None)]).with_refresh_count(refresh_count.clone()),
        ) as Box<dyn PlatformPlugin>];

        let failed = menubar_refresh(
            &failing,
            MenubarRefreshCommand {
                provider: "codex".to_string(),
                kind: RefreshKind::Background,
            },
            None,
        )
        .unwrap();
        let skipped = menubar_refresh(
            &working,
            MenubarRefreshCommand {
                provider: "codex".to_string(),
                kind: RefreshKind::Background,
            },
            None,
        )
        .unwrap();

        assert_eq!(failed.operation.status, MenubarOperationStatus::Failed);
        assert!(
            failed.operation.diagnostics[0]
                .message
                .contains("network timeout")
        );
        assert!(!skipped.refreshed);
        assert_eq!(skipped.skipped_reason.as_deref(), Some("error_backoff"));
        assert_eq!(refresh_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn accounts_report_keeps_safe_diagnostic_when_plugin_is_unavailable() {
        let plugins = vec![
            Box::new(FakePlugin::unavailable("access_token leaked")) as Box<dyn PlatformPlugin>
        ];

        let report = menubar_accounts(&plugins, MenubarQuery::default()).unwrap();

        assert!(report.accounts.is_empty());
        assert_eq!(report.diagnostics[0].code, "provider_unavailable");
        assert_eq!(
            report.diagnostics[0].message,
            "[redacted sensitive diagnostic]"
        );
    }

    #[test]
    fn dashboard_allows_no_active_account() {
        let plugins = vec![
            Box::new(FakePlugin::new(vec![account(1, false, None)])) as Box<dyn PlatformPlugin>
        ];

        let report = menubar_dashboard(&plugins, MenubarQuery::default(), None).unwrap();

        assert!(report.active.is_none());
        assert_eq!(report.accounts.active_local_id, None);
        assert_eq!(report.accounts.accounts.len(), 1);
    }

    #[test]
    fn stale_quota_keeps_last_known_quota() {
        let plugins = vec![
            Box::new(FakePlugin::new(vec![account(1, true, Some("timeout"))]))
                as Box<dyn PlatformPlugin>,
        ];

        let report = menubar_accounts(&plugins, MenubarQuery::default()).unwrap();
        let account = &report.accounts[0];

        assert_eq!(account.status, MenubarAccountStatus::Stale);
        assert_eq!(account.quota.as_ref().unwrap().summary, "80%");
        assert_eq!(account.diagnostic.as_ref().unwrap().code, "test");
    }

    #[test]
    fn refresh_failure_preserves_account_listing_error() {
        let _guard = TEST_OPERATION_LOCK
            .lock()
            .unwrap_or_else(|err| err.into_inner());
        reset_refresh_state();
        let plugins = vec![Box::new(
            FakePlugin::new(vec![account(1, true, None)]).with_refresh_error("network timeout"),
        ) as Box<dyn PlatformPlugin>];

        let report = menubar_refresh(
            &plugins,
            MenubarRefreshCommand {
                provider: "codex".to_string(),
                kind: RefreshKind::Interactive,
            },
            None,
        )
        .unwrap();

        assert_eq!(report.operation.status, MenubarOperationStatus::Failed);
        assert!(
            report.operation.diagnostics[0]
                .message
                .contains("network timeout")
        );
    }

    #[test]
    fn today_usage_empty_does_not_need_usage_scan() {
        let temp = tempfile::tempdir().unwrap();
        let store = omx_core::StateStore::open(temp.path()).unwrap();

        let usage = menubar_today_usage(Some(&store)).unwrap();

        assert_eq!(usage.total_tokens, 0);
        assert_eq!(usage.coverage.status, "empty");
    }

    #[test]
    fn today_usage_summarizes_top_client_and_model() {
        let temp = tempfile::tempdir().unwrap();
        let store = omx_core::StateStore::open(temp.path()).unwrap();
        store
            .ingest_usage_events(
                &[UsageEvent {
                    client: "codex".to_string(),
                    model_provider: None,
                    model: Some("gpt-5".to_string()),
                    session_id: None,
                    request_id: None,
                    project_path: None,
                    occurred_at_unix: unix_now() as i64,
                    tokens: UsageTokenBreakdown {
                        input: 2,
                        output: 3,
                        ..UsageTokenBreakdown::default()
                    },
                    provider_total_tokens: None,
                    estimated_cost_usd: None,
                    cost_status: CostStatus::Missing,
                    source: UsageEventSource {
                        kind: "test".to_string(),
                        path: None,
                        fingerprint_json: None,
                        offset: None,
                        record_id: None,
                        record_hash: None,
                        backend: "test".to_string(),
                        backend_version: "1".to_string(),
                        parser_schema_version: 1,
                    },
                    quality: UsageDataQuality::Parsed,
                    event_hash: "event-1".to_string(),
                }],
                None,
                unix_now(),
            )
            .unwrap();

        let usage = menubar_today_usage(Some(&store)).unwrap();

        assert_eq!(usage.total_tokens, 5);
        assert_eq!(usage.top_client.as_deref(), Some("codex"));
        assert_eq!(usage.top_model.as_deref(), Some("gpt-5"));
        assert_eq!(usage.model_breakdown[0].model, "gpt-5");
        assert_eq!(usage.model_breakdown[0].total_tokens, 5);
        assert_eq!(usage.coverage.status, "complete");
    }

    #[test]
    fn usage_summary_does_not_emit_account_attribution() {
        let temp = tempfile::tempdir().unwrap();
        let store = omx_core::StateStore::open(temp.path()).unwrap();

        let usage = menubar_today_usage(Some(&store)).unwrap();
        let json = serde_json::to_value(&usage).unwrap();

        assert!(json.get("account").is_none());
        assert!(json.get("account_id").is_none());
        assert!(json.get("active_local_id").is_none());
    }

    struct FakePlugin {
        provider: &'static str,
        accounts: Vec<AccountStatus>,
        profiles: Vec<ConfigProfile>,
        switched: Arc<StdMutex<Option<String>>>,
        refresh_count: Option<Arc<AtomicUsize>>,
        list_error: Option<String>,
        refresh_error: Option<String>,
        switch_error: Option<String>,
    }

    impl FakePlugin {
        fn new(accounts: Vec<AccountStatus>) -> Self {
            Self {
                provider: "codex",
                accounts,
                profiles: Vec::new(),
                switched: Arc::new(StdMutex::new(None)),
                refresh_count: None,
                list_error: None,
                refresh_error: None,
                switch_error: None,
            }
        }

        fn unavailable(message: &str) -> Self {
            Self {
                provider: "codex",
                accounts: Vec::new(),
                profiles: Vec::new(),
                switched: Arc::new(StdMutex::new(None)),
                refresh_count: None,
                list_error: Some(message.to_string()),
                refresh_error: None,
                switch_error: None,
            }
        }

        fn with_provider(mut self, provider: &'static str) -> Self {
            self.provider = provider;
            self
        }

        fn with_switched(mut self, switched: Arc<StdMutex<Option<String>>>) -> Self {
            self.switched = switched;
            self
        }

        fn with_profiles(mut self, profiles: Vec<ConfigProfile>) -> Self {
            self.profiles = profiles;
            self
        }

        fn with_refresh_error(mut self, message: &str) -> Self {
            self.refresh_error = Some(message.to_string());
            self
        }

        fn with_refresh_count(mut self, count: Arc<AtomicUsize>) -> Self {
            self.refresh_count = Some(count);
            self
        }

        fn with_switch_error(mut self, message: &str) -> Self {
            self.switch_error = Some(message.to_string());
            self
        }
    }

    impl PlatformPlugin for FakePlugin {
        fn id(&self) -> &'static str {
            self.provider
        }

        fn name(&self) -> &'static str {
            "Codex"
        }

        fn detect(&self) -> Result<PlatformInstall> {
            unimplemented!()
        }

        fn pool_summary(&self) -> Result<PlatformPoolSummary> {
            unimplemented!()
        }

        fn current(&self) -> Result<Option<AccountStatus>> {
            Ok(self.accounts.iter().find(|account| account.active).cloned())
        }

        fn list_accounts(&self) -> Result<Vec<AccountStatus>> {
            if let Some(message) = self.list_error.as_ref() {
                return Err(OpenMuxError::Message(message.clone()));
            }
            Ok(self.accounts.clone())
        }

        fn refresh_accounts(&self) -> Result<Vec<AccountStatus>> {
            if let Some(count) = self.refresh_count.as_ref() {
                count.fetch_add(1, Ordering::SeqCst);
            }
            if let Some(message) = self.refresh_error.as_ref() {
                return Err(OpenMuxError::Message(message.clone()));
            }
            self.list_accounts()
        }

        fn capabilities(&self) -> PlatformCapabilities {
            PlatformCapabilities::account_pool()
        }

        fn login(&self, _options: LoginOptions) -> Result<AccountRef> {
            unimplemented!()
        }

        fn save_current(&self, _options: SaveOptions) -> Result<AccountRef> {
            unimplemented!()
        }

        fn import_config(&self, _options: ImportConfigOptions) -> Result<ImportedConfig> {
            unimplemented!()
        }

        fn switch_to(&self, selector: &str) -> Result<SwitchReport> {
            if let Some(message) = self.switch_error.as_ref() {
                return Err(OpenMuxError::Message(message.clone()));
            }
            *self.switched.lock().unwrap() = Some(selector.to_string());
            Ok(SwitchReport {
                previous: self
                    .accounts
                    .iter()
                    .find(|account| account.active)
                    .map(|account| account.account.clone()),
                current: self
                    .accounts
                    .iter()
                    .find(|account| account.account.local_id == selector)
                    .map(|account| account.account.clone())
                    .ok_or_else(|| OpenMuxError::AccountNotFound {
                        platform: "codex".to_string(),
                        account: selector.to_string(),
                    })?,
            })
        }

        fn list_configs(&self) -> Result<Vec<ConfigProfile>> {
            Ok(self.profiles.clone())
        }

        fn set_alias(&self, _selector: &str, _alias: &str) -> Result<AccountRef> {
            unimplemented!()
        }

        fn doctor(&self) -> Result<DoctorReport> {
            unimplemented!()
        }
    }

    fn profile(number: u32, active: bool) -> ConfigProfile {
        profile_for_provider("codex", number, active)
    }

    fn profile_for_provider(provider: &str, number: u32, active: bool) -> ConfigProfile {
        ConfigProfile {
            platform: PlatformInfo {
                id: provider.to_string(),
                name: provider.to_string(),
            },
            local_id: format!("{provider}-profile-{number}"),
            name: format!("profile {number}"),
            active,
            config_path: format!("/tmp/profile-{number}.toml"),
            provider_id: Some("openai".to_string()),
            base_url: None,
            model: Some("gpt-5.5".to_string()),
            number: Some(number),
            auth_type: Some("api-key".to_string()),
        }
    }

    fn account(number: u32, active: bool, diagnostic: Option<&str>) -> AccountStatus {
        account_for_provider("codex", number, active, diagnostic)
    }

    fn account_for_provider(
        provider: &str,
        number: u32,
        active: bool,
        diagnostic: Option<&str>,
    ) -> AccountStatus {
        AccountStatus {
            account: AccountRef {
                platform: provider.to_string(),
                local_id: format!("{provider}-account-{number}"),
                number,
                alias: Some(format!("acct-{number}")),
            },
            active,
            account_label: Some(format!("account {number}")),
            plan_label: Some("Plus".to_string()),
            auth_type: Some("chatgpt".to_string()),
            expires_at_unix: None,
            availability: Availability {
                state: if diagnostic.is_some() {
                    AvailabilityState::Unknown
                } else {
                    AvailabilityState::Available
                },
                display: if diagnostic.is_some() {
                    "unknown".to_string()
                } else {
                    "80%".to_string()
                },
            },
            usage: Some(UsageSnapshot {
                source: omx_core::UsageSource::StoredSnapshot,
                refreshed_at_unix: Some(100),
                summary: Availability {
                    state: AvailabilityState::Available,
                    display: "80%".to_string(),
                },
                limits: Vec::new(),
                diagnostics: diagnostic
                    .map(|message| {
                        vec![omx_core::UsageDiagnostic {
                            code: "test".to_string(),
                            message: message.to_string(),
                        }]
                    })
                    .unwrap_or_default(),
            }),
        }
    }

    fn switch_command(local_id: &str) -> MenubarSwitchCommand {
        MenubarSwitchCommand {
            provider: "codex".to_string(),
            local_id: local_id.to_string(),
            target_kind: None,
        }
    }

    fn reset_refresh_state() {
        REFRESH_STATE
            .lock()
            .unwrap_or_else(|err| err.into_inner())
            .clear();
    }
}
