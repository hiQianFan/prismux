use crate::compatibility::{CONTROL_PLANE_SCHEMA_VERSION, STATE_SCHEMA_VERSION};
use crate::dto::*;
use crate::mapper;
use chrono::{Datelike, Duration, Local, TimeZone};
use omx_core::{
    AccountStatus, ConfigProfile, CostStatus, OpenMuxError, PlatformPlugin, RemoveReport, Result,
    TargetCatalog, TargetResolution, UsagePeriod, UsageSummary, UsageSummaryQuery, UseReport,
    storage::unix_now,
};

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
            Ok(statuses) => accounts.extend(statuses.iter().map(mapper::account_from_status)),
            Err(err) => diagnostics.push(MenubarDiagnostic {
                code: "provider_unavailable".to_string(),
                message: mapper::sanitize_diagnostic(&err.to_string()),
                recovery_action: None,
            }),
        }
        match plugin.list_configs() {
            Ok(configs) => profiles.extend(configs.iter().map(mapper::profile_from_config)),
            Err(err) => diagnostics.push(MenubarDiagnostic {
                code: "profiles_unavailable".to_string(),
                message: mapper::sanitize_diagnostic(&err.to_string()),
                recovery_action: None,
            }),
        }
    }
    mapper::sort_accounts(&mut accounts);
    mapper::sort_profiles(&mut profiles);
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
                recovery_action: Some(format!("Run `omx doctor {provider}`.")),
            });
        }
    }
    mapper::normalize_active_targets(&mut accounts, &mut profiles);
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
    let system_active_target =
        mapper::active_target_from_parts(active_account.cloned(), active_profile.cloned());
    Ok(MenubarAccountsReport {
        control_plane_schema_version: CONTROL_PLANE_SCHEMA_VERSION,
        state_schema_version: STATE_SCHEMA_VERSION,
        generated_at_unix: unix_now(),
        providers,
        accounts,
        profiles,
        active_local_id,
        active_target_key,
        active_target_kind,
        system_active_target: system_active_target.clone(),
        selected_ui_target: system_active_target.clone(),
        refresh_scope_target: system_active_target.clone(),
        observed_target: system_active_target,
        diagnostics,
    })
}

pub fn dashboard_view(
    plugins: &[Box<dyn PlatformPlugin>],
    query: MenubarQuery,
    store: Option<&omx_core::StateStore>,
) -> Result<MenubarDashboardReport> {
    menubar_dashboard(plugins, query, store)
}

pub fn provider_view(
    plugins: &[Box<dyn PlatformPlugin>],
    query: MenubarQuery,
    store: Option<&omx_core::StateStore>,
) -> Result<MenubarDashboardReport> {
    if query.provider.is_none() {
        return Err(OpenMuxError::Message(
            "provider_view requires a provider".to_string(),
        ));
    }
    menubar_dashboard(plugins, query, store)
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
        control_plane_schema_version: CONTROL_PLANE_SCHEMA_VERSION,
        state_schema_version: STATE_SCHEMA_VERSION,
        generated_at_unix: unix_now(),
        active,
        provider_views: provider_views(&accounts),
        usage: menubar_today_usage(store)?,
        provider_usage: menubar_provider_usage(store, &accounts.providers)?,
        accounts,
    })
}

fn provider_views(accounts: &MenubarAccountsReport) -> Vec<MenubarProviderView> {
    accounts
        .providers
        .iter()
        .map(|provider| {
            let provider_accounts = accounts
                .accounts
                .iter()
                .filter(|account| account.provider == *provider)
                .collect::<Vec<_>>();
            let provider_profiles = accounts
                .profiles
                .iter()
                .filter(|profile| profile.provider == *provider)
                .collect::<Vec<_>>();
            let target_count = provider_accounts.len() + provider_profiles.len();
            let status = provider_status(&provider_accounts, &provider_profiles);
            MenubarProviderView {
                provider: provider.clone(),
                display_label: provider_display_label(provider),
                status: status.clone(),
                status_text: provider_status_text(target_count, &status).to_string(),
                status_tone: provider_status_tone(target_count, &status),
                target_count,
                diagnostics: accounts
                    .diagnostics
                    .iter()
                    .filter(|diagnostic| diagnostic.message.contains(provider))
                    .cloned()
                    .collect(),
            }
        })
        .collect()
}

fn provider_status(
    accounts: &[&MenubarAccount],
    profiles: &[&MenubarProfile],
) -> MenubarAccountStatus {
    if accounts.is_empty() && profiles.is_empty() {
        return MenubarAccountStatus::Unavailable;
    }
    if accounts.iter().any(|account| {
        matches!(
            account.status,
            MenubarAccountStatus::Exhausted | MenubarAccountStatus::Unavailable
        )
    }) || profiles.iter().any(|profile| {
        matches!(
            profile.status,
            MenubarAccountStatus::Exhausted | MenubarAccountStatus::Unavailable
        )
    }) {
        return MenubarAccountStatus::Unavailable;
    }
    if accounts.iter().any(|account| {
        account.status != MenubarAccountStatus::Healthy || account.diagnostic.is_some()
    }) || profiles.iter().any(|profile| {
        profile.status != MenubarAccountStatus::Healthy || profile.diagnostic.is_some()
    }) {
        return MenubarAccountStatus::Stale;
    }
    MenubarAccountStatus::Healthy
}

fn provider_status_text(target_count: usize, status: &MenubarAccountStatus) -> &'static str {
    if target_count == 0 {
        return "Planned";
    }
    match status {
        MenubarAccountStatus::Healthy => "OK",
        MenubarAccountStatus::Stale | MenubarAccountStatus::Limited => "Stale",
        MenubarAccountStatus::Exhausted | MenubarAccountStatus::Unavailable => "Alert",
    }
}

fn provider_status_tone(target_count: usize, status: &MenubarAccountStatus) -> MenubarViewTone {
    if target_count == 0 {
        return MenubarViewTone::Neutral;
    }
    match status {
        MenubarAccountStatus::Healthy => MenubarViewTone::Success,
        MenubarAccountStatus::Limited | MenubarAccountStatus::Stale => MenubarViewTone::Warning,
        MenubarAccountStatus::Exhausted | MenubarAccountStatus::Unavailable => {
            MenubarViewTone::Danger
        }
    }
}

fn provider_display_label(provider: &str) -> String {
    let mut chars = provider.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => provider.to_string(),
    }
}

pub fn menubar_today_usage(store: Option<&omx_core::StateStore>) -> Result<MenubarUsageSummary> {
    menubar_usage_for_client(store, None, MenubarUsageChartSeriesKind::Provider)
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
                usage: menubar_usage_for_client(
                    store,
                    Some(provider),
                    MenubarUsageChartSeriesKind::Model,
                )?,
            })
        })
        .collect()
}

fn menubar_usage_for_client(
    store: Option<&omx_core::StateStore>,
    client: Option<&str>,
    series_kind: MenubarUsageChartSeriesKind,
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
    let hourly_buckets = hourly_buckets_30d(store, client)?;
    let series = usage_chart_series_30d(store, client, series_kind)?;
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
        hourly_buckets,
        series,
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
            tone: if total.event_count == 0 {
                MenubarViewTone::Warning
            } else {
                MenubarViewTone::Success
            },
            requested_clients: Vec::new(),
            available_clients,
            missing_clients: Vec::new(),
        },
    })
}

/// Aggregate token usage per local **hour** across the last 30 days (inclusive
/// of today). Hours with no usage are omitted; the frontend renders gaps as
/// empty cells and rolls hours up into days for the 7d/30d views (a day is the
/// `YYYY-MM-DD` prefix of `local_hour`). Buckets are returned in ascending
/// chronological order.
fn hourly_buckets_30d(
    store: &omx_core::StateStore,
    client: Option<&str>,
) -> Result<Vec<MenubarHourlyBucket>> {
    let summaries = usage_hourly_summaries_30d(store, client, false)?;

    let mut by_hour = std::collections::BTreeMap::<String, u64>::new();
    for summary in &summaries {
        let Some(hour) = summary.local_hour.clone() else {
            continue;
        };
        *by_hour.entry(hour).or_insert(0) += summary.normalized_total_tokens;
    }

    Ok(by_hour
        .into_iter()
        .map(|(local_hour, total_tokens)| MenubarHourlyBucket {
            local_hour,
            total_tokens,
        })
        .collect())
}

fn usage_chart_series_30d(
    store: &omx_core::StateStore,
    client: Option<&str>,
    kind: MenubarUsageChartSeriesKind,
) -> Result<Vec<MenubarUsageChartSeries>> {
    match kind {
        MenubarUsageChartSeriesKind::Provider => provider_usage_series_30d(store),
        MenubarUsageChartSeriesKind::Model => model_usage_series_30d(store, client),
    }
}

fn provider_usage_series_30d(store: &omx_core::StateStore) -> Result<Vec<MenubarUsageChartSeries>> {
    let summaries = usage_hourly_summaries_30d(store, None, false)?;
    let mut by_provider =
        std::collections::BTreeMap::<String, std::collections::BTreeMap<String, u64>>::new();
    for summary in summaries {
        let Some(hour) = summary.local_hour else {
            continue;
        };
        *by_provider
            .entry(summary.client)
            .or_default()
            .entry(hour)
            .or_insert(0) += summary.normalized_total_tokens;
    }
    Ok(by_provider
        .into_iter()
        .map(|(provider, buckets)| MenubarUsageChartSeries {
            kind: MenubarUsageChartSeriesKind::Provider,
            label: provider_display_label(&provider),
            key: provider,
            hourly_buckets: hourly_bucket_entries(buckets),
        })
        .collect())
}

fn model_usage_series_30d(
    store: &omx_core::StateStore,
    client: Option<&str>,
) -> Result<Vec<MenubarUsageChartSeries>> {
    let summaries = usage_hourly_summaries_30d(store, client, true)?;
    let mut by_model =
        std::collections::BTreeMap::<String, std::collections::BTreeMap<String, u64>>::new();
    for summary in summaries {
        let Some(hour) = summary.local_hour else {
            continue;
        };
        let model = summary.model.unwrap_or_else(|| "unknown".to_string());
        *by_model.entry(model).or_default().entry(hour).or_insert(0) +=
            summary.normalized_total_tokens;
    }
    Ok(by_model
        .into_iter()
        .map(|(model, buckets)| MenubarUsageChartSeries {
            kind: MenubarUsageChartSeriesKind::Model,
            key: model.clone(),
            label: model,
            hourly_buckets: hourly_bucket_entries(buckets),
        })
        .collect())
}

fn usage_hourly_summaries_30d(
    store: &omx_core::StateStore,
    client: Option<&str>,
    group_by_model: bool,
) -> Result<Vec<UsageSummary>> {
    let today = Local::now().date_naive();
    let window_start = today - Duration::days(29);
    let tomorrow = today
        .succ_opt()
        .ok_or_else(|| OpenMuxError::Message("invalid local date".to_string()))?;
    store.usage_summaries_by(UsageSummaryQuery {
        client: client.map(str::to_string),
        since_unix: Some(local_date_start_unix(window_start)?),
        until_unix: Some(local_date_start_unix(tomorrow)?),
        group_by_local_hour: true,
        group_by_model,
        local_day_offset_seconds: Local::now().offset().local_minus_utc(),
        ..UsageSummaryQuery::default()
    })
}

fn hourly_bucket_entries(
    buckets: std::collections::BTreeMap<String, u64>,
) -> Vec<MenubarHourlyBucket> {
    buckets
        .into_iter()
        .map(|(local_hour, total_tokens)| MenubarHourlyBucket {
            local_hour,
            total_tokens,
        })
        .collect()
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

pub(crate) fn selected_plugins<'a>(
    plugins: &'a [Box<dyn PlatformPlugin>],
    provider: Option<&str>,
) -> Result<Vec<&'a dyn PlatformPlugin>> {
    if let Some(provider) = provider {
        return Ok(vec![find_plugin(plugins, provider)?]);
    }
    Ok(plugins.iter().map(|plugin| plugin.as_ref()).collect())
}

pub(crate) fn find_plugin<'a>(
    plugins: &'a [Box<dyn PlatformPlugin>],
    provider: &str,
) -> Result<&'a dyn PlatformPlugin> {
    plugins
        .iter()
        .map(|plugin| plugin.as_ref())
        .find(|plugin| plugin.id() == provider)
        .ok_or_else(|| OpenMuxError::Message(format!("unknown provider `{provider}`")))
}

fn empty_usage(generated_at_unix: u64, status: &str) -> MenubarUsageSummary {
    MenubarUsageSummary {
        period: UsagePeriod::Today,
        total_tokens: 0,
        top_client: None,
        top_model: None,
        model_breakdown: Vec::new(),
        hourly_buckets: Vec::new(),
        series: Vec::new(),
        cost_status: CostStatus::Missing,
        estimated_cost_usd: None,
        freshness: MenubarFreshness {
            generated_at_unix,
            stale: true,
        },
        coverage: MenubarCoverage {
            status: status.to_string(),
            tone: MenubarViewTone::Warning,
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
