use super::*;

#[test]
fn imports_lists_and_uses_claude_profile_without_leaking_registry_secret() {
    let temp = test_temp_dir("claude-profile");
    let claude_home = temp.join("claude-home");
    let state_root = temp.join("openmux-state");
    fs::create_dir_all(&claude_home).unwrap();
    fs::write(
            claude_home.join(SETTINGS_FILE_NAME),
            br#"{"permissions":{"allow":["Bash(ls)"]},"env":{"ANTHROPIC_BASE_URL":"old","OTHER":"keep"}}"#,
        )
        .unwrap();
    let plugin = ClaudePlugin::with_paths(&claude_home, &state_root);

    let imported = plugin
            .import_config(ImportConfigOptions {
                name: Some("gateway-work".to_string()),
                content: "ANTHROPIC_BASE_URL=https://gateway.example.com ANTHROPIC_AUTH_TOKEN=secret ANTHROPIC_MODEL=sonnet".to_string(),
            })
            .unwrap();

    assert_eq!(imported.number, Some(1));
    assert_eq!(imported.auth_type.as_deref(), Some("bearer-token"));
    let registry = fs::read_to_string(plugin.registry_path().unwrap()).unwrap();
    assert!(!registry.contains("secret"));

    let profiles = plugin.list_configs().unwrap();
    assert_eq!(profiles.len(), 1);
    assert_eq!(profiles[0].number, Some(1));
    assert_eq!(profiles[0].auth_type.as_deref(), Some("bearer-token"));

    plugin.use_target("gateway-work").unwrap();
    let settings: Value =
        serde_json::from_slice(&fs::read(claude_home.join(SETTINGS_FILE_NAME)).unwrap()).unwrap();
    assert_eq!(
        settings
            .pointer("/permissions/allow/0")
            .and_then(Value::as_str),
        Some("Bash(ls)")
    );
    assert_eq!(
        settings
            .pointer("/env/ANTHROPIC_BASE_URL")
            .and_then(Value::as_str),
        Some("https://gateway.example.com")
    );
    assert_eq!(
        settings.pointer("/env/OTHER").and_then(Value::as_str),
        Some("keep")
    );
    assert!(plugin.list_configs().unwrap()[0].active);
    let backups = fs::read_dir(plugin.backups_dir().unwrap())
        .unwrap()
        .collect::<std::result::Result<Vec<_>, _>>()
        .unwrap();
    assert!(!backups.is_empty());
}

#[test]
fn duplicate_claude_profile_updates_existing_number() {
    let temp = test_temp_dir("claude-duplicate");
    let plugin = ClaudePlugin::with_paths(temp.join("claude-home"), temp.join("openmux-state"));

    let first = plugin
        .import_config(ImportConfigOptions {
            name: Some("api-direct".to_string()),
            content: "ANTHROPIC_API_KEY=sk-test".to_string(),
        })
        .unwrap();
    let second = plugin
        .import_config(ImportConfigOptions {
            name: Some("api-direct".to_string()),
            content: "ANTHROPIC_API_KEY=sk-test-2".to_string(),
        })
        .unwrap();

    assert_eq!(first.number, Some(1));
    assert_eq!(second.number, Some(1));
    assert_eq!(plugin.list_configs().unwrap().len(), 1);
}

#[test]
fn use_claude_profile_reports_missing_selector() {
    let temp = test_temp_dir("claude-missing-profile");
    let plugin = ClaudePlugin::with_paths(temp.join("claude-home"), temp.join("openmux-state"));

    let err = plugin.use_target("missing").unwrap_err();

    assert!(err.to_string().contains("not found"));
}

#[test]
fn claude_oauth_account_methods_are_deferred() {
    let temp = test_temp_dir("claude-deferred");
    let plugin = ClaudePlugin::with_paths(temp.join("claude-home"), temp.join("openmux-state"));

    let err = plugin.login(LoginOptions::default()).unwrap_err();

    assert!(err.to_string().contains("deferred"));
}

#[cfg(unix)]
#[test]
fn login_runs_official_claude_cli_then_imports_account_snapshot() {
    let temp = test_temp_dir("claude-account-login");
    let claude_home = temp.join("claude-home");
    let state_root = temp.join("openmux-state");
    let fake_claude = fake_claude_login_executable(&temp);
    let plugin = ClaudeAccountPlugin::with_paths_and_claude_executable(
        &claude_home,
        &state_root,
        &fake_claude,
    );

    let account = plugin
        .login(LoginOptions {
            alias: Some("work".to_string()),
            activate: true,
            ..LoginOptions::default()
        })
        .unwrap();

    assert_eq!(account.number, 1);
    assert_eq!(account.alias.as_deref(), Some("work"));
    assert_eq!(plugin.current().unwrap().unwrap().account.number, 1);
    let args_log = fs::read_to_string(fake_claude.with_extension("args")).unwrap();
    assert_eq!(args_log.trim(), "auth login");
    let registry = fs::read_to_string(plugin.registry_path().unwrap()).unwrap();
    assert!(!registry.contains("login-access"));
    assert!(!registry.contains("login-refresh"));
}

#[test]
fn imports_and_switches_plaintext_claude_account_without_registry_token_leak() {
    let temp = test_temp_dir("claude-account");
    let claude_home = temp.join("claude-home");
    let state_root = temp.join("openmux-state");
    fs::create_dir_all(&claude_home).unwrap();
    fs::write(
            claude_home.join(".credentials.json"),
            br#"{"claudeAiOauth":{"accessToken":"access-secret","refreshToken":"refresh-secret","expiresAt":1781629000,"scopes":["user:inference"]}}"#,
        )
        .unwrap();
    fs::write(
            claude_home.join(SETTINGS_FILE_NAME),
            br#"{"oauthAccount":{"email":"person@example.com","accountUuid":"account-1","organizationUuid":"org-1"},"theme":"dark"}"#,
        )
        .unwrap();
    let plugin = ClaudeAccountPlugin::with_paths(&claude_home, &state_root);

    let imported = plugin
        .import_config(ImportConfigOptions {
            name: Some("work".to_string()),
            content: String::new(),
        })
        .unwrap();

    assert_eq!(imported.number, Some(1));
    let registry = fs::read_to_string(plugin.registry_path().unwrap()).unwrap();
    assert!(!registry.contains("access-secret"));
    assert!(!registry.contains("refresh-secret"));
    assert!(registry.contains("p***@example.com"));
    let accounts = plugin.list_accounts().unwrap();
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].account.alias.as_deref(), Some("work"));
    assert_eq!(accounts[0].auth_type.as_deref(), Some("oauth/full"));
    assert_eq!(accounts[0].expires_at_unix, Some(1_781_629_000));

    fs::write(
            claude_home.join(".credentials.json"),
            br#"{"claudeAiOauth":{"accessToken":"other","refreshToken":"other-refresh","expiresAt":1781620000,"scopes":["user:inference"]}}"#,
        )
        .unwrap();
    plugin.use_target("work").unwrap();
    let credentials = fs::read_to_string(claude_home.join(".credentials.json")).unwrap();
    assert!(credentials.contains("access-secret"));
    let backups = fs::read_dir(plugin.backups_dir().unwrap())
        .unwrap()
        .collect::<std::result::Result<Vec<_>, _>>()
        .unwrap();
    assert!(
        backups
            .iter()
            .any(|entry| entry.file_name().to_string_lossy().contains("credentials"))
    );
    assert!(
        backups
            .iter()
            .any(|entry| entry.file_name().to_string_lossy().contains("settings"))
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let credential_mode = fs::metadata(claude_home.join(".credentials.json"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        let snapshot_mode = fs::metadata(plugin.account_snapshot_path(1).unwrap())
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(credential_mode, 0o600);
        assert_eq!(snapshot_mode, 0o600);
    }
    let settings: Value =
        serde_json::from_slice(&fs::read(claude_home.join(SETTINGS_FILE_NAME)).unwrap()).unwrap();
    assert_eq!(
        settings.pointer("/theme").and_then(Value::as_str),
        Some("dark")
    );
    assert_eq!(
        settings
            .pointer("/oauthAccount/email")
            .and_then(Value::as_str),
        Some("person@example.com")
    );
}

#[test]
fn claude_account_and_profile_active_states_are_mutually_exclusive() {
    let temp = test_temp_dir("claude-active-mutual-exclusion");
    let claude_home = temp.join("claude-home");
    let state_root = temp.join("openmux-state");
    fs::create_dir_all(&claude_home).unwrap();
    fs::write(
            claude_home.join(".credentials.json"),
            br#"{"claudeAiOauth":{"accessToken":"access-secret","refreshToken":"refresh-secret","expiresAt":1781629000,"scopes":["user:inference"]}}"#,
        )
        .unwrap();
    fs::write(
            claude_home.join(SETTINGS_FILE_NAME),
            br#"{"oauthAccount":{"email":"person@example.com","accountUuid":"account-1","organizationUuid":"org-1"}}"#,
        )
        .unwrap();

    let account_plugin = ClaudeAccountPlugin::with_paths(&claude_home, &state_root);
    let profile_plugin = ClaudePlugin::with_paths(&claude_home, &state_root);
    account_plugin
        .import_config(ImportConfigOptions {
            name: Some("work".to_string()),
            content: String::new(),
        })
        .unwrap();
    account_plugin.use_target("work").unwrap();
    assert!(account_plugin.list_accounts().unwrap()[0].active);

    profile_plugin
        .import_config(ImportConfigOptions {
            name: Some("gateway".to_string()),
            content: "ANTHROPIC_BASE_URL=https://gateway.example.com ANTHROPIC_AUTH_TOKEN=secret"
                .to_string(),
        })
        .unwrap();
    profile_plugin.use_target("gateway").unwrap();

    assert!(account_plugin.current().unwrap().is_none());
    assert!(!account_plugin.list_accounts().unwrap()[0].active);
    assert!(profile_plugin.list_configs().unwrap()[0].active);

    account_plugin.use_target("work").unwrap();

    assert_eq!(
        account_plugin
            .current()
            .unwrap()
            .unwrap()
            .account
            .alias
            .as_deref(),
        Some("work")
    );
    assert!(account_plugin.list_accounts().unwrap()[0].active);
    assert!(!profile_plugin.list_configs().unwrap()[0].active);
}

#[cfg(unix)]
#[test]
fn claude_account_switch_rolls_back_credential_when_oauth_metadata_write_fails() {
    let temp = test_temp_dir("claude-account-settings-rollback");
    let claude_home = temp.join("claude-home");
    let state_root = temp.join("openmux-state");
    let keychain_path = temp.join("fake-keychain-secret.json");
    fs::create_dir_all(&claude_home).unwrap();
    fs::write(
        &keychain_path,
        br#"{"claudeAiOauth":{"accessToken":"target-access","refreshToken":"target-refresh","expiresAt":1781629000,"scopes":["user:inference"]}}"#,
    )
    .unwrap();
    fs::write(
        claude_home.join(SETTINGS_FILE_NAME),
        br#"{"oauthAccount":{"email":"target@example.com","accountUuid":"account-target","organizationUuid":"org-target"},"theme":"dark"}"#,
    )
    .unwrap();
    let plugin = ClaudeAccountPlugin::with_paths_and_fake_keychain_settings_write_failure(
        &claude_home,
        &state_root,
        &keychain_path,
    );
    plugin
        .import_config(ImportConfigOptions {
            name: Some("target".to_string()),
            content: String::new(),
        })
        .unwrap();

    fs::write(
        &keychain_path,
        br#"{"claudeAiOauth":{"accessToken":"current-access","refreshToken":"current-refresh","expiresAt":1781620000,"scopes":["user:inference"]}}"#,
    )
    .unwrap();

    let err = plugin.use_target("target").unwrap_err();

    assert!(err.to_string().contains("credential rollback attempted"));
    let restored = fs::read_to_string(&keychain_path).unwrap();
    assert!(restored.contains("current-access"));
    assert!(!restored.contains("target-access"));
}

#[test]
fn claude_account_switch_keeps_current_credential_when_backend_write_fails() {
    let temp = test_temp_dir("claude-account-credential-write-failure");
    let claude_home = temp.join("claude-home");
    let state_root = temp.join("openmux-state");
    let keychain_path = temp.join("fake-keychain-secret.json");
    fs::create_dir_all(&claude_home).unwrap();
    fs::write(
        &keychain_path,
        br#"{"claudeAiOauth":{"accessToken":"target-access","refreshToken":"target-refresh","expiresAt":1781629000,"scopes":["user:inference"]}}"#,
    )
    .unwrap();
    fs::write(
        claude_home.join(SETTINGS_FILE_NAME),
        br#"{"oauthAccount":{"email":"target@example.com","accountUuid":"account-target","organizationUuid":"org-target"}}"#,
    )
    .unwrap();
    let setup = ClaudeAccountPlugin::with_paths_and_fake_keychain(
        &claude_home,
        &state_root,
        &keychain_path,
    );
    setup
        .import_config(ImportConfigOptions {
            name: Some("target".to_string()),
            content: String::new(),
        })
        .unwrap();

    fs::write(
        &keychain_path,
        br#"{"claudeAiOauth":{"accessToken":"current-access","refreshToken":"current-refresh","expiresAt":1781620000,"scopes":["user:inference"]}}"#,
    )
    .unwrap();
    let plugin = ClaudeAccountPlugin::with_paths_and_fake_keychain_credential_write_failure(
        &claude_home,
        &state_root,
        &keychain_path,
    );

    let err = plugin.use_target("target").unwrap_err();

    assert!(err.to_string().contains("write failure"));
    let current = fs::read_to_string(&keychain_path).unwrap();
    assert!(current.contains("current-access"));
    assert!(!current.contains("target-access"));
}

#[test]
fn imports_and_switches_fake_keychain_claude_account() {
    let temp = test_temp_dir("claude-account-fake-keychain");
    let claude_home = temp.join("claude-home");
    let state_root = temp.join("openmux-state");
    let keychain_path = temp.join("fake-keychain-secret.json");
    fs::create_dir_all(&claude_home).unwrap();
    fs::write(
            &keychain_path,
            br#"{"claudeAiOauth":{"accessToken":"keychain-access","refreshToken":"keychain-refresh","expiresAt":1781629000,"scopes":["user:inference"]}}"#,
        )
        .unwrap();
    fs::write(
            claude_home.join(SETTINGS_FILE_NAME),
            br#"{"oauthAccount":{"email":"keychain@example.com","accountUuid":"account-2","organizationUuid":"org-2"}}"#,
        )
        .unwrap();
    let plugin = ClaudeAccountPlugin::with_paths_and_fake_keychain(
        &claude_home,
        &state_root,
        &keychain_path,
    );

    plugin
        .import_config(ImportConfigOptions {
            name: Some("keychain".to_string()),
            content: String::new(),
        })
        .unwrap();
    fs::write(
            &keychain_path,
            br#"{"claudeAiOauth":{"accessToken":"other","refreshToken":"other-refresh","expiresAt":1781620000}}"#,
        )
        .unwrap();
    plugin.use_target("keychain").unwrap();

    let restored = fs::read_to_string(&keychain_path).unwrap();
    assert!(restored.contains("keychain-access"));
    assert_eq!(
        plugin.list_accounts().unwrap()[0].account_label.as_deref(),
        Some("k***@example.com")
    );
}

#[test]
fn claude_account_import_rejects_incomplete_oauth_payload() {
    let temp = test_temp_dir("claude-account-incomplete");
    let claude_home = temp.join("claude-home");
    let state_root = temp.join("openmux-state");
    fs::create_dir_all(&claude_home).unwrap();
    fs::write(
        claude_home.join(".credentials.json"),
        br#"{"claudeAiOauth":{"accessToken":"access-secret"}}"#,
    )
    .unwrap();
    let plugin = ClaudeAccountPlugin::with_paths(&claude_home, &state_root);

    let err = plugin
        .import_config(ImportConfigOptions {
            name: None,
            content: String::new(),
        })
        .unwrap_err();

    assert!(err.to_string().contains("refresh token"));

    fs::write(
        claude_home.join(".credentials.json"),
        br#"{"claudeAiOauth":{"accessToken":"access-secret","refreshToken":"refresh-secret"}}"#,
    )
    .unwrap();
    let err = plugin
        .import_config(ImportConfigOptions {
            name: None,
            content: String::new(),
        })
        .unwrap_err();

    assert!(err.to_string().contains("expiresAt"));
}

#[test]
fn claude_account_switch_rejects_tampered_snapshot() {
    let temp = test_temp_dir("claude-account-tamper");
    let claude_home = temp.join("claude-home");
    let state_root = temp.join("openmux-state");
    fs::create_dir_all(&claude_home).unwrap();
    fs::write(
            claude_home.join(".credentials.json"),
            br#"{"claudeAiOauth":{"accessToken":"access-secret","refreshToken":"refresh-secret","expiresAt":1781629000,"scopes":["user:inference"]}}"#,
        )
        .unwrap();
    let plugin = ClaudeAccountPlugin::with_paths(&claude_home, &state_root);
    plugin
        .import_config(ImportConfigOptions {
            name: Some("work".to_string()),
            content: String::new(),
        })
        .unwrap();
    fs::write(
            plugin.account_snapshot_path(1).unwrap(),
            br#"{"claudeAiOauth":{"accessToken":"tampered","refreshToken":"refresh-secret","expiresAt":1781629000}}"#,
        )
        .unwrap();

    let err = plugin.use_target("1").unwrap_err();

    assert!(err.to_string().contains("hash verification"));
}

fn test_temp_dir(name: &str) -> PathBuf {
    let path = env::temp_dir().join(format!("openmux-test-{name}-{}", unix_now_nanos()));
    fs::create_dir_all(&path).unwrap();
    path
}

#[cfg(unix)]
fn fake_claude_login_executable(temp: &Path) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;

    let script = temp.join("claude");
    fs::write(
        &script,
        r#"#!/bin/sh
set -eu
printf '%s\n' "$*" > "$0.args"
mkdir -p "$CLAUDE_CONFIG_DIR"
printf '%s' '{"claudeAiOauth":{"accessToken":"login-access","refreshToken":"login-refresh","expiresAt":1781629000,"scopes":["user:inference"]}}' > "$CLAUDE_CONFIG_DIR/.credentials.json"
printf '%s' '{"oauthAccount":{"email":"login@example.com","accountUuid":"account-login","organizationUuid":"org-login"}}' > "$CLAUDE_CONFIG_DIR/settings.json"
"#,
    )
    .unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    script
}
