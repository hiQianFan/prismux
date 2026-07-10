use super::*;

#[test]
fn saves_lists_and_switches_codex_auth_snapshots_by_number_and_alias() {
    let temp = test_temp_dir("save-switch");
    let codex_home = temp.join("codex-home");
    let state_root = temp.join("prismux-state");
    fs::create_dir_all(&codex_home).unwrap();
    fs::write(codex_home.join(AUTH_FILE_NAME), br#"{"account":"work"}"#).unwrap();

    let plugin = CodexPlugin::with_paths(&codex_home, &state_root);
    let first = plugin
        .save_current(SaveOptions {
            alias: Some("work".to_string()),
        })
        .unwrap();
    assert_eq!(first.number, 1);
    assert_eq!(first.alias.as_deref(), Some("work"));

    fs::write(
        codex_home.join(AUTH_FILE_NAME),
        br#"{"account":"personal"}"#,
    )
    .unwrap();
    let second = plugin.save_current(SaveOptions::default()).unwrap();
    assert_eq!(second.number, 2);
    assert_eq!(second.alias, None);

    let accounts = plugin.list_accounts().unwrap();
    assert_eq!(accounts.len(), 2);
    assert_eq!(accounts[0].account.number, 1);
    assert_eq!(accounts[1].account.number, 2);

    let report = plugin.switch_to("1").unwrap();
    assert_eq!(report.current.number, 1);
    assert_eq!(
        fs::read(codex_home.join(AUTH_FILE_NAME)).unwrap(),
        br#"{"account":"work"}"#
    );
    assert_eq!(plugin.current().unwrap().unwrap().account.number, 1);
    assert!(plugin.backups_dir().unwrap().exists());

    let report = plugin.switch_to("work").unwrap();
    assert_eq!(report.current.number, 1);
}

#[test]
fn duplicate_save_updates_existing_account_instead_of_appending() {
    let temp = test_temp_dir("duplicate-save");
    let codex_home = temp.join("codex-home");
    let state_root = temp.join("prismux-state");
    fs::create_dir_all(&codex_home).unwrap();
    fs::write(codex_home.join(AUTH_FILE_NAME), br#"{"account":"same"}"#).unwrap();

    let plugin = CodexPlugin::with_paths(&codex_home, &state_root);
    let first = plugin.save_current(SaveOptions::default()).unwrap();
    let second = plugin
        .save_current(SaveOptions {
            alias: Some("same".to_string()),
        })
        .unwrap();

    assert_eq!(first.number, second.number);
    assert_eq!(plugin.list_accounts().unwrap().len(), 1);
    assert_eq!(second.alias.as_deref(), Some("same"));
}

#[test]
fn codex_usage_proxy_reads_prismux_settings() {
    let temp = test_temp_dir("usage-proxy-settings");
    let state_root = temp.join("prismux-state");
    write_file_atomic_private(
        &state_root.join("control-plane").join("settings.json"),
        br#"{"schema_version":2,"general":{"refresh_cadence_seconds":300},"network":{"proxy_enabled":true,"proxy_url":"socks5h://127.0.0.1:1080"},"providers":[{"provider":"codex","display_label":"Codex","enabled":true,"status":{"status":"ready","status_text":"Ready","status_tone":"success"},"diagnostics":[]}],"privacy":{"hide_personal_identifiers":false}}"#,
    )
    .unwrap();

    assert_eq!(
        codex_usage_proxy(&state_root).as_deref(),
        Some("socks5h://127.0.0.1:1080")
    );
}

#[test]
fn duplicate_codex_subject_updates_existing_account_even_when_auth_hash_changes() {
    let temp = test_temp_dir("duplicate-subject");
    let codex_home = temp.join("codex-home");
    let state_root = temp.join("prismux-state");
    fs::create_dir_all(&codex_home).unwrap();
    let plugin = CodexPlugin::with_paths(&codex_home, &state_root);

    fs::write(
        codex_home.join(AUTH_FILE_NAME),
        codex_auth_with_claims("profile@example.com", "plus", "user-123", "account-456", 1),
    )
    .unwrap();
    let first = plugin.save_current(SaveOptions::default()).unwrap();

    fs::write(
        codex_home.join(AUTH_FILE_NAME),
        codex_auth_with_claims("profile@example.com", "plus", "user-123", "account-456", 2),
    )
    .unwrap();
    let second = plugin
        .save_current(SaveOptions {
            alias: Some("work".to_string()),
        })
        .unwrap();

    let accounts = plugin.list_accounts().unwrap();
    assert_eq!(first.number, second.number);
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].account.alias.as_deref(), Some("work"));
}

#[test]
fn same_email_with_different_codex_account_subjects_stays_separate() {
    let temp = test_temp_dir("same-email-different-subject");
    let codex_home = temp.join("codex-home");
    let state_root = temp.join("prismux-state");
    fs::create_dir_all(&codex_home).unwrap();
    let plugin = CodexPlugin::with_paths(&codex_home, &state_root);

    fs::write(
        codex_home.join(AUTH_FILE_NAME),
        codex_auth_with_claims("profile@example.com", "plus", "user-123", "account-456", 1),
    )
    .unwrap();
    let first = plugin.save_current(SaveOptions::default()).unwrap();

    fs::write(
        codex_home.join(AUTH_FILE_NAME),
        codex_auth_with_claims("profile@example.com", "team", "user-123", "account-789", 2),
    )
    .unwrap();
    let second = plugin.save_current(SaveOptions::default()).unwrap();

    assert_ne!(first.number, second.number);
    assert_eq!(plugin.list_accounts().unwrap().len(), 2);
}

#[test]
fn switching_persists_rotated_active_auth_before_replacing_it() {
    let temp = test_temp_dir("rotated-active-auth");
    let codex_home = temp.join("codex-home");
    let state_root = temp.join("prismux-state");
    fs::create_dir_all(&codex_home).unwrap();
    let plugin = CodexPlugin::with_paths(&codex_home, &state_root);
    let auth_path = codex_home.join(AUTH_FILE_NAME);

    let first_auth = codex_auth_with_claims(
        "first@example.com",
        "plus",
        "user-first",
        "account-first",
        1,
    );
    fs::write(&auth_path, &first_auth).unwrap();
    plugin
        .save_current(SaveOptions {
            alias: Some("first".to_string()),
        })
        .unwrap();

    let second_auth = codex_auth_with_claims(
        "second@example.com",
        "plus",
        "user-second",
        "account-second",
        1,
    );
    fs::write(&auth_path, &second_auth).unwrap();
    plugin
        .save_current(SaveOptions {
            alias: Some("second".to_string()),
        })
        .unwrap();
    plugin.switch_to("first").unwrap();

    let rotated_first_auth = codex_auth_with_claims(
        "first@example.com",
        "plus",
        "user-first",
        "account-first",
        2,
    );
    fs::write(&auth_path, &rotated_first_auth).unwrap();
    let old_snapshot = plugin.account_snapshot_path_for_number(1).unwrap();

    plugin.switch_to("second").unwrap();

    let refreshed_snapshot = plugin.account_snapshot_path_for_number(1).unwrap();
    assert_ne!(old_snapshot, refreshed_snapshot);
    assert!(!old_snapshot.exists());
    assert_eq!(
        fs::read(&refreshed_snapshot).unwrap(),
        rotated_first_auth.as_bytes()
    );

    plugin.switch_to("first").unwrap();
    assert_eq!(fs::read(auth_path).unwrap(), rotated_first_auth.as_bytes());
}

#[test]
fn snapshot_write_failure_keeps_active_auth_and_metadata_unchanged() {
    let temp = test_temp_dir("snapshot-write-failure");
    let codex_home = temp.join("codex-home");
    let state_root = temp.join("prismux-state");
    fs::create_dir_all(&codex_home).unwrap();
    let mut plugin = CodexPlugin::with_paths(&codex_home, &state_root);
    let auth_path = codex_home.join(AUTH_FILE_NAME);

    let first_auth = codex_auth_with_claims(
        "first@example.com",
        "plus",
        "user-first",
        "account-first",
        1,
    );
    fs::write(&auth_path, &first_auth).unwrap();
    plugin
        .save_current(SaveOptions {
            alias: Some("first".to_string()),
        })
        .unwrap();
    let second_auth = codex_auth_with_claims(
        "second@example.com",
        "plus",
        "user-second",
        "account-second",
        1,
    );
    fs::write(&auth_path, &second_auth).unwrap();
    plugin
        .save_current(SaveOptions {
            alias: Some("second".to_string()),
        })
        .unwrap();
    plugin.switch_to("first").unwrap();

    let first_before = plugin.resolve_account("first").unwrap();
    let rotated_first_auth = codex_auth_with_claims(
        "first@example.com",
        "plus",
        "user-first",
        "account-first",
        2,
    );
    fs::write(&auth_path, &rotated_first_auth).unwrap();
    plugin.fail_snapshot_write();

    let err = plugin.switch_to("second").unwrap_err();

    assert!(err.to_string().contains("snapshot write failure"));
    assert_eq!(fs::read(&auth_path).unwrap(), rotated_first_auth.as_bytes());
    let first_after = plugin.resolve_account("first").unwrap();
    assert_eq!(first_after.auth_hash, first_before.auth_hash);
    assert_eq!(first_after.secret_ref, first_before.secret_ref);
    assert_eq!(
        plugin.current().unwrap().unwrap().account.alias.as_deref(),
        Some("first")
    );
}

#[test]
fn switching_rejects_auth_changed_after_snapshot_sync() {
    let temp = test_temp_dir("auth-changed-after-sync");
    let codex_home = temp.join("codex-home");
    let state_root = temp.join("prismux-state");
    fs::create_dir_all(&codex_home).unwrap();
    let mut plugin = CodexPlugin::with_paths(&codex_home, &state_root);
    let auth_path = codex_home.join(AUTH_FILE_NAME);

    fs::write(&auth_path, br#"{"account":"first"}"#).unwrap();
    plugin
        .save_current(SaveOptions {
            alias: Some("first".to_string()),
        })
        .unwrap();
    fs::write(&auth_path, br#"{"account":"second"}"#).unwrap();
    plugin
        .save_current(SaveOptions {
            alias: Some("second".to_string()),
        })
        .unwrap();
    plugin.switch_to("first").unwrap();

    plugin.set_before_auth_replace(overwrite_auth_during_replace);
    let err = plugin.switch_to("second").unwrap_err();

    assert!(err.to_string().contains("changed during account switching"));
    assert_eq!(
        fs::read(&auth_path).unwrap(),
        br#"{"account":"concurrent"}"#
    );
    assert_eq!(
        plugin.current().unwrap().unwrap().account.alias.as_deref(),
        Some("first")
    );
}

#[test]
fn switching_rejects_active_auth_from_a_different_account() {
    let temp = test_temp_dir("mismatched-active-auth");
    let codex_home = temp.join("codex-home");
    let state_root = temp.join("prismux-state");
    fs::create_dir_all(&codex_home).unwrap();
    let plugin = CodexPlugin::with_paths(&codex_home, &state_root);
    let auth_path = codex_home.join(AUTH_FILE_NAME);

    fs::write(
        &auth_path,
        codex_auth_with_claims("a@example.com", "plus", "user-a", "account-a", 1),
    )
    .unwrap();
    plugin
        .save_current(SaveOptions {
            alias: Some("a".to_string()),
        })
        .unwrap();
    fs::write(
        &auth_path,
        codex_auth_with_claims("b@example.com", "plus", "user-b", "account-b", 1),
    )
    .unwrap();
    plugin
        .save_current(SaveOptions {
            alias: Some("b".to_string()),
        })
        .unwrap();
    plugin.switch_to("a").unwrap();

    let a_before = plugin.resolve_account("a").unwrap();
    let foreign_auth = codex_auth_with_claims(
        "foreign@example.com",
        "plus",
        "user-foreign",
        "account-foreign",
        1,
    );
    fs::write(&auth_path, &foreign_auth).unwrap();

    let err = plugin.switch_to("b").unwrap_err();

    assert!(err.to_string().contains("does not belong"));
    assert_eq!(fs::read(&auth_path).unwrap(), foreign_auth.as_bytes());
    let a_after = plugin.resolve_account("a").unwrap();
    assert_eq!(a_after.auth_hash, a_before.auth_hash);
    assert_eq!(a_after.secret_ref, a_before.secret_ref);
    assert_eq!(
        plugin.current().unwrap().unwrap().account.alias.as_deref(),
        Some("a")
    );
}

#[test]
fn switching_rejects_changed_auth_when_identity_cannot_be_verified() {
    let temp = test_temp_dir("unverifiable-active-auth");
    let codex_home = temp.join("codex-home");
    let state_root = temp.join("prismux-state");
    fs::create_dir_all(&codex_home).unwrap();
    let plugin = CodexPlugin::with_paths(&codex_home, &state_root);
    let auth_path = codex_home.join(AUTH_FILE_NAME);

    fs::write(&auth_path, br#"{"account":"a"}"#).unwrap();
    plugin
        .save_current(SaveOptions {
            alias: Some("a".to_string()),
        })
        .unwrap();
    fs::write(&auth_path, br#"{"account":"b"}"#).unwrap();
    plugin
        .save_current(SaveOptions {
            alias: Some("b".to_string()),
        })
        .unwrap();
    plugin.switch_to("a").unwrap();

    let changed_auth = br#"{"account":"a","rotated":true}"#;
    fs::write(&auth_path, changed_auth).unwrap();
    let err = plugin.switch_to("b").unwrap_err();

    assert!(err.to_string().contains("identity cannot be verified"));
    assert_eq!(fs::read(auth_path).unwrap(), changed_auth);
    assert_eq!(
        plugin.current().unwrap().unwrap().account.alias.as_deref(),
        Some("a")
    );
}

#[test]
fn legacy_duplicate_codex_accounts_are_merged_when_subject_can_be_backfilled() {
    let temp = test_temp_dir("legacy-duplicate-subject");
    let codex_home = temp.join("codex-home");
    let state_root = temp.join("prismux-state");
    let snapshot_dir = temp.join("snapshots");
    fs::create_dir_all(&codex_home).unwrap();
    fs::create_dir_all(&snapshot_dir).unwrap();
    let plugin = CodexPlugin::with_paths(&codex_home, &state_root);
    let store = plugin.state_store().unwrap();

    for nonce in 1..=2 {
        let auth = codex_auth_with_claims(
            "profile@example.com",
            "plus",
            "user-123",
            "account-456",
            nonce,
        );
        let snapshot_path = snapshot_dir.join(format!("{nonce}.auth.json"));
        fs::write(&snapshot_path, auth.as_bytes()).unwrap();
        store
            .upsert_account(UpsertAccount {
                provider: plugin.id().to_string(),
                alias: None,
                provider_subject_kind: None,
                provider_subject_hash: None,
                provider_subject_label: None,
                account_label: Some("profile@example.com".to_string()),
                plan_label: Some("Plus".to_string()),
                auth_type: None,
                expires_at_unix: None,
                auth_hash: sha256_hex(auth.as_bytes()),
                secret_ref: display_path(&snapshot_path),
                imported_at_unix: nonce as u64,
            })
            .unwrap();
    }

    plugin.reconcile_account_subjects(&store).unwrap();
    let accounts = plugin.list_accounts().unwrap();

    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].account.number, 1);
    let stored = store.list_accounts(plugin.id()).unwrap();
    assert_eq!(stored.len(), 1);
    assert_eq!(
        stored[0].provider_subject_kind.as_deref(),
        Some("codex_chatgpt_account")
    );
    assert!(stored[0].secret_ref.ends_with("2.auth.json"));
    assert!(snapshot_dir.join("1.auth.json").exists());
    assert!(snapshot_dir.join("2.auth.json").exists());
}

#[test]
fn remove_account_deletes_snapshot_and_excludes_from_list() {
    let temp = test_temp_dir("remove-account");
    let codex_home = temp.join("codex-home");
    let state_root = temp.join("prismux-state");
    fs::create_dir_all(&codex_home).unwrap();
    fs::write(codex_home.join(AUTH_FILE_NAME), br#"{"account":"work"}"#).unwrap();

    let plugin = CodexPlugin::with_paths(&codex_home, &state_root);
    plugin
        .save_current(SaveOptions {
            alias: Some("work".to_string()),
        })
        .unwrap();
    plugin.switch_to("work").unwrap();
    let snapshot_path = plugin.account_snapshot_path_for_number(1).unwrap();
    assert!(snapshot_path.exists());

    let report = plugin.remove_target("work").unwrap();

    let RemoveReport::Account(report) = report else {
        panic!("expected account removal");
    };
    assert!(report.was_active);
    assert_eq!(report.account.number, 1);
    assert!(!snapshot_path.exists());
    assert!(plugin.current().unwrap().is_none());
    assert!(plugin.list_accounts().unwrap().is_empty());
}

#[cfg(unix)]
#[test]
fn duplicate_login_updates_existing_account_instead_of_appending() {
    let temp = test_temp_dir("duplicate-login");
    let codex_home = temp.join("codex-home");
    let state_root = temp.join("prismux-state");
    let fake_codex = fake_codex_static_executable(&temp);
    let plugin =
        CodexPlugin::with_paths_and_codex_executable(&codex_home, &state_root, &fake_codex);

    let first = plugin.login(LoginOptions::default()).unwrap();
    let second = plugin
        .login(LoginOptions {
            alias: Some("same".to_string()),
            ..LoginOptions::default()
        })
        .unwrap();

    assert_eq!(first.number, 1);
    assert_eq!(second.number, 1);
    assert_eq!(second.alias.as_deref(), Some("same"));
    assert_eq!(plugin.list_accounts().unwrap().len(), 1);
}

#[cfg(unix)]
#[test]
fn login_cleans_up_temporary_codex_home_after_import() {
    let temp = test_temp_dir("login-cleanup");
    let codex_home = temp.join("codex-home");
    let state_root = temp.join("prismux-state");
    let fake_codex = fake_codex_static_executable(&temp);
    let plugin =
        CodexPlugin::with_paths_and_codex_executable(&codex_home, &state_root, &fake_codex);

    plugin.login(LoginOptions::default()).unwrap();

    let login_dir = plugin.login_dir().unwrap();
    let remaining = if login_dir.exists() {
        fs::read_dir(&login_dir)
            .unwrap()
            .collect::<std::result::Result<Vec<_>, _>>()
            .unwrap()
    } else {
        Vec::new()
    };
    assert!(remaining.is_empty());
}

#[cfg(unix)]
#[test]
fn login_assigns_numbers_supports_alias_device_auth_and_use() {
    let temp = test_temp_dir("login");
    let codex_home = temp.join("codex-home");
    let state_root = temp.join("prismux-state");
    let fake_codex = fake_codex_executable(&temp);
    let plugin =
        CodexPlugin::with_paths_and_codex_executable(&codex_home, &state_root, &fake_codex);

    let first = plugin.login(LoginOptions::default()).unwrap();
    assert_eq!(first.number, 1);
    assert!(plugin.current().unwrap().is_none());

    let second = plugin
        .login(LoginOptions {
            device_auth: true,
            alias: Some("work".to_string()),
            activate: true,
        })
        .unwrap();
    assert_eq!(second.number, 2);
    assert_eq!(second.alias.as_deref(), Some("work"));
    assert_eq!(plugin.current().unwrap().unwrap().account.number, 2);
    assert_eq!(
        fs::read(codex_home.join(AUTH_FILE_NAME)).unwrap(),
        br#"{"account":"2"}"#
    );

    let args_log = fs::read_to_string(fake_codex.with_extension("args")).unwrap();
    assert!(args_log.contains("login\n"));
    assert!(args_log.contains("login --device-auth\n"));
}

#[cfg(unix)]
#[test]
fn relogin_of_active_account_keeps_new_credentials_active_without_use_flag() {
    let temp = test_temp_dir("relogin-active-account");
    let codex_home = temp.join("codex-home");
    let state_root = temp.join("prismux-state");
    let first_auth =
        codex_auth_with_claims("same@example.com", "plus", "same-user", "same-account", 1);
    let refreshed_auth =
        codex_auth_with_claims("same@example.com", "plus", "same-user", "same-account", 2);
    let fake_codex =
        fake_codex_rotating_same_account_executable(&temp, &first_auth, &refreshed_auth);
    let plugin =
        CodexPlugin::with_paths_and_codex_executable(&codex_home, &state_root, &fake_codex);

    plugin
        .login(LoginOptions {
            activate: true,
            ..LoginOptions::default()
        })
        .unwrap();
    let relogged = plugin.login(LoginOptions::default()).unwrap();

    assert_eq!(relogged.number, 1);
    assert_eq!(plugin.list_accounts().unwrap().len(), 1);
    assert_eq!(
        fs::read(codex_home.join(AUTH_FILE_NAME)).unwrap(),
        refreshed_auth.as_bytes()
    );
    assert_eq!(
        fs::read(plugin.account_snapshot_path_for_number(1).unwrap()).unwrap(),
        refreshed_auth.as_bytes()
    );
}

#[cfg(unix)]
#[test]
fn relogin_read_failure_rolls_back_active_snapshot_metadata() {
    let temp = test_temp_dir("relogin-read-rollback");
    let codex_home = temp.join("codex-home");
    let state_root = temp.join("prismux-state");
    let first_auth =
        codex_auth_with_claims("same@example.com", "plus", "same-user", "same-account", 1);
    let refreshed_auth =
        codex_auth_with_claims("same@example.com", "plus", "same-user", "same-account", 2);
    let fake_codex =
        fake_codex_rotating_same_account_executable(&temp, &first_auth, &refreshed_auth);
    let mut plugin =
        CodexPlugin::with_paths_and_codex_executable(&codex_home, &state_root, &fake_codex);

    plugin
        .login(LoginOptions {
            activate: true,
            ..LoginOptions::default()
        })
        .unwrap();
    let original_snapshot = plugin.account_snapshot_path_for_number(1).unwrap();
    let original_record = plugin
        .state_store()
        .unwrap()
        .account_by_selector(plugin.id(), "1")
        .unwrap()
        .unwrap();

    plugin.set_before_auth_replace(remove_auth_during_replace);
    let err = plugin.login(LoginOptions::default()).unwrap_err();

    assert!(err.to_string().contains("metadata was rolled back"));
    let rolled_back = plugin
        .state_store()
        .unwrap()
        .account_by_selector(plugin.id(), "1")
        .unwrap()
        .unwrap();
    assert_eq!(rolled_back.auth_hash, original_record.auth_hash);
    assert_eq!(rolled_back.secret_ref, original_record.secret_ref);
    assert!(original_snapshot.exists());
    assert_eq!(fs::read(original_snapshot).unwrap(), first_auth.as_bytes());
}

#[test]
fn alias_set_rejects_all_digit_aliases() {
    let temp = test_temp_dir("alias");
    let codex_home = temp.join("codex-home");
    let state_root = temp.join("prismux-state");
    fs::create_dir_all(&codex_home).unwrap();
    fs::write(codex_home.join(AUTH_FILE_NAME), br#"{"account":"work"}"#).unwrap();

    let plugin = CodexPlugin::with_paths(&codex_home, &state_root);
    plugin.save_current(SaveOptions::default()).unwrap();
    let err = plugin.set_alias("1", "123").unwrap_err();
    assert!(err.to_string().contains("all digits"));
}

#[test]
fn clear_alias_removes_alias_without_removing_account() {
    let temp = test_temp_dir("clear-alias");
    let codex_home = temp.join("codex-home");
    let state_root = temp.join("prismux-state");
    fs::create_dir_all(&codex_home).unwrap();
    fs::write(codex_home.join(AUTH_FILE_NAME), br#"{"account":"work"}"#).unwrap();

    let plugin = CodexPlugin::with_paths(&codex_home, &state_root);
    plugin
        .save_current(SaveOptions {
            alias: Some("work".to_string()),
        })
        .unwrap();

    let store = plugin.state_store().unwrap();
    let cleared = store
        .clear_account_alias_by_selector(plugin.id(), "work", unix_now())
        .unwrap();

    assert_eq!(cleared.number, 1);
    assert_eq!(cleared.alias, None);
    assert!(plugin.switch_to("1").is_ok());
    assert!(
        store
            .account_by_selector(plugin.id(), "work")
            .unwrap()
            .is_none()
    );
    assert!(
        store
            .account_by_selector(plugin.id(), "1")
            .unwrap()
            .is_some()
    );
}

#[test]
fn save_rejects_alias_used_by_another_account() {
    let temp = test_temp_dir("duplicate-alias");
    let codex_home = temp.join("codex-home");
    let state_root = temp.join("prismux-state");
    fs::create_dir_all(&codex_home).unwrap();

    fs::write(codex_home.join(AUTH_FILE_NAME), br#"{"account":"work"}"#).unwrap();
    let plugin = CodexPlugin::with_paths(&codex_home, &state_root);
    plugin
        .save_current(SaveOptions {
            alias: Some("shared".to_string()),
        })
        .unwrap();

    fs::write(
        codex_home.join(AUTH_FILE_NAME),
        br#"{"account":"personal"}"#,
    )
    .unwrap();
    let err = plugin
        .save_current(SaveOptions {
            alias: Some("shared".to_string()),
        })
        .unwrap_err();

    assert!(err.to_string().contains("already used"));
}

#[test]
fn switch_rejects_tampered_auth_snapshot() {
    let temp = test_temp_dir("tampered-snapshot");
    let codex_home = temp.join("codex-home");
    let state_root = temp.join("prismux-state");
    fs::create_dir_all(&codex_home).unwrap();
    let plugin = CodexPlugin::with_paths(&codex_home, &state_root);

    fs::write(codex_home.join(AUTH_FILE_NAME), br#"{"account":"work"}"#).unwrap();
    plugin.save_current(SaveOptions::default()).unwrap();
    fs::write(
        codex_home.join(AUTH_FILE_NAME),
        br#"{"account":"personal"}"#,
    )
    .unwrap();
    plugin.save_current(SaveOptions::default()).unwrap();

    fs::write(
        plugin.account_snapshot_path_for_number(1).unwrap(),
        br#"{"account":"tampered"}"#,
    )
    .unwrap();
    let err = plugin.switch_to("1").unwrap_err();

    assert!(err.to_string().contains("hash verification"));
    assert_eq!(
        fs::read(codex_home.join(AUTH_FILE_NAME)).unwrap(),
        br#"{"account":"personal"}"#
    );
}

#[test]
fn use_target_rejects_account_profile_selector_ambiguity() {
    let temp = test_temp_dir("selector-ambiguity");
    let codex_home = temp.join("codex-home");
    let state_root = temp.join("prismux-state");
    fs::create_dir_all(&codex_home).unwrap();
    fs::write(codex_home.join(AUTH_FILE_NAME), br#"{"account":"work"}"#).unwrap();
    let plugin = CodexPlugin::with_paths(&codex_home, &state_root);
    plugin
        .save_current(SaveOptions {
            alias: Some("work".to_string()),
        })
        .unwrap();
    plugin
        .import_config(ImportConfigOptions {
            name: Some("work".to_string()),
            content: "OPENAI_BASE_URL=https://gateway.example.com OPENAI_API_KEY=sk-test"
                .to_string(),
        })
        .unwrap();

    let err = plugin.use_target("work").unwrap_err();

    assert!(err.to_string().contains("matches both account"));
}

#[test]
fn codex_account_switch_deactivates_managed_provider_config() {
    let temp = test_temp_dir("active-account-provider-independent");
    let codex_home = temp.join("codex-home");
    let state_root = temp.join("prismux-state");
    fs::create_dir_all(&codex_home).unwrap();
    fs::write(codex_home.join(AUTH_FILE_NAME), br#"{"account":"work"}"#).unwrap();
    fs::write(
        codex_home.join("config.toml"),
        "model = \"default\"\n\n[plugins.\"ponytail@ponytail\"]\nenabled = true\n",
    )
    .unwrap();

    let plugin = CodexPlugin::with_paths(&codex_home, &state_root);
    plugin
        .save_current(SaveOptions {
            alias: Some("work".to_string()),
        })
        .unwrap();
    assert!(plugin.list_accounts().unwrap()[0].active);

    plugin
        .import_config(ImportConfigOptions {
            name: Some("gateway".to_string()),
            content: "OPENAI_BASE_URL=https://gateway.example.com OPENAI_API_KEY=sk-test"
                .to_string(),
        })
        .unwrap();
    plugin.use_target("gateway").unwrap();

    assert!(plugin.current().unwrap().is_none());
    assert!(!plugin.list_accounts().unwrap()[0].active);
    assert!(
        plugin
            .list_configs()
            .unwrap()
            .into_iter()
            .find(|profile| profile.name == "gateway")
            .unwrap()
            .active
    );
    let provider_config = fs::read(codex_home.join("config.toml")).unwrap();
    let provider_text = String::from_utf8(provider_config.clone()).unwrap();
    assert!(provider_text.contains("model_provider = \"prismux-gateway\""));
    assert!(provider_text.contains("[model_providers.prismux-gateway]"));
    assert!(provider_text.contains("[plugins.\"ponytail@ponytail\"]"));

    plugin.use_target("work").unwrap();

    assert_eq!(plugin.current().unwrap().unwrap().account.number, 1);
    assert!(
        !plugin
            .list_configs()
            .unwrap()
            .into_iter()
            .find(|profile| profile.name == "gateway")
            .unwrap()
            .active
    );
    let account_text = fs::read_to_string(codex_home.join("config.toml")).unwrap();
    assert!(!account_text.contains("model_provider = \"prismux-gateway\""));
    assert!(account_text.contains("[model_providers.prismux-gateway]"));
    assert!(account_text.contains("[plugins.\"ponytail@ponytail\"]"));
}

#[test]
fn codex_account_switch_ignores_legacy_default_config_snapshot() {
    let temp = test_temp_dir("ignore-legacy-default-config");
    let codex_home = temp.join("codex-home");
    let state_root = temp.join("prismux-state");
    fs::create_dir_all(&codex_home).unwrap();
    fs::write(codex_home.join(AUTH_FILE_NAME), br#"{"account":"one"}"#).unwrap();
    fs::write(
        codex_home.join("config.toml"),
        "[plugins.\"ponytail@ponytail\"]\nenabled = true\n",
    )
    .unwrap();

    let plugin = CodexPlugin::with_paths(&codex_home, &state_root);
    plugin
        .save_current(SaveOptions {
            alias: Some("one".to_string()),
        })
        .unwrap();
    fs::write(codex_home.join(AUTH_FILE_NAME), br#"{"account":"two"}"#).unwrap();
    plugin
        .save_current(SaveOptions {
            alias: Some("two".to_string()),
        })
        .unwrap();
    let legacy = state_root.join("platforms/codex/configs/default.config.toml");
    fs::create_dir_all(legacy.parent().unwrap()).unwrap();
    fs::write(&legacy, "model = \"stale\"\n").unwrap();
    let expected = fs::read(codex_home.join("config.toml")).unwrap();

    plugin.use_target("one").unwrap();

    assert_eq!(fs::read(codex_home.join("config.toml")).unwrap(), expected);
}

#[test]
fn imports_codex_toml_gateway_config_as_profile_file() {
    let temp = test_temp_dir("import-codex-toml-config");
    let codex_home = temp.join("codex-home");
    let state_root = temp.join("prismux-state");
    let plugin = CodexPlugin::with_paths(&codex_home, &state_root);

    let imported = plugin
        .import_config(ImportConfigOptions {
            name: None,
            content: r#"
model_provider = "codex"
model = "gpt-5.5"
review_model = "gpt-5.5"
disable_response_storage = true

[model_providers.codex]
name = "codex"
base_url = "https://api.apikey.fun"
wire_api = "responses"
requires_openai_auth = true

[features]
goals = true
"#
            .to_string(),
        })
        .unwrap();

    assert_eq!(imported.profile_name, "api-apikey-fun");
    assert_eq!(imported.provider_id.as_deref(), Some("codex"));
    assert_eq!(imported.model.as_deref(), Some("gpt-5.5"));
    let profile = fs::read_to_string(codex_home.join("api-apikey-fun.config.toml")).unwrap();
    assert!(profile.contains("requires_openai_auth = true"));
    assert!(profile.contains("[features]"));

    let profiles = plugin.list_configs().unwrap();
    assert_eq!(profiles.len(), 1);
    assert_eq!(profiles[0].name, "api-apikey-fun");
    assert_eq!(profiles[0].provider_id.as_deref(), Some("codex"));
    assert_eq!(
        profiles[0].base_url.as_deref(),
        Some("https://api.apikey.fun")
    );
    assert_eq!(profiles[0].model.as_deref(), Some("gpt-5.5"));
    let live = fs::read_to_string(codex_home.join("config.toml")).unwrap();
    assert!(live.contains("[model_providers.prismux-api-apikey-fun]"));
    assert!(!live.contains("model_provider = \"prismux-api-apikey-fun\""));
}

#[test]
fn provider_switch_preserves_external_live_config_changes() {
    let temp = test_temp_dir("provider-preserves-live-config");
    let codex_home = temp.join("codex-home");
    let state_root = temp.join("prismux-state");
    fs::create_dir_all(&codex_home).unwrap();
    fs::write(
        codex_home.join("config.toml"),
        "# user config\n[plugins.\"ponytail@ponytail\"]\nenabled = true\n",
    )
    .unwrap();
    let plugin = CodexPlugin::with_paths(&codex_home, &state_root);
    plugin
        .import_config(ImportConfigOptions {
            name: Some("gateway".to_string()),
            content: "OPENAI_BASE_URL=https://original.example/v1 OPENAI_API_KEY=secret"
                .to_string(),
        })
        .unwrap();
    let config_path = codex_home.join("config.toml");
    let edited = fs::read_to_string(&config_path)
        .unwrap()
        .replace("https://original.example/v1", "https://edited.example/v1");
    fs::write(&config_path, edited).unwrap();

    plugin.use_target("gateway").unwrap();

    let live = fs::read_to_string(&config_path).unwrap();
    assert!(live.contains("# user config"));
    assert!(live.contains("[plugins.\"ponytail@ponytail\"]"));
    assert!(live.contains("base_url = \"https://edited.example/v1\""));
    assert!(live.contains("model_provider = \"prismux-gateway\""));
}

#[test]
fn provider_switch_rejects_concurrent_codex_config_change() {
    let temp = test_temp_dir("provider-concurrent-config-change");
    let codex_home = temp.join("codex-home");
    let state_root = temp.join("prismux-state");
    fs::create_dir_all(&codex_home).unwrap();
    let mut plugin = CodexPlugin::with_paths(&codex_home, &state_root);
    plugin
        .import_config(ImportConfigOptions {
            name: Some("gateway".to_string()),
            content: "OPENAI_BASE_URL=https://gateway.example/v1".to_string(),
        })
        .unwrap();
    plugin.set_before_config_replace(append_config_during_replace);

    let err = plugin.use_target("gateway").unwrap_err();

    assert!(
        err.to_string()
            .contains("config changed during provider switching")
    );
    assert!(
        fs::read_to_string(codex_home.join("config.toml"))
            .unwrap()
            .contains("# concurrent Codex App update")
    );
    assert!(!plugin.list_configs().unwrap()[0].active);
}

#[test]
fn remove_profile_refuses_externally_modified_provider_section() {
    let temp = test_temp_dir("remove-modified-provider");
    let codex_home = temp.join("codex-home");
    let state_root = temp.join("prismux-state");
    fs::create_dir_all(&codex_home).unwrap();
    let plugin = CodexPlugin::with_paths(&codex_home, &state_root);
    plugin
        .import_config(ImportConfigOptions {
            name: Some("gateway".to_string()),
            content: "OPENAI_BASE_URL=https://original.example/v1".to_string(),
        })
        .unwrap();
    let config_path = codex_home.join("config.toml");
    let edited = fs::read_to_string(&config_path)
        .unwrap()
        .replace("https://original.example/v1", "https://edited.example/v1");
    fs::write(&config_path, edited).unwrap();

    let err = plugin.remove_target("gateway").unwrap_err();

    assert!(err.to_string().contains("modified outside Prismux"));
    assert!(codex_home.join("gateway.config.toml").exists());
    assert_eq!(plugin.list_configs().unwrap().len(), 1);
}

#[test]
fn remove_profile_deletes_codex_profile_file() {
    let temp = test_temp_dir("remove-profile");
    let codex_home = temp.join("codex-home");
    let state_root = temp.join("prismux-state");
    fs::create_dir_all(&codex_home).unwrap();
    let plugin = CodexPlugin::with_paths(&codex_home, &state_root);
    plugin
        .import_config(ImportConfigOptions {
            name: Some("gateway".to_string()),
            content: "OPENAI_BASE_URL=https://gateway.example.com OPENAI_API_KEY=sk-test"
                .to_string(),
        })
        .unwrap();
    let profile_path = codex_home.join("gateway.config.toml");
    assert!(profile_path.exists());

    let report = plugin.remove_target("gateway").unwrap();

    let RemoveReport::Config(report) = report else {
        panic!("expected profile removal");
    };
    assert_eq!(report.profile.name, "gateway");
    assert!(!profile_path.exists());
    assert!(plugin.list_configs().unwrap().is_empty());
    let live = fs::read_to_string(codex_home.join("config.toml")).unwrap();
    assert!(!live.contains("prismux-gateway"));
}

#[test]
fn imports_openai_compatible_kv_without_storing_raw_api_key() {
    let temp = test_temp_dir("import-codex-kv-config");
    let codex_home = temp.join("codex-home");
    let state_root = temp.join("prismux-state");
    let plugin = CodexPlugin::with_paths(&codex_home, &state_root);

    let imported = plugin
        .import_config(ImportConfigOptions {
            name: Some("api-key-fun".to_string()),
            content: r#"
export OPENAI_API_KEY=sk-secret
OPENAI_BASE_URL=https://api.apikey.fun/v1
OPENAI_MODEL=gpt-5.5
"#
            .to_string(),
        })
        .unwrap();

    assert_eq!(imported.profile_name, "api-key-fun");
    assert_eq!(imported.provider_id.as_deref(), Some("api-key-fun"));
    let profile = fs::read_to_string(codex_home.join("api-key-fun.config.toml")).unwrap();
    assert!(profile.contains("env_key = \"OPENAI_API_KEY\""));
    assert!(profile.contains("base_url = \"https://api.apikey.fun/v1\""));
    assert!(!profile.contains("sk-secret"));
}

#[test]
fn sha256_matches_known_vector() {
    assert_eq!(
        sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn extracts_account_and_plan_from_codex_id_token_like_official_codex() {
    let token = fake_jwt(
        r#"{"alg":"none"}"#,
        r#"{"https://api.openai.com/profile":{"email":"profile@example.com"},"https://api.openai.com/auth":{"chatgpt_plan_type":"pro","chatgpt_user_id":"user-123","chatgpt_account_id":"account-456"}}"#,
    );
    let auth = format!(r#"{{"tokens":{{"id_token":"{token}","account_id":"fallback-account"}}}}"#);

    let metadata = extract_codex_account_metadata(auth.as_bytes());
    assert_eq!(
        metadata.account_label.as_deref(),
        Some("profile@example.com")
    );
    assert_eq!(metadata.plan_label.as_deref(), Some("Pro"));
    assert_eq!(
        metadata
            .provider_subject
            .as_ref()
            .map(|subject| subject.kind.as_str()),
        Some("codex_chatgpt_account")
    );
}

#[test]
fn extracts_account_from_account_id_when_jwt_has_no_account_claims() {
    let auth = br#"{"tokens":{"account_id":"account-456"}}"#;
    let metadata = extract_codex_account_metadata(auth);
    assert_eq!(metadata.account_label.as_deref(), Some("account-456"));
    assert_eq!(metadata.plan_label, None);
    assert_eq!(
        metadata
            .provider_subject
            .as_ref()
            .map(|subject| subject.kind.as_str()),
        Some("codex_account_id")
    );
}

#[test]
fn parses_codex_usage_auth_without_exposing_tokens() {
    let auth = br#"{"tokens":{"access_token":"access-secret","account_id":"account-456"}}"#;
    assert_eq!(
        parse_codex_usage_auth(auth),
        Some(CodexUsageAuth {
            access_token: "access-secret".to_string(),
            account_id: "account-456".to_string(),
            fedramp: false,
        })
    );
}

#[test]
fn parses_codex_usage_auth_fedramp_claim() {
    let token = fake_jwt(
        r#"{"alg":"none"}"#,
        r#"{"https://api.openai.com/auth":{"chatgpt_account_is_fedramp":true}}"#,
    );
    let auth = format!(r#"{{"tokens":{{"access_token":"{token}","account_id":"account-456"}}}}"#);

    assert_eq!(
        parse_codex_usage_auth(auth.as_bytes()),
        Some(CodexUsageAuth {
            access_token: token,
            account_id: "account-456".to_string(),
            fedramp: true,
        })
    );
}

#[test]
fn parses_curl_http_output_status_without_touching_body() {
    let output = br#"{"ok":true}
200"#;

    let (status, body) = parse_curl_http_output(output).unwrap();
    assert_eq!(status, 200);
    assert_eq!(body, br#"{"ok":true}"#);
}

#[test]
fn parses_curl_http_output_error_status() {
    let output = br#"{"error":{"code":"rate_limited"}}
429"#;

    let (status, body) = parse_curl_http_output(output).unwrap();
    assert_eq!(status, 429);
    assert_eq!(body, br#"{"error":{"code":"rate_limited"}}"#);
}

#[test]
fn parses_codex_usage_windows_as_structured_limits() {
    let payload = serde_json::json!({
        "rate_limit": {
            "primary_window": {
                "used_percent": 42,
                "limit_window_seconds": 18000,
                "reset_at": 1_725_000_000
            },
            "secondary_window": {
                "used_percent": 81,
                "limit_window_seconds": 604800
            }
        }
    });

    let usage = parse_codex_usage_snapshot(&payload, 1_785_000_000).unwrap();
    assert_eq!(usage.summary.display, "19%");
    assert_eq!(usage.summary.state, AvailabilityState::Limited);
    assert_eq!(usage.refreshed_at_unix, Some(1_785_000_000));
    assert_eq!(usage.limits.len(), 2);
    assert_eq!(usage.limits[0].label, "5h");
    assert_eq!(usage.limits[0].remaining_percent_x100, Some(5_800));
    assert_eq!(usage.limits[0].reset_at_unix, Some(1_725_000_000));
    assert_eq!(usage.limits[1].label, "weekly");
    assert_eq!(usage.limits[1].remaining_percent_x100, Some(1_900));
}

#[test]
fn parses_codex_usage_reset_credits() {
    let payload = serde_json::json!({
        "rate_limit": {
            "primary_window": {
                "used_percent": 42,
                "limit_window_seconds": 18000
            }
        },
        "rate_limit_reset_credits": {
            "available_count": 2
        }
    });

    let usage = parse_codex_usage_snapshot(&payload, 1_785_000_000).unwrap();

    assert_eq!(
        usage.reset_credits,
        Some(UsageResetCredits {
            available_count: 2,
            credits: Vec::new(),
        })
    );
}

#[test]
fn parses_codex_usage_reset_credits_from_string_count() {
    let payload = serde_json::json!({
        "rate_limit": {
            "primary_window": {
                "used_percent": 42,
                "limit_window_seconds": 18000
            }
        },
        "rate_limit_reset_credits": {
            "available_count": "3"
        }
    });

    let usage = parse_codex_usage_snapshot(&payload, 1_785_000_000).unwrap();

    assert_eq!(
        usage.reset_credits,
        Some(UsageResetCredits {
            available_count: 3,
            credits: Vec::new(),
        })
    );
}

#[test]
fn omits_codex_usage_reset_credits_when_missing_or_invalid() {
    for available_count in [serde_json::Value::Null, serde_json::json!("soon")] {
        let payload = serde_json::json!({
            "rate_limit": {
                "primary_window": {
                    "used_percent": 42,
                    "limit_window_seconds": 18000
                }
            },
            "rate_limit_reset_credits": {
                "available_count": available_count
            }
        });

        let usage = parse_codex_usage_snapshot(&payload, 1_785_000_000).unwrap();
        assert_eq!(usage.reset_credits, None);
    }
}

#[test]
fn parses_codex_reset_credit_detail_expiries() {
    let payload = serde_json::json!({
        "available_count": 2,
        "credits": [
            {
                "id": "redacted-2",
                "status": "available",
                "reset_type": "codex_rate_limits",
                "granted_at": "2026-06-27T00:01:21.691005Z",
                "expires_at": "2026-07-27T00:01:21.691005Z",
                "redeemed_at": null
            },
            {
                "id": "redacted-1",
                "status": "available",
                "reset_type": "codex_rate_limits",
                "granted_at": "2026-06-18T00:27:47.174188Z",
                "expires_at": "2026-07-18T00:27:47.174188Z",
                "redeemed_at": null
            }
        ]
    });

    let credits = parse_codex_reset_credit_details(&payload);

    assert_eq!(credits.len(), 2);
    assert_eq!(
        credits[0].expires_at_unix,
        rfc3339_unix("2026-07-18T00:27:47.174188Z")
    );
    assert_eq!(
        credits[1].expires_at_unix,
        rfc3339_unix("2026-07-27T00:01:21.691005Z")
    );
    assert_eq!(credits[0].status.as_deref(), Some("available"));
    assert_eq!(credits[0].reset_type.as_deref(), Some("codex_rate_limits"));
    assert_eq!(
        credits[0].granted_at_unix,
        rfc3339_unix("2026-06-18T00:27:47.174188Z")
    );
}

#[test]
fn reset_credit_detail_parser_ignores_unavailable_or_malformed_entries() {
    let payload = serde_json::json!({
        "credits": [
            {
                "status": "redeemed",
                "expires_at": "2026-07-18T00:27:47.174188Z"
            },
            {
                "status": "available",
                "expires_at": null
            },
            {
                "status": "available",
                "expires_at": "soon"
            },
            {
                "status": "available",
                "expires_at": "2026-07-27T00:01:21.691005Z"
            }
        ]
    });

    let credits = parse_codex_reset_credit_details(&payload);

    assert_eq!(credits.len(), 1);
    assert_eq!(
        credits[0].expires_at_unix,
        rfc3339_unix("2026-07-27T00:01:21.691005Z")
    );
}

#[test]
fn reset_credit_detail_parser_handles_empty_or_missing_credits() {
    assert!(parse_codex_reset_credit_details(&serde_json::json!({})).is_empty());
    assert!(parse_codex_reset_credit_details(&serde_json::json!({ "credits": [] })).is_empty());
}

#[test]
fn refresh_keeps_usage_snapshot_when_reset_credit_detail_fails() {
    let temp = test_temp_dir("reset-credit-detail-fallback");
    let codex_home = temp.join("codex-home");
    let state_root = temp.join("prismux-state");
    fs::create_dir_all(&codex_home).unwrap();
    fs::write(
        codex_home.join(AUTH_FILE_NAME),
        codex_auth_with_usage_token("work@example.com", "pro", "user-1", "account-1", 1),
    )
    .unwrap();

    let mut plugin = CodexPlugin::with_paths(&codex_home, &state_root);
    plugin
        .save_current(SaveOptions {
            alias: Some("work".to_string()),
        })
        .unwrap();
    plugin.set_usage_payload(Ok(serde_json::json!({
        "rate_limit": {
            "primary_window": {
                "used_percent": 42,
                "limit_window_seconds": 18000,
                "reset_at": 1_785_018_000
            }
        },
        "rate_limit_reset_credits": {
            "available_count": 2
        }
    })));
    plugin.set_reset_credit_detail_payload(Err(UsageDiagnostic {
        code: "network".to_string(),
        message: "reset credit expiry unavailable".to_string(),
    }));

    let status = plugin.refresh_account("work").unwrap();
    let usage = status.usage.unwrap();

    assert_eq!(usage.summary.display, "58%");
    assert_eq!(usage.reset_credits.as_ref().unwrap().available_count, 2);
    assert!(usage.reset_credits.as_ref().unwrap().credits.is_empty());
    assert!(usage.diagnostics.is_empty());
}

#[test]
fn parses_reset_credit_outcome_codes() {
    assert_eq!(
        parse_reset_credit_outcome(&serde_json::json!({
            "code": "reset",
            "windows_reset": 2
        }))
        .unwrap(),
        ResetCreditOutcome::Reset { windows_reset: 2 }
    );
    assert_eq!(
        parse_reset_credit_outcome(&serde_json::json!({
            "code": "reset",
            "windows_reset": "3"
        }))
        .unwrap(),
        ResetCreditOutcome::Reset { windows_reset: 3 }
    );
    assert_eq!(
        parse_reset_credit_outcome(&serde_json::json!({ "code": "nothing_to_reset" })).unwrap(),
        ResetCreditOutcome::NothingToReset
    );
    assert_eq!(
        parse_reset_credit_outcome(&serde_json::json!({ "code": "no_credit" })).unwrap(),
        ResetCreditOutcome::NoCredit
    );
    assert_eq!(
        parse_reset_credit_outcome(&serde_json::json!({ "code": "already_redeemed" })).unwrap(),
        ResetCreditOutcome::AlreadyRedeemed
    );
}

#[test]
fn rejects_unknown_reset_credit_outcome_code() {
    let err = parse_reset_credit_outcome(&serde_json::json!({ "code": "surprise" })).unwrap_err();
    assert!(
        err.to_string()
            .contains("unknown Codex reset credit response code")
    );

    let err = parse_reset_credit_outcome(&serde_json::json!({})).unwrap_err();
    assert!(err.to_string().contains("did not include a code"));
}

#[test]
fn list_accounts_uses_cached_usage_without_refreshing_remote_quota() {
    let temp = test_temp_dir("usage-refresh-fallback");
    let codex_home = temp.join("codex-home");
    let state_root = temp.join("prismux-state");
    fs::create_dir_all(&codex_home).unwrap();
    fs::write(codex_home.join(AUTH_FILE_NAME), br#"{"account":"work"}"#).unwrap();

    let plugin = CodexPlugin::with_paths(&codex_home, &state_root);
    plugin
        .save_current(SaveOptions {
            alias: Some("work".to_string()),
        })
        .unwrap();
    let store = plugin.state_store().unwrap();
    let account = store.list_accounts(plugin.id()).unwrap().remove(0);
    store
        .save_quota_snapshot(
            &account.local_id,
            plugin.id(),
            &UsageSnapshot {
                source: UsageSource::RemoteApi,
                refreshed_at_unix: Some(1_785_000_000),
                summary: Availability {
                    state: AvailabilityState::Available,
                    display: "72%".to_string(),
                },
                limits: vec![UsageLimit {
                    id: "five-hour".to_string(),
                    label: "5h".to_string(),
                    scope: UsageLimitScope::Account,
                    kind: UsageLimitKind::RollingWindow,
                    window_seconds: Some(18_000),
                    used_percent_x100: Some(2_800),
                    remaining_percent_x100: Some(7_200),
                    reset_at_unix: Some(1_785_018_000),
                    exhausted: Some(false),
                    raw_provider_key: None,
                }],
                reset_credits: None,
                diagnostics: Vec::new(),
            },
        )
        .unwrap();

    let status = plugin.list_accounts().unwrap().remove(0);
    let usage = status.usage.unwrap();

    assert_eq!(usage.source, UsageSource::StoredSnapshot);
    assert_eq!(usage.refreshed_at_unix, Some(1_785_000_000));
    assert_eq!(usage.limits[0].remaining_percent_x100, Some(7_200));
    assert!(usage.diagnostics.is_empty());
}

#[test]
fn refresh_accounts_refreshes_usage_for_every_saved_account() {
    let temp = test_temp_dir("refresh-all-usage");
    let codex_home = temp.join("codex-home");
    let state_root = temp.join("prismux-state");
    fs::create_dir_all(&codex_home).unwrap();
    let plugin = CodexPlugin::with_paths(&codex_home, &state_root);

    fs::write(codex_home.join(AUTH_FILE_NAME), br#"{"account":"first"}"#).unwrap();
    plugin
        .save_current(SaveOptions {
            alias: Some("first".to_string()),
        })
        .unwrap();

    fs::write(codex_home.join(AUTH_FILE_NAME), br#"{"account":"second"}"#).unwrap();
    plugin
        .save_current(SaveOptions {
            alias: Some("second".to_string()),
        })
        .unwrap();

    plugin.switch_to("1").unwrap();

    let accounts = plugin.refresh_accounts().unwrap();

    assert_eq!(accounts.len(), 2);
    assert!(accounts[0].active);
    assert!(!accounts[1].active);
    assert_eq!(
        accounts[0].usage.as_ref().unwrap().diagnostics[0].code,
        "managed_runtime_auth"
    );
    assert_eq!(
        accounts[1].usage.as_ref().unwrap().diagnostics[0].code,
        "managed_runtime_auth"
    );
}

#[test]
fn refresh_inactive_account_uses_managed_runtime_without_changing_active_auth() {
    let temp = test_temp_dir("refresh-inactive-managed-runtime");
    let codex_home = temp.join("codex-home");
    let state_root = temp.join("prismux-state");
    fs::create_dir_all(&codex_home).unwrap();
    let plugin = CodexPlugin::with_paths(&codex_home, &state_root);

    let first_auth = br#"{"account":"first"}"#;
    let second_auth = br#"{"account":"second"}"#;
    fs::write(codex_home.join(AUTH_FILE_NAME), first_auth).unwrap();
    plugin
        .save_current(SaveOptions {
            alias: Some("first".to_string()),
        })
        .unwrap();
    fs::write(codex_home.join(AUTH_FILE_NAME), second_auth).unwrap();
    plugin
        .save_current(SaveOptions {
            alias: Some("second".to_string()),
        })
        .unwrap();
    plugin.switch_to("1").unwrap();
    let active_before = fs::read(codex_home.join(AUTH_FILE_NAME)).unwrap();
    let second = plugin
        .state_store()
        .unwrap()
        .account_by_selector(plugin.id(), "2")
        .unwrap()
        .unwrap();
    let second_snapshot = PathBuf::from(&second.secret_ref);

    let accounts = plugin.refresh_accounts().unwrap();

    assert_eq!(
        fs::read(codex_home.join(AUTH_FILE_NAME)).unwrap(),
        active_before
    );
    assert!(accounts[0].active);
    assert!(!accounts[1].active);
    assert_eq!(
        fs::read(plugin.managed_runtime_auth_path(&second).unwrap()).unwrap(),
        second_auth
    );
    assert_eq!(fs::read(second_snapshot).unwrap(), second_auth);
}

#[test]
fn inactive_managed_token_refresh_stays_in_managed_scope() {
    let temp = test_temp_dir("managed-token-refresh-scope");
    let codex_home = temp.join("codex-home");
    let state_root = temp.join("prismux-state");
    fs::create_dir_all(&codex_home).unwrap();
    let plugin = CodexPlugin::with_paths(&codex_home, &state_root);

    let first_auth = br#"{"account":"first"}"#;
    let second_snapshot_auth = br#"{"account":"second","refresh":"old"}"#;
    let second_runtime_auth = br#"{"account":"second","refresh":"new"}"#;
    fs::write(codex_home.join(AUTH_FILE_NAME), first_auth).unwrap();
    plugin
        .save_current(SaveOptions {
            alias: Some("first".to_string()),
        })
        .unwrap();
    fs::write(codex_home.join(AUTH_FILE_NAME), second_snapshot_auth).unwrap();
    plugin
        .save_current(SaveOptions {
            alias: Some("second".to_string()),
        })
        .unwrap();
    plugin.switch_to("1").unwrap();
    let active_before = fs::read(codex_home.join(AUTH_FILE_NAME)).unwrap();
    let second = plugin
        .state_store()
        .unwrap()
        .account_by_selector(plugin.id(), "2")
        .unwrap()
        .unwrap();
    let managed_auth_path = plugin.managed_runtime_auth_path(&second).unwrap();
    fs::create_dir_all(managed_auth_path.parent().unwrap()).unwrap();
    fs::write(&managed_auth_path, second_runtime_auth).unwrap();

    plugin.refresh_accounts().unwrap();

    assert_eq!(
        fs::read(codex_home.join(AUTH_FILE_NAME)).unwrap(),
        active_before
    );
    assert_eq!(
        fs::read(PathBuf::from(&second.secret_ref)).unwrap(),
        second_snapshot_auth
    );
    assert_eq!(fs::read(managed_auth_path).unwrap(), second_runtime_auth);
}

#[test]
fn summarizes_known_account_availability_by_tightest_remaining_capacity() {
    let summary = summarize_availability(vec![
        Availability {
            state: AvailabilityState::Available,
            display: "80%".to_string(),
        },
        Availability {
            state: AvailabilityState::Limited,
            display: "20%".to_string(),
        },
    ]);

    assert_eq!(summary.display, "20%");
    assert_eq!(summary.state, AvailabilityState::Limited);
}

#[cfg(unix)]
fn fake_codex_executable(temp: &Path) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;

    let script = temp.join("codex");
    fs::write(
        &script,
        r#"#!/bin/sh
set -eu
count_file="$0.count"
args_file="$0.args"
count=0
if [ -f "$count_file" ]; then
  count="$(cat "$count_file")"
fi
count=$((count + 1))
printf '%s' "$count" > "$count_file"
printf '%s\n' "$*" >> "$args_file"
mkdir -p "$CODEX_HOME"
printf '{"account":"%s"}' "$count" > "$CODEX_HOME/auth.json"
"#,
    )
    .unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    script
}

#[cfg(unix)]
fn fake_codex_rotating_same_account_executable(
    temp: &Path,
    first_auth: &str,
    refreshed_auth: &str,
) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;

    let script = temp.join("codex-rotating-same-account");
    let content = format!(
        r#"#!/bin/sh
set -eu
count_file="$0.count"
count=0
if [ -f "$count_file" ]; then
  count="$(cat "$count_file")"
fi
count=$((count + 1))
printf '%s' "$count" > "$count_file"
mkdir -p "$CODEX_HOME"
if [ "$count" -eq 1 ]; then
  printf '%s' '{first_auth}' > "$CODEX_HOME/auth.json"
else
  printf '%s' '{refreshed_auth}' > "$CODEX_HOME/auth.json"
fi
"#
    );
    fs::write(&script, content).unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    script
}

#[cfg(unix)]
fn fake_codex_static_executable(temp: &Path) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;

    let script = temp.join("codex-static");
    fs::write(
        &script,
        r#"#!/bin/sh
set -eu
mkdir -p "$CODEX_HOME"
printf '{"account":"same"}' > "$CODEX_HOME/auth.json"
"#,
    )
    .unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    script
}

fn fake_jwt(header: &str, payload: &str) -> String {
    format!(
        "{}.{}.signature",
        base64_url_encode(header.as_bytes()),
        base64_url_encode(payload.as_bytes())
    )
}

fn overwrite_auth_during_replace(auth_path: &Path) {
    fs::write(auth_path, br#"{"account":"concurrent"}"#).unwrap();
}

fn append_config_during_replace(config_path: &Path) {
    let mut config = fs::read_to_string(config_path).unwrap();
    config.push_str("\n# concurrent Codex App update\n");
    fs::write(config_path, config).unwrap();
}

fn remove_auth_during_replace(auth_path: &Path) {
    fs::remove_file(auth_path).unwrap();
}

fn codex_auth_with_claims(
    email: &str,
    plan: &str,
    user_id: &str,
    account_id: &str,
    nonce: u32,
) -> String {
    let token = fake_jwt(
        r#"{"alg":"none"}"#,
        &format!(
            r#"{{"iss":"https://auth.openai.com","sub":"{user_id}","https://api.openai.com/profile":{{"email":"{email}"}},"https://api.openai.com/auth":{{"chatgpt_plan_type":"{plan}","chatgpt_user_id":"{user_id}","chatgpt_account_id":"{account_id}"}},"nonce":{nonce}}}"#
        ),
    );
    format!(r#"{{"tokens":{{"id_token":"{token}","account_id":"{account_id}"}}}}"#)
}

fn codex_auth_with_usage_token(
    email: &str,
    plan: &str,
    user_id: &str,
    account_id: &str,
    nonce: u32,
) -> String {
    let id_token = fake_jwt(
        r#"{"alg":"none"}"#,
        &format!(
            r#"{{"iss":"https://auth.openai.com","sub":"{user_id}","https://api.openai.com/profile":{{"email":"{email}"}},"https://api.openai.com/auth":{{"chatgpt_plan_type":"{plan}","chatgpt_user_id":"{user_id}","chatgpt_account_id":"{account_id}"}},"nonce":{nonce}}}"#
        ),
    );
    format!(
        r#"{{"tokens":{{"id_token":"{id_token}","access_token":"test-access-token","account_id":"{account_id}"}}}}"#
    )
}

/// Build a Codex auth.json whose access token is a (fake) JWT carrying `exp`,
/// with a refresh token, so `ensure_fresh_auth` can decide expiry.
fn codex_auth_with_expiring_token(exp: i64, refresh: &str, account_id: &str) -> String {
    let access = fake_jwt(r#"{"alg":"none"}"#, &format!(r#"{{"exp":{exp}}}"#));
    format!(
        r#"{{"tokens":{{"access_token":"{access}","refresh_token":"{refresh}","account_id":"{account_id}"}}}}"#
    )
}

#[test]
fn refresh_mints_new_token_when_access_token_expired() {
    let temp = test_temp_dir("codex-token-refresh");
    let codex_home = temp.join("codex-home");
    let state_root = temp.join("prismux-state");
    fs::create_dir_all(&codex_home).unwrap();
    let expired = unix_now() as i64 - 10;
    fs::write(
        codex_home.join(AUTH_FILE_NAME),
        codex_auth_with_expiring_token(expired, "refresh-old", "account-1"),
    )
    .unwrap();

    let mut plugin = CodexPlugin::with_paths(&codex_home, &state_root);
    plugin
        .save_current(SaveOptions {
            alias: Some("work".to_string()),
        })
        .unwrap();

    let fresh = unix_now() as i64 + 3600;
    let new_access = fake_jwt(r#"{"alg":"none"}"#, &format!(r#"{{"exp":{fresh}}}"#));
    plugin.set_oauth_refresh_payload(Ok(serde_json::json!({
        "access_token": new_access,
        "refresh_token": "refresh-new",
        "id_token": "id-new",
    })));
    plugin.set_usage_payload(Ok(serde_json::json!({
        "rate_limit": {
            "primary_window": {
                "used_percent": 10,
                "limit_window_seconds": 18000,
                "reset_at": 1_785_018_000
            }
        }
    })));

    let status = plugin.refresh_account("work").unwrap();
    let usage = status.usage.unwrap();
    assert!(
        usage.diagnostics.is_empty(),
        "unexpected: {:?}",
        usage.diagnostics
    );

    let account = plugin
        .state_store()
        .unwrap()
        .account_by_selector(plugin.id(), "work")
        .unwrap()
        .unwrap();
    // Rotated tokens land in the runtime scope...
    let runtime: serde_json::Value = serde_json::from_slice(
        &fs::read(plugin.managed_runtime_auth_path(&account).unwrap()).unwrap(),
    )
    .unwrap();
    assert_eq!(
        runtime.pointer("/tokens/refresh_token").unwrap(),
        "refresh-new"
    );
    // ...while the immutable import-time snapshot keeps its original identity.
    let snapshot: serde_json::Value =
        serde_json::from_slice(&fs::read(&account.secret_ref).unwrap()).unwrap();
    assert_eq!(
        snapshot.pointer("/tokens/refresh_token").unwrap(),
        "refresh-old"
    );
}

#[test]
fn expired_runtime_adopts_fresher_snapshot_without_network_refresh() {
    let temp = test_temp_dir("codex-adopt-snapshot");
    let codex_home = temp.join("codex-home");
    let state_root = temp.join("prismux-state");
    fs::create_dir_all(&codex_home).unwrap();
    // Snapshot carries a still-valid token; the stale runtime copy below is
    // older. Adoption must avoid spending the refresh_token.
    let valid = unix_now() as i64 + 3600;
    fs::write(
        codex_home.join(AUTH_FILE_NAME),
        codex_auth_with_expiring_token(valid, "refresh-snapshot", "account-1"),
    )
    .unwrap();

    let mut plugin = CodexPlugin::with_paths(&codex_home, &state_root);
    plugin
        .save_current(SaveOptions {
            alias: Some("work".to_string()),
        })
        .unwrap();

    // Freeze an expired runtime copy that predates the snapshot.
    let account = plugin
        .state_store()
        .unwrap()
        .account_by_selector(plugin.id(), "work")
        .unwrap()
        .unwrap();
    let runtime_path = plugin.managed_runtime_auth_path(&account).unwrap();
    fs::create_dir_all(runtime_path.parent().unwrap()).unwrap();
    let stale = unix_now() as i64 - 10;
    fs::write(
        &runtime_path,
        codex_auth_with_expiring_token(stale, "refresh-runtime", "account-1"),
    )
    .unwrap();

    // A network refresh here would panic (no mock set); adoption must not call it.
    plugin.set_usage_payload(Ok(serde_json::json!({
        "rate_limit": {
            "primary_window": {
                "used_percent": 5,
                "limit_window_seconds": 18000,
                "reset_at": 1_785_018_000
            }
        }
    })));

    let status = plugin.refresh_account("work").unwrap();
    assert!(status.usage.unwrap().diagnostics.is_empty());

    let runtime: serde_json::Value =
        serde_json::from_slice(&fs::read(&runtime_path).unwrap()).unwrap();
    assert_eq!(
        runtime.pointer("/tokens/refresh_token").unwrap(),
        "refresh-snapshot",
        "runtime should have adopted the fresher snapshot token"
    );
}

fn base64_url_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut output = String::new();
    let mut index = 0;
    while index < bytes.len() {
        let first = bytes[index];
        let second = bytes.get(index + 1).copied();
        let third = bytes.get(index + 2).copied();

        output.push(TABLE[(first >> 2) as usize] as char);
        output.push(
            TABLE[(((first & 0b0000_0011) << 4) | second.unwrap_or(0) >> 4) as usize] as char,
        );
        if let Some(second) = second {
            output.push(
                TABLE[(((second & 0b0000_1111) << 2) | third.unwrap_or(0) >> 6) as usize] as char,
            );
        }
        if let Some(third) = third {
            output.push(TABLE[(third & 0b0011_1111) as usize] as char);
        }

        index += 3;
    }
    output
}

fn test_temp_dir(name: &str) -> PathBuf {
    let path = env::temp_dir().join(format!("prismux-test-{name}-{}", unix_now_nanos()));
    fs::create_dir_all(&path).unwrap();
    path
}
