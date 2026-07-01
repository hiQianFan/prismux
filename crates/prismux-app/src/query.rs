use crate::compatibility::{CONTROL_PLANE_SCHEMA_VERSION, STATE_SCHEMA_VERSION};
use crate::dto::*;
use crate::mapper;
use prismux_core::{
    AccountStatus, ConfigProfile, PlatformPlugin, PrismuxError, RemoveReport, Result,
    TargetCatalog, TargetResolution, UseReport, storage::unix_now,
};

/// Raw control-plane facts collected from providers before any surface DTO
/// mapping. Keeping the core `AccountStatus` / `ConfigProfile` lists lets the
/// neutral quota fact fold run over domain types (design Decision 2) instead of
/// the mapped `TargetAccount` projection.
struct CollectedTargets {
    providers: Vec<String>,
    statuses: Vec<AccountStatus>,
    profiles: Vec<ConfigProfile>,
    diagnostics: Vec<Diagnostic>,
}

fn collect_targets(
    plugins: &[Box<dyn PlatformPlugin>],
    query: &DashboardQuery,
) -> Result<CollectedTargets> {
    let mut diagnostics = Vec::new();
    let mut statuses = Vec::new();
    let mut profiles = Vec::new();
    let mut providers = Vec::new();
    for plugin in selected_plugins(plugins, query.provider.as_deref())? {
        providers.push(plugin.id().to_string());
        match plugin.list_accounts() {
            Ok(found) => statuses.extend(found),
            Err(err) => diagnostics.push(Diagnostic {
                code: "provider_unavailable".to_string(),
                message: mapper::sanitize_diagnostic(&err.to_string()),
                provider_id: Some(plugin.id().to_string()),
                target_id: None,
                scope: Some("provider".to_string()),
                recovery_action: None,
            }),
        }
        match plugin.list_configs() {
            Ok(found) => profiles.extend(found),
            Err(err) => diagnostics.push(Diagnostic {
                code: "profiles_unavailable".to_string(),
                message: mapper::sanitize_diagnostic(&err.to_string()),
                provider_id: Some(plugin.id().to_string()),
                target_id: None,
                scope: Some("provider".to_string()),
                recovery_action: None,
            }),
        }
    }
    Ok(CollectedTargets {
        providers,
        statuses,
        profiles,
        diagnostics,
    })
}

impl CollectedTargets {
    /// Map collected core facts into the surface `AccountsReport`: build the
    /// DTOs, sort, raise the multi-active diagnostic, and normalize active
    /// targets. This is presentation projection, kept separate from the neutral
    /// fact fold that runs over `self.statuses`.
    fn into_report(self) -> AccountsReport {
        let mut accounts = self
            .statuses
            .iter()
            .map(mapper::account_from_status)
            .collect::<Vec<_>>();
        let mut profiles = self
            .profiles
            .iter()
            .map(mapper::profile_from_config)
            .collect::<Vec<_>>();
        mapper::sort_accounts(&mut accounts);
        mapper::sort_profiles(&mut profiles);
        let mut diagnostics = self.diagnostics;
        for provider in &self.providers {
            let active_accounts = accounts
                .iter()
                .filter(|account| account.provider == *provider && account.active)
                .count();
            let active_profiles = profiles
                .iter()
                .filter(|profile| profile.provider == *provider && profile.active)
                .count();
            if active_accounts + active_profiles > 1 {
                diagnostics.push(Diagnostic {
                    code: "multiple_active_targets".to_string(),
                    message: format!(
                        "`{provider}` reported multiple active targets; using one active target"
                    ),
                    provider_id: Some(provider.clone()),
                    target_id: None,
                    scope: Some("provider".to_string()),
                    recovery_action: Some(format!("Run `prismux doctor {provider}`.")),
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
            .map(|_| TargetKindView::Account)
            .or_else(|| active_profile.map(|_| TargetKindView::Profile));
        let system_active_target =
            mapper::active_target_from_parts(active_account.cloned(), active_profile.cloned());
        AccountsReport {
            control_plane_schema_version: CONTROL_PLANE_SCHEMA_VERSION,
            state_schema_version: STATE_SCHEMA_VERSION,
            generated_at_unix: unix_now(),
            providers: self.providers,
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
        }
    }
}

pub fn menubar_accounts(
    plugins: &[Box<dyn PlatformPlugin>],
    query: DashboardQuery,
) -> Result<AccountsReport> {
    Ok(collect_targets(plugins, &query)?.into_report())
}

pub fn dashboard_view(
    plugins: &[Box<dyn PlatformPlugin>],
    query: DashboardQuery,
    store: Option<&prismux_core::StateStore>,
) -> Result<DashboardReport> {
    menubar_dashboard(plugins, query, store)
}

pub fn provider_view(
    plugins: &[Box<dyn PlatformPlugin>],
    query: DashboardQuery,
    store: Option<&prismux_core::StateStore>,
) -> Result<DashboardReport> {
    if query.provider.is_none() {
        return Err(PrismuxError::Message(
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
    query: DashboardQuery,
    _store: Option<&prismux_core::StateStore>,
) -> Result<DashboardReport> {
    let collected = collect_targets(plugins, &query)?;
    let statuses = collected.statuses.clone();
    let accounts = collected.into_report();
    let active = accounts
        .accounts
        .iter()
        .find(|account| account.active)
        .cloned();
    let provider_views = provider_views(&accounts, &statuses);
    let aggregate = DashboardAggregateView {
        quota_health: quota_health_rollup(
            &statuses.iter().collect::<Vec<_>>(),
            &accounts.accounts.iter().collect::<Vec<_>>(),
            &accounts.profiles.iter().collect::<Vec<_>>(),
        ),
        provider_aggregates: provider_views
            .iter()
            .map(|view| view.aggregate.clone())
            .collect(),
        diagnostics: accounts.diagnostics.clone(),
    };
    Ok(DashboardReport {
        control_plane_schema_version: CONTROL_PLANE_SCHEMA_VERSION,
        state_schema_version: STATE_SCHEMA_VERSION,
        generated_at_unix: unix_now(),
        active,
        provider_views,
        aggregate,
        accounts,
    })
}

fn provider_views(accounts: &AccountsReport, statuses: &[AccountStatus]) -> Vec<ProviderView> {
    let groups = group_targets_by_provider(&accounts.accounts, &accounts.profiles);
    accounts
        .providers
        .iter()
        .map(|provider| {
            let provider_accounts = groups
                .get(provider)
                .map(|group| group.accounts.clone())
                .unwrap_or_default();
            let provider_profiles = groups
                .get(provider)
                .map(|group| group.profiles.clone())
                .unwrap_or_default();
            let provider_statuses = statuses
                .iter()
                .filter(|status| status.account.platform == *provider)
                .collect::<Vec<_>>();
            let diagnostics = accounts
                .diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.provider_id.as_deref() == Some(provider.as_str()))
                .cloned()
                .collect::<Vec<_>>();
            let target_count = provider_accounts.len() + provider_profiles.len();
            let status = provider_status(&provider_accounts, &provider_profiles);
            let status_text = provider_status_text(target_count, &status).to_string();
            let status_tone = provider_status_tone(target_count, &status);
            let aggregate = ProviderAggregateView {
                provider_id: provider.clone(),
                provider_display_label: provider_display_label(provider),
                account_count: provider_accounts.len() as u32,
                profile_count: provider_profiles.len() as u32,
                target_count: target_count as u32,
                active_target: active_target_from_group(
                    provider,
                    &provider_accounts,
                    &provider_profiles,
                ),
                quota_health: quota_health_rollup(
                    &provider_statuses,
                    &provider_accounts,
                    &provider_profiles,
                ),
                status: status.clone(),
                status_tone: status_tone.clone(),
                status_text: status_text.clone(),
                diagnostics: diagnostics.clone(),
            };
            ProviderView {
                provider: provider.clone(),
                display_label: aggregate.provider_display_label.clone(),
                status,
                status_text,
                status_tone,
                target_count,
                aggregate,
                diagnostics,
            }
        })
        .collect()
}

#[derive(Default)]
struct ProviderTargetGroup<'a> {
    accounts: Vec<&'a TargetAccount>,
    profiles: Vec<&'a TargetProfile>,
}

fn group_targets_by_provider<'a>(
    accounts: &'a [TargetAccount],
    profiles: &'a [TargetProfile],
) -> std::collections::BTreeMap<String, ProviderTargetGroup<'a>> {
    let mut groups = std::collections::BTreeMap::<String, ProviderTargetGroup<'a>>::new();
    for account in accounts {
        groups
            .entry(account.provider.clone())
            .or_default()
            .accounts
            .push(account);
    }
    for profile in profiles {
        groups
            .entry(profile.provider.clone())
            .or_default()
            .profiles
            .push(profile);
    }
    groups
}

fn active_target_from_group(
    provider: &str,
    accounts: &[&TargetAccount],
    profiles: &[&TargetProfile],
) -> Option<ActiveTarget> {
    accounts
        .iter()
        .find(|account| account.active)
        .map(|account| active_target_from_account(account))
        .or_else(|| {
            profiles
                .iter()
                .find(|profile| profile.active)
                .map(|profile| active_target_from_profile(profile))
        })
        .map(|mut target| {
            target.provider = provider.to_string();
            target
        })
}

fn active_target_from_account(account: &TargetAccount) -> ActiveTarget {
    ActiveTarget {
        provider: account.provider.clone(),
        target_kind: TargetKindView::Account,
        local_id: account.local_id.clone(),
        account_key: account.account_key.clone(),
        display_label: account.display_label.clone(),
    }
}

fn active_target_from_profile(profile: &TargetProfile) -> ActiveTarget {
    ActiveTarget {
        provider: profile.provider.clone(),
        target_kind: TargetKindView::Profile,
        local_id: profile.local_id.clone(),
        account_key: profile.account_key.clone(),
        display_label: profile.display_label.clone(),
    }
}

/// Remaining percent of an account's primary window, derived from core facts
/// the same way the surface mapper picks it: the limit with the least remaining.
fn primary_remaining_percent_x100(status: &AccountStatus) -> Option<u32> {
    status
        .usage
        .as_ref()?
        .limits
        .iter()
        .min_by_key(|limit| limit.remaining_percent_x100.unwrap_or(u32::MAX))
        .and_then(|limit| limit.remaining_percent_x100)
}

/// Neutral quota fact fold. Per design Decision 2 this consumes core domain
/// facts (`AccountStatus`) only — never a surface DTO — and carries no product
/// policy (no health buckets, status text, tone, or best alternative).
fn quota_facts_rollup(accounts: &[&AccountStatus]) -> QuotaFactsRollup {
    let mut facts = QuotaFactsRollup {
        account_count: accounts.len() as u32,
        ..QuotaFactsRollup::default()
    };
    let mut remaining_total = 0_u32;
    for account in accounts {
        let Some(usage) = &account.usage else {
            continue;
        };
        facts.reset_credit_total += usage
            .reset_credits
            .as_ref()
            .map(|credits| credits.available_count)
            .unwrap_or_default();
        for limit in &usage.limits {
            if let Some(reset_at) = limit.reset_at_unix {
                facts.soonest_reset_at_unix = Some(
                    facts
                        .soonest_reset_at_unix
                        .map_or(reset_at, |current| current.min(reset_at)),
                );
            }
        }
        let Some(remaining) = primary_remaining_percent_x100(account) else {
            continue;
        };
        facts.reporting_count += 1;
        remaining_total += remaining;
        facts.min_remaining_percent_x100 = Some(
            facts
                .min_remaining_percent_x100
                .map_or(remaining, |current| current.min(remaining)),
        );
        facts.max_remaining_percent_x100 = Some(
            facts
                .max_remaining_percent_x100
                .map_or(remaining, |current| current.max(remaining)),
        );
    }
    facts.avg_remaining_percent_x100 = remaining_total.checked_div(facts.reporting_count);
    facts
}

/// Control-plane quota health policy. `facts` is the neutral fold over core
/// `AccountStatus`; the surface `accounts`/`profiles` projections drive only
/// product policy (health buckets, worst target, best alternative, status/tone).
fn quota_health_rollup(
    statuses: &[&AccountStatus],
    accounts: &[&TargetAccount],
    profiles: &[&TargetProfile],
) -> QuotaHealthRollup {
    let facts = quota_facts_rollup(statuses);
    let mut healthy_count = 0_u32;
    let mut low_count = 0_u32;
    let mut exhausted_count = 0_u32;
    let mut worst_account: Option<&TargetAccount> = None;

    for account in accounts {
        let remaining = account
            .quota
            .as_ref()
            .and_then(|quota| quota.primary_window.as_ref())
            .and_then(|window| window.remaining_percent_x100);
        if let Some(remaining) = remaining {
            if remaining == 0 || account.status == TargetStatus::Exhausted {
                exhausted_count += 1;
            } else if remaining < 2_000 {
                low_count += 1;
            } else {
                healthy_count += 1;
            }
            if worst_account.is_none_or(|current| {
                current
                    .quota
                    .as_ref()
                    .and_then(|quota| quota.primary_window.as_ref())
                    .and_then(|window| window.remaining_percent_x100)
                    .unwrap_or(u32::MAX)
                    > remaining
            }) {
                worst_account = Some(account);
            }
        }
    }

    let status = if facts.account_count == 0 && profiles.is_empty() {
        TargetStatus::Unavailable
    } else if exhausted_count > 0 {
        TargetStatus::Exhausted
    } else if low_count > 0 {
        TargetStatus::Limited
    } else {
        TargetStatus::Healthy
    };
    let status_tone = match status {
        TargetStatus::Healthy => ViewTone::Success,
        TargetStatus::Limited | TargetStatus::Stale => ViewTone::Warning,
        TargetStatus::Exhausted | TargetStatus::Unavailable => ViewTone::Danger,
    };

    QuotaHealthRollup {
        facts,
        healthy_count,
        low_count,
        exhausted_count,
        worst_target: worst_account.map(active_target_from_account),
        best_alternative: best_alternative(accounts, profiles),
        window_averages: window_averages(statuses),
        status,
        status_tone,
    }
}

/// Average remaining percent per window class (5h / 7d) across reporting
/// accounts. Classifies each core `UsageLimit` by its id/label text the same way
/// the frontend window picker does, then averages within each class. None when
/// no account reported that class.
fn window_averages(statuses: &[&AccountStatus]) -> WindowAverages {
    let mut short_total = 0_u32;
    let mut short_count = 0_u32;
    let mut weekly_total = 0_u32;
    let mut weekly_count = 0_u32;
    for status in statuses {
        let Some(usage) = &status.usage else { continue };
        for limit in &usage.limits {
            let Some(remaining) = limit.remaining_percent_x100 else {
                continue;
            };
            let text = format!("{} {}", limit.id, limit.label).to_lowercase();
            if text.contains("5h") || text.contains("session") || text.contains("short") {
                short_total += remaining;
                short_count += 1;
            } else if text.contains("7d") || text.contains("week") {
                weekly_total += remaining;
                weekly_count += 1;
            }
        }
    }
    WindowAverages {
        short_remaining_percent_x100: short_total.checked_div(short_count),
        weekly_remaining_percent_x100: weekly_total.checked_div(weekly_count),
    }
}

fn best_alternative(
    accounts: &[&TargetAccount],
    profiles: &[&TargetProfile],
) -> Option<TargetRecommendation> {
    let account = accounts
        .iter()
        .filter(|account| account.actions.can_activate)
        .max_by_key(|account| {
            let remaining = account
                .quota
                .as_ref()
                .and_then(|quota| quota.primary_window.as_ref())
                .and_then(|window| window.remaining_percent_x100)
                .unwrap_or_default();
            (remaining, std::cmp::Reverse(account.display_number))
        })
        .map(|account| TargetRecommendation {
            target: active_target_from_account(account),
            reason: "higher_remaining_quota".to_string(),
            action: account.actions.primary_label.clone(),
        });
    account.or_else(|| {
        profiles
            .iter()
            .find(|profile| profile.actions.can_activate)
            .map(|profile| TargetRecommendation {
                target: active_target_from_profile(profile),
                reason: "activatable_profile".to_string(),
                action: profile.actions.primary_label.clone(),
            })
    })
}

fn provider_status(accounts: &[&TargetAccount], profiles: &[&TargetProfile]) -> TargetStatus {
    if accounts.is_empty() && profiles.is_empty() {
        return TargetStatus::Unavailable;
    }
    if accounts.iter().any(|account| {
        matches!(
            account.status,
            TargetStatus::Exhausted | TargetStatus::Unavailable
        )
    }) || profiles.iter().any(|profile| {
        matches!(
            profile.status,
            TargetStatus::Exhausted | TargetStatus::Unavailable
        )
    }) {
        return TargetStatus::Unavailable;
    }
    if accounts
        .iter()
        .any(|account| account.status != TargetStatus::Healthy || account.diagnostic.is_some())
        || profiles
            .iter()
            .any(|profile| profile.status != TargetStatus::Healthy || profile.diagnostic.is_some())
    {
        return TargetStatus::Stale;
    }
    TargetStatus::Healthy
}

fn provider_status_text(target_count: usize, status: &TargetStatus) -> &'static str {
    if target_count == 0 {
        return "Planned";
    }
    match status {
        TargetStatus::Healthy => "OK",
        TargetStatus::Stale | TargetStatus::Limited => "Stale",
        TargetStatus::Exhausted | TargetStatus::Unavailable => "Alert",
    }
}

fn provider_status_tone(target_count: usize, status: &TargetStatus) -> ViewTone {
    if target_count == 0 {
        return ViewTone::Neutral;
    }
    match status {
        TargetStatus::Healthy => ViewTone::Success,
        TargetStatus::Limited | TargetStatus::Stale => ViewTone::Warning,
        TargetStatus::Exhausted | TargetStatus::Unavailable => ViewTone::Danger,
    }
}

fn provider_display_label(provider: &str) -> String {
    let mut chars = provider.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => provider.to_string(),
    }
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
        .ok_or_else(|| PrismuxError::Message(format!("unknown provider `{provider}`")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use prismux_core::{
        AccountRef, Availability, AvailabilityState, UsageLimit, UsageLimitKind, UsageLimitScope,
        UsageResetCredits, UsageSnapshot, UsageSource,
    };

    /// Core-fact builder for the neutral quota fold (design Decision 2: the fold
    /// consumes `AccountStatus`, never a surface DTO).
    fn status(number: u32, remaining: Option<u32>, credit: u32) -> AccountStatus {
        AccountStatus {
            account: AccountRef {
                platform: "codex".to_string(),
                local_id: format!("account-{number}"),
                number,
                alias: None,
            },
            active: false,
            account_label: None,
            plan_label: None,
            auth_type: None,
            expires_at_unix: None,
            availability: Availability {
                state: AvailabilityState::Available,
                display: "available".to_string(),
            },
            usage: remaining.map(|remaining| UsageSnapshot {
                source: UsageSource::RemoteApi,
                refreshed_at_unix: Some(1),
                summary: Availability {
                    state: AvailabilityState::Available,
                    display: format!("{}%", remaining / 100),
                },
                limits: vec![UsageLimit {
                    id: "weekly".to_string(),
                    label: "Weekly".to_string(),
                    scope: UsageLimitScope::Account,
                    kind: UsageLimitKind::RollingWindow,
                    window_seconds: Some(604_800),
                    used_percent_x100: Some(10_000_u32.saturating_sub(remaining)),
                    remaining_percent_x100: Some(remaining),
                    reset_at_unix: Some(100 + number as i64),
                    exhausted: Some(remaining == 0),
                    raw_provider_key: None,
                }],
                reset_credits: Some(UsageResetCredits {
                    available_count: credit,
                    credits: Vec::new(),
                }),
                diagnostics: Vec::new(),
            }),
        }
    }

    fn account(
        number: u32,
        remaining: Option<u32>,
        credit: u32,
        can_activate: bool,
    ) -> TargetAccount {
        TargetAccount {
            provider: "codex".to_string(),
            account_key: format!("codex/account/{number}"),
            target_kind: TargetKindView::Account,
            display_number: number,
            local_id: format!("account-{number}"),
            display_label: format!("Account {number}"),
            secondary_label: String::new(),
            alias: None,
            account_label: None,
            plan: None,
            auth_type: None,
            active: !can_activate,
            quota: remaining.map(|remaining| QuotaView {
                summary: format!("{}%", remaining / 100),
                refreshed_at_unix: Some(1),
                primary_window: Some(QuotaWindow {
                    id: "weekly".to_string(),
                    label: "Weekly".to_string(),
                    window_seconds: Some(604_800),
                    used_percent_x100: Some(10_000_u32.saturating_sub(remaining)),
                    remaining_percent_x100: Some(remaining),
                    reset_at_unix: Some(100 + number as i64),
                    exhausted: Some(remaining == 0),
                }),
                windows: vec![QuotaWindow {
                    id: "weekly".to_string(),
                    label: "Weekly".to_string(),
                    window_seconds: Some(604_800),
                    used_percent_x100: Some(10_000_u32.saturating_sub(remaining)),
                    remaining_percent_x100: Some(remaining),
                    reset_at_unix: Some(100 + number as i64),
                    exhausted: Some(remaining == 0),
                }],
                reset_credits: Some(ResetCreditsView {
                    available_count: credit,
                    credits: Vec::new(),
                }),
            }),
            status: TargetStatus::Healthy,
            actions: TargetActions {
                can_activate,
                can_remove: true,
                primary_label: "Use this account".to_string(),
                disabled_reason: (!can_activate).then(|| "already_active".to_string()),
            },
            diagnostic: None,
        }
    }

    #[test]
    fn quota_facts_skip_missing_quota_and_keep_reset_credit_separate() {
        let accounts = [
            status(1, Some(8_000), 2),
            status(2, Some(6_000), 1),
            status(3, None, 0),
        ];
        let refs = accounts.iter().collect::<Vec<_>>();
        let facts = quota_facts_rollup(&refs);

        assert_eq!(facts.account_count, 3);
        assert_eq!(facts.reporting_count, 2);
        assert_eq!(facts.avg_remaining_percent_x100, Some(7_000));
        assert_eq!(facts.min_remaining_percent_x100, Some(6_000));
        assert_eq!(facts.max_remaining_percent_x100, Some(8_000));
        assert_eq!(facts.reset_credit_total, 3);
    }

    #[test]
    fn quota_facts_return_none_when_nothing_reports_quota() {
        let accounts = [status(1, None, 0), status(2, None, 0)];
        let refs = accounts.iter().collect::<Vec<_>>();
        let facts = quota_facts_rollup(&refs);

        assert_eq!(facts.account_count, 2);
        assert_eq!(facts.reporting_count, 0);
        assert_eq!(facts.avg_remaining_percent_x100, None);
    }

    #[test]
    fn global_quota_average_folds_raw_reporting_accounts() {
        let accounts = [
            status(1, Some(9_000), 0),
            status(2, Some(3_000), 0),
            status(3, Some(3_000), 0),
            status(4, Some(3_000), 0),
        ];
        let refs = accounts.iter().collect::<Vec<_>>();

        assert_eq!(
            quota_facts_rollup(&refs).avg_remaining_percent_x100,
            Some(4_500)
        );
    }

    #[test]
    fn best_alternative_uses_action_eligibility_and_stable_tie_break() {
        let accounts = [
            account(1, Some(9_000), 0, false),
            account(2, Some(7_000), 0, true),
            account(3, Some(7_000), 0, true),
        ];
        let refs = accounts.iter().collect::<Vec<_>>();

        let best = best_alternative(&refs, &[]).unwrap();

        assert_eq!(best.target.local_id, "account-2");
        assert_eq!(best.reason, "higher_remaining_quota");
    }
}
