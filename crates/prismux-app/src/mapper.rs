use crate::dto::*;
use prismux_core::{
    AccountStatus, AvailabilityState, ConfigProfile, TargetCatalog, UsageLimit, UsageSnapshot,
};

pub(crate) fn active_target(provider: &str, catalog: &TargetCatalog) -> Option<ActiveTarget> {
    catalog
        .accounts
        .iter()
        .find(|status| status.active)
        .map(|status| ActiveTarget {
            provider: provider.to_string(),
            target_kind: TargetKindView::Account,
            local_id: status.account.local_id.clone(),
            account_key: target_key(provider, TargetKindView::Account, &status.account.local_id),
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
                .map(|profile| ActiveTarget {
                    provider: provider.to_string(),
                    target_kind: TargetKindView::Profile,
                    local_id: profile.local_id.clone(),
                    account_key: target_key(provider, TargetKindView::Profile, &profile.local_id),
                    display_label: profile.name.clone(),
                })
        })
}

pub(crate) fn active_target_from_parts(
    account: Option<TargetAccount>,
    profile: Option<TargetProfile>,
) -> Option<ActiveTarget> {
    account
        .map(|account| ActiveTarget {
            provider: account.provider,
            target_kind: TargetKindView::Account,
            local_id: account.local_id,
            account_key: account.account_key,
            display_label: account.display_label,
        })
        .or_else(|| {
            profile.map(|profile| ActiveTarget {
                provider: profile.provider,
                target_kind: TargetKindView::Profile,
                local_id: profile.local_id,
                account_key: profile.account_key,
                display_label: profile.display_label,
            })
        })
}

pub(crate) fn active_target_for_provider_from_report(
    provider: &str,
    report: &AccountsReport,
) -> Option<ActiveTarget> {
    report
        .accounts
        .iter()
        .find(|account| account.provider == provider && account.active)
        .map(|account| ActiveTarget {
            provider: account.provider.clone(),
            target_kind: TargetKindView::Account,
            local_id: account.local_id.clone(),
            account_key: account.account_key.clone(),
            display_label: account.display_label.clone(),
        })
        .or_else(|| {
            report
                .profiles
                .iter()
                .find(|profile| profile.provider == provider && profile.active)
                .map(|profile| ActiveTarget {
                    provider: profile.provider.clone(),
                    target_kind: TargetKindView::Profile,
                    local_id: profile.local_id.clone(),
                    account_key: profile.account_key.clone(),
                    display_label: profile.display_label.clone(),
                })
        })
}

pub(crate) fn account_from_status(status: &AccountStatus) -> TargetAccount {
    let diagnostic = status
        .usage
        .as_ref()
        .and_then(|usage| usage.diagnostics.first())
        .map(|diagnostic| Diagnostic {
            code: diagnostic.code.clone(),
            message: sanitize_diagnostic(&diagnostic.message),
            provider_id: Some(status.account.platform.clone()),
            target_id: Some(status.account.local_id.clone()),
            scope: Some("target".to_string()),
            recovery_action: recovery_action_for_code(&diagnostic.code),
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
    TargetAccount {
        provider: status.account.platform.clone(),
        account_key: target_key(
            &status.account.platform,
            TargetKindView::Account,
            &status.account.local_id,
        ),
        target_kind: TargetKindView::Account,
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
        actions: target_actions(status.active, "Use this account"),
        diagnostic,
    }
}

pub(crate) fn profile_from_config(profile: &ConfigProfile) -> TargetProfile {
    let secondary_label = [
        profile.provider_id.as_deref(),
        profile.model.as_deref(),
        profile.auth_type.as_deref(),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" · ");
    TargetProfile {
        provider: profile.platform.id.clone(),
        account_key: target_key(
            &profile.platform.id,
            TargetKindView::Profile,
            &profile.local_id,
        ),
        target_kind: TargetKindView::Profile,
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
        status: TargetStatus::Healthy,
        actions: target_actions(profile.active, "Use this profile"),
        diagnostic: None,
    }
}

fn target_actions(active: bool, label: &str) -> TargetActions {
    TargetActions {
        can_activate: !active,
        can_remove: true,
        primary_label: if active {
            "Current".to_string()
        } else {
            label.to_string()
        },
        disabled_reason: active.then(|| "already_active".to_string()),
    }
}

fn target_key(provider: &str, kind: TargetKindView, local_id: &str) -> String {
    let kind = match kind {
        TargetKindView::Account => "account",
        TargetKindView::Profile => "profile",
    };
    format!("{provider}/{kind}/{local_id}")
}

fn account_state(status: &AccountStatus) -> TargetStatus {
    if status
        .usage
        .as_ref()
        .is_some_and(|usage| !usage.diagnostics.is_empty() && usage.refreshed_at_unix.is_some())
    {
        return TargetStatus::Stale;
    }
    match status.availability.state {
        AvailabilityState::Available => TargetStatus::Healthy,
        AvailabilityState::Limited => TargetStatus::Limited,
        AvailabilityState::Exhausted => TargetStatus::Exhausted,
        AvailabilityState::Unknown => TargetStatus::Unavailable,
    }
}

fn quota_from_usage(usage: &UsageSnapshot) -> QuotaView {
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
    QuotaView {
        summary: usage.summary.display.clone(),
        refreshed_at_unix: usage.refreshed_at_unix,
        primary_window,
        windows,
        reset_credits: usage
            .reset_credits
            .as_ref()
            .map(|credits| ResetCreditsView {
                available_count: credits.available_count,
                credits: credits
                    .credits
                    .iter()
                    .map(|credit| ResetCreditView {
                        status: credit.status.clone(),
                        reset_type: credit.reset_type.clone(),
                        granted_at_unix: credit.granted_at_unix,
                        expires_at_unix: credit.expires_at_unix,
                    })
                    .collect(),
            }),
    }
}

fn quota_window_from_limit(limit: &UsageLimit) -> QuotaWindow {
    QuotaWindow {
        id: limit.id.clone(),
        label: limit.label.clone(),
        window_seconds: limit.window_seconds,
        used_percent_x100: limit.used_percent_x100,
        remaining_percent_x100: limit.remaining_percent_x100,
        reset_at_unix: limit.reset_at_unix,
        exhausted: limit.exhausted,
    }
}

pub(crate) fn sort_accounts(accounts: &mut [TargetAccount]) {
    accounts.sort_by_key(|account| account.display_number);
}

pub(crate) fn sort_profiles(profiles: &mut [TargetProfile]) {
    profiles.sort_by_key(|profile| (profile.display_number, profile.name.clone()));
}

pub(crate) fn normalize_active_targets(
    accounts: &mut [TargetAccount],
    profiles: &mut [TargetProfile],
) {
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

pub(crate) fn sanitize_diagnostic(message: &str) -> String {
    crate::diagnostics::redaction::redact(message)
}

pub(crate) fn recovery_action_for_code(code: &str) -> Option<String> {
    match code {
        "managed_runtime_unavailable" | "managed_runtime_auth" => {
            Some("Run `prismux doctor codex`, then `prismux save codex --alias recovery` if the active account is valid.".to_string())
        }
        "auth" | "schema" => Some("Run `prismux doctor codex` and refresh this provider again.".to_string()),
        _ => None,
    }
}
