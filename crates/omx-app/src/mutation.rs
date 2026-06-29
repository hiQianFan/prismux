use crate::compatibility::{CONTROL_PLANE_SCHEMA_VERSION, STATE_SCHEMA_VERSION};
use crate::dto::*;
use crate::mapper::{active_target, active_target_for_provider_from_report, sanitize_diagnostic};
use crate::query::{find_plugin, menubar_dashboard, target_catalog};
use crate::runtime::{
    RefreshAdmission, begin_refresh_request, record_refresh_result, refresh_skip_reason,
    release_refresh_request,
};
use omx_core::{
    OpenMuxError, PlatformPlugin, ResetCreditOutcome, Result, TargetCatalog, TargetKind,
    TargetResolution, storage::unix_now,
};
use std::sync::Mutex;

pub(crate) static OPERATION_LOCK: Mutex<()> = Mutex::new(());

pub fn activate_target(
    plugins: &[Box<dyn PlatformPlugin>],
    command: SwitchCommand,
    store: Option<&omx_core::StateStore>,
) -> Result<SwitchReport> {
    menubar_switch(plugins, command, store)
}

pub fn refresh_provider(
    plugins: &[Box<dyn PlatformPlugin>],
    command: RefreshCommand,
    store: Option<&omx_core::StateStore>,
) -> Result<RefreshReport> {
    menubar_refresh(plugins, command, store)
}

pub fn refresh_all(
    plugins: &[Box<dyn PlatformPlugin>],
    command: RefreshCommand,
    store: Option<&omx_core::StateStore>,
) -> Result<RefreshReport> {
    menubar_refresh(plugins, command, store)
}

pub fn remove_target(
    plugins: &[Box<dyn PlatformPlugin>],
    command: RemoveCommand,
    store: Option<&omx_core::StateStore>,
) -> Result<RemoveReportView> {
    menubar_remove(plugins, command, store)
}

pub fn consume_reset_credit(
    plugins: &[Box<dyn PlatformPlugin>],
    command: ConsumeResetCreditCommand,
    store: Option<&omx_core::StateStore>,
) -> Result<ConsumeResetCreditReport> {
    menubar_consume_reset_credit(plugins, command, store)
}

pub fn menubar_switch(
    plugins: &[Box<dyn PlatformPlugin>],
    command: SwitchCommand,
    store: Option<&omx_core::StateStore>,
) -> Result<SwitchReport> {
    let _guard = OPERATION_LOCK
        .try_lock()
        .map_err(|_| OpenMuxError::Message("menubar operation already in progress".to_string()))?;
    let plugin = find_plugin(plugins, &command.provider)?;
    let before_catalog = target_catalog(plugin)?;
    let active_before = active_target(plugin.id(), &before_catalog);
    let target = resolve_menubar_target(plugin.id(), &before_catalog, &command)?;
    plugin.use_target(&target.target_id)?;
    let dashboard = menubar_dashboard(plugins, DashboardQuery::default(), store)?;
    let active_after = active_target_for_provider_from_report(plugin.id(), &dashboard.accounts);
    let changed = active_before.as_ref().map(|target| &target.account_key)
        != active_after.as_ref().map(|target| &target.account_key);
    Ok(SwitchReport {
        control_plane_schema_version: CONTROL_PLANE_SCHEMA_VERSION,
        state_schema_version: STATE_SCHEMA_VERSION,
        generated_at_unix: unix_now(),
        provider: command.provider,
        requested_local_id: command.local_id,
        operation: OperationResult {
            status: OperationStatus::Success,
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
    command: RemoveCommand,
    store: Option<&omx_core::StateStore>,
) -> Result<RemoveReportView> {
    let _guard = OPERATION_LOCK
        .try_lock()
        .map_err(|_| OpenMuxError::Message("menubar operation already in progress".to_string()))?;
    let plugin = find_plugin(plugins, &command.provider)?;
    let before_catalog = target_catalog(plugin)?;
    let active_before = active_target(plugin.id(), &before_catalog);
    let target = resolve_menubar_target(
        plugin.id(),
        &before_catalog,
        &SwitchCommand {
            provider: command.provider.clone(),
            local_id: command.local_id.clone(),
            target_kind: command.target_kind,
        },
    )?;
    plugin.remove_target(&target.target_id)?;
    let dashboard = menubar_dashboard(plugins, DashboardQuery::default(), store)?;
    let active_after = active_target_for_provider_from_report(plugin.id(), &dashboard.accounts);
    let accounts = dashboard.accounts.clone();
    Ok(RemoveReportView {
        control_plane_schema_version: CONTROL_PLANE_SCHEMA_VERSION,
        state_schema_version: STATE_SCHEMA_VERSION,
        generated_at_unix: unix_now(),
        provider: command.provider,
        requested_local_id: command.local_id,
        operation: OperationResult {
            status: OperationStatus::Success,
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

pub fn menubar_consume_reset_credit(
    plugins: &[Box<dyn PlatformPlugin>],
    command: ConsumeResetCreditCommand,
    store: Option<&omx_core::StateStore>,
) -> Result<ConsumeResetCreditReport> {
    let _guard = OPERATION_LOCK
        .try_lock()
        .map_err(|_| OpenMuxError::Message("menubar operation already in progress".to_string()))?;
    let plugin = find_plugin(plugins, &command.provider)?;
    let before_catalog = target_catalog(plugin)?;
    let active = active_target(plugin.id(), &before_catalog);
    let target = resolve_menubar_target(
        plugin.id(),
        &before_catalog,
        &SwitchCommand {
            provider: command.provider.clone(),
            local_id: command.local_id.clone(),
            target_kind: command.target_kind,
        },
    )?;
    let consume_result = plugin.consume_reset_credit(&target.target_id, &command.idempotency_key);
    let mut diagnostics = Vec::new();
    let (status, outcome, message) = match consume_result {
        Ok(outcome) => {
            let _ = plugin.refresh_accounts();
            (
                OperationStatus::Success,
                Some(menubar_reset_credit_outcome(&outcome)),
                reset_credit_message(&outcome),
            )
        }
        Err(err) => {
            diagnostics.push(Diagnostic {
                code: "reset_credit_failed".to_string(),
                message: sanitize_diagnostic(&err.to_string()),
                provider_id: Some(command.provider.clone()),
                target_id: Some(command.local_id.clone()),
                scope: Some("target".to_string()),
                recovery_action: Some(format!("Run `omx doctor {}`.", command.provider)),
            });
            (
                OperationStatus::Failed,
                None,
                "Reset credit consume failed.".to_string(),
            )
        }
    };
    let dashboard = menubar_dashboard(plugins, DashboardQuery::default(), store)?;
    let active_after = active_target_for_provider_from_report(plugin.id(), &dashboard.accounts);
    let accounts = dashboard.accounts.clone();
    Ok(ConsumeResetCreditReport {
        control_plane_schema_version: CONTROL_PLANE_SCHEMA_VERSION,
        state_schema_version: STATE_SCHEMA_VERSION,
        generated_at_unix: unix_now(),
        provider: command.provider,
        requested_local_id: command.local_id,
        operation: OperationResult {
            status,
            changed: matches!(outcome, Some(ResetCreditOutcomeView::Reset { .. })),
            active_before: active,
            active_after,
            message,
            diagnostics,
        },
        outcome,
        dashboard,
        accounts,
    })
}

pub fn menubar_refresh(
    plugins: &[Box<dyn PlatformPlugin>],
    command: RefreshCommand,
    store: Option<&omx_core::StateStore>,
) -> Result<RefreshReport> {
    let _guard = OPERATION_LOCK
        .try_lock()
        .map_err(|_| OpenMuxError::Message("menubar operation already in progress".to_string()))?;
    let now = unix_now();
    let plugin = find_plugin(plugins, &command.provider)?;
    let refresh_target = match command.local_id.as_ref() {
        Some(local_id) => {
            let catalog = target_catalog(plugin)?;
            let target = resolve_menubar_target(
                plugin.id(),
                &catalog,
                &SwitchCommand {
                    provider: command.provider.clone(),
                    local_id: local_id.clone(),
                    target_kind: command.target_kind,
                },
            )?;
            if target.kind != TargetKind::Account {
                return Err(OpenMuxError::Message(format!(
                    "`{local_id}` is not an account target for `{}`",
                    command.provider
                )));
            }
            Some(target)
        }
        None => None,
    };
    let generation = match begin_refresh_request(&command.provider, command.request_generation) {
        RefreshAdmission::Accepted(generation) => generation,
        RefreshAdmission::Skipped { generation, reason } => {
            let dashboard = menubar_dashboard(plugins, DashboardQuery::default(), store)?;
            let active =
                active_target_for_provider_from_report(&command.provider, &dashboard.accounts);
            let operation = OperationResult {
                status: OperationStatus::Skipped,
                changed: false,
                active_before: active.clone(),
                active_after: active,
                message: format!("Refresh skipped: {reason}."),
                diagnostics: Vec::new(),
            };
            let accounts = dashboard.accounts.clone();
            return Ok(RefreshReport {
                control_plane_schema_version: CONTROL_PLANE_SCHEMA_VERSION,
                state_schema_version: STATE_SCHEMA_VERSION,
                generated_at_unix: unix_now(),
                provider: command.provider,
                requested_local_id: command.local_id,
                kind: command.kind,
                generation,
                operation,
                dashboard,
                refreshed: false,
                skipped_reason: Some(reason),
                accounts,
            });
        }
    };
    let skipped_reason = refresh_skip_reason(&command.provider, &command.kind, now);
    let refreshed = skipped_reason.is_none();
    let mut operation_status = if refreshed {
        OperationStatus::Success
    } else {
        OperationStatus::Skipped
    };
    let target_label = command.local_id.as_deref();
    let mut operation_message = skipped_reason.as_ref().map_or_else(
        || match target_label {
            Some(_) => "Account usage refreshed.".to_string(),
            None => "Provider refreshed.".to_string(),
        },
        |reason| format!("Refresh skipped: {reason}."),
    );
    let mut operation_diagnostics = Vec::new();
    if refreshed {
        let result = match refresh_target.as_ref() {
            Some(target) => plugin
                .refresh_account(&target.target_id)
                .map(|status| vec![status]),
            None => plugin.refresh_accounts(),
        };
        record_refresh_result(&command.provider, generation, now, result.is_ok());
        if let Err(err) = result {
            operation_status = OperationStatus::Failed;
            operation_message = match target_label {
                Some(_) => "Account usage refresh failed; showing last known data.".to_string(),
                None => "Refresh failed; showing last known data.".to_string(),
            };
            operation_diagnostics.push(Diagnostic {
                code: "refresh_failed".to_string(),
                message: sanitize_diagnostic(&err.to_string()),
                provider_id: Some(command.provider.clone()),
                target_id: command.local_id.clone(),
                scope: Some(if command.local_id.is_some() {
                    "target".to_string()
                } else {
                    "provider".to_string()
                }),
                recovery_action: Some(format!("Run `omx doctor {}`.", command.provider)),
            });
        }
    } else {
        release_refresh_request(&command.provider, generation);
    }
    let dashboard = menubar_dashboard(plugins, DashboardQuery::default(), store)?;
    let active = active_target_for_provider_from_report(&command.provider, &dashboard.accounts);
    let refreshed = refreshed && operation_status == OperationStatus::Success;
    let operation = OperationResult {
        status: operation_status,
        changed: false,
        active_before: active.clone(),
        active_after: active,
        message: operation_message,
        diagnostics: operation_diagnostics,
    };
    let accounts = dashboard.accounts.clone();
    Ok(RefreshReport {
        control_plane_schema_version: CONTROL_PLANE_SCHEMA_VERSION,
        state_schema_version: STATE_SCHEMA_VERSION,
        generated_at_unix: unix_now(),
        provider: command.provider,
        requested_local_id: command.local_id,
        kind: command.kind,
        generation,
        operation,
        dashboard,
        refreshed,
        skipped_reason,
        accounts,
    })
}

fn menubar_reset_credit_outcome(outcome: &ResetCreditOutcome) -> ResetCreditOutcomeView {
    match outcome {
        ResetCreditOutcome::Reset { windows_reset } => ResetCreditOutcomeView::Reset {
            windows_reset: *windows_reset,
        },
        ResetCreditOutcome::NothingToReset => ResetCreditOutcomeView::NothingToReset,
        ResetCreditOutcome::NoCredit => ResetCreditOutcomeView::NoCredit,
        ResetCreditOutcome::AlreadyRedeemed => ResetCreditOutcomeView::AlreadyRedeemed,
    }
}

fn reset_credit_message(outcome: &ResetCreditOutcome) -> String {
    match outcome {
        ResetCreditOutcome::Reset { windows_reset } => {
            format!("Reset credit consumed; reset {windows_reset} usage window(s).")
        }
        ResetCreditOutcome::NothingToReset => {
            "No active limit was eligible for reset; no credit was consumed.".to_string()
        }
        ResetCreditOutcome::NoCredit => "No reset credits available.".to_string(),
        ResetCreditOutcome::AlreadyRedeemed => {
            "Reset credit was already redeemed for this request.".to_string()
        }
    }
}

fn resolve_menubar_target(
    provider: &str,
    catalog: &TargetCatalog,
    command: &SwitchCommand,
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
        Some(TargetKindView::Account) => matched_account
            .map(|status| TargetResolution {
                kind: TargetKind::Account,
                target_id: status.account.local_id.clone(),
            })
            .ok_or_else(|| missing_target(provider, &command.local_id, "account")),
        Some(TargetKindView::Profile) => matched_profile
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
