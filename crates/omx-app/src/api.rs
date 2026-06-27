pub use crate::compatibility::{
    ClientDescriptor, CompatibilityResult, compatibility_view as control_plane_compatibility,
};
pub use crate::dto::*;
pub use crate::mutation::{
    activate_target, menubar_refresh, menubar_remove, menubar_switch, refresh_all,
    refresh_provider, remove_target,
};
pub use crate::query::{
    account_statuses, active_account_status, config_profiles, dashboard_view, menubar_accounts,
    menubar_dashboard, provider_view, remove_resolved_target, resolve_target, target_catalog,
    use_resolved_target,
};
pub use crate::runtime::reset_menubar_refresh_state_for_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{mutation::OPERATION_LOCK, query::menubar_today_usage};
    use omx_core::{
        AccountRef, AccountStatus, Availability, AvailabilityState, ConfigProfile, CostStatus,
        DoctorReport, ImportConfigOptions, ImportedConfig, LoginOptions, OpenMuxError,
        PlatformCapabilities, PlatformInfo, PlatformInstall, PlatformPlugin, PlatformPoolSummary,
        Result, SaveOptions, SwitchReport, UsageDataQuality, UsageEvent, UsageEventSource,
        UsageSnapshot, UsageTokenBreakdown, storage::unix_now,
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
        assert!(!report.accounts[0].actions.can_activate);
        assert_eq!(
            report.accounts[0].actions.disabled_reason.as_deref(),
            Some("already_active")
        );
        assert!(report.accounts[1].actions.can_activate);
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
                request_generation: None,
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
                request_generation: None,
            },
            None,
        )
        .unwrap();
        let second = menubar_refresh(
            &plugins,
            MenubarRefreshCommand {
                provider: "codex".to_string(),
                kind: RefreshKind::Background,
                request_generation: None,
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
    fn stale_refresh_generation_is_skipped_without_provider_call() {
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
                kind: RefreshKind::Interactive,
                request_generation: Some(10),
            },
            None,
        )
        .unwrap();
        let stale = menubar_refresh(
            &plugins,
            MenubarRefreshCommand {
                provider: "codex".to_string(),
                kind: RefreshKind::Interactive,
                request_generation: Some(9),
            },
            None,
        )
        .unwrap();

        assert_eq!(first.generation, 10);
        assert_eq!(stale.generation, 10);
        assert_eq!(stale.skipped_reason.as_deref(), Some("stale_request"));
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
                request_generation: None,
            },
            None,
        )
        .unwrap();
        let skipped = menubar_refresh(
            &working,
            MenubarRefreshCommand {
                provider: "codex".to_string(),
                kind: RefreshKind::Background,
                request_generation: None,
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
    fn provider_view_requires_provider_scope() {
        let plugins = vec![
            Box::new(FakePlugin::new(vec![account(1, true, None)])) as Box<dyn PlatformPlugin>
        ];

        let err = provider_view(&plugins, MenubarQuery::default(), None).unwrap_err();

        assert!(err.to_string().contains("requires a provider"));
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
                request_generation: None,
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
        reset_menubar_refresh_state_for_tests();
    }
}
