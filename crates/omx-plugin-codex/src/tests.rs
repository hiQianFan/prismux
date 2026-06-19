use super::*;

#[test]
fn saves_lists_and_switches_codex_auth_snapshots_by_number_and_alias() {
    let temp = test_temp_dir("save-switch");
    let codex_home = temp.join("codex-home");
    let state_root = temp.join("openmux-state");
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
    let state_root = temp.join("openmux-state");
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
fn remove_account_deletes_snapshot_and_excludes_from_list() {
    let temp = test_temp_dir("remove-account");
    let codex_home = temp.join("codex-home");
    let state_root = temp.join("openmux-state");
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
    let state_root = temp.join("openmux-state");
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
    let state_root = temp.join("openmux-state");
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
    let state_root = temp.join("openmux-state");
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

#[test]
fn alias_set_rejects_all_digit_aliases() {
    let temp = test_temp_dir("alias");
    let codex_home = temp.join("codex-home");
    let state_root = temp.join("openmux-state");
    fs::create_dir_all(&codex_home).unwrap();
    fs::write(codex_home.join(AUTH_FILE_NAME), br#"{"account":"work"}"#).unwrap();

    let plugin = CodexPlugin::with_paths(&codex_home, &state_root);
    plugin.save_current(SaveOptions::default()).unwrap();
    let err = plugin.set_alias("1", "123").unwrap_err();
    assert!(err.to_string().contains("all digits"));
}

#[test]
fn save_rejects_alias_used_by_another_account() {
    let temp = test_temp_dir("duplicate-alias");
    let codex_home = temp.join("codex-home");
    let state_root = temp.join("openmux-state");
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
    let state_root = temp.join("openmux-state");
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
    let state_root = temp.join("openmux-state");
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
fn codex_account_and_profile_active_states_are_mutually_exclusive() {
    let temp = test_temp_dir("active-mutual-exclusion");
    let codex_home = temp.join("codex-home");
    let state_root = temp.join("openmux-state");
    fs::create_dir_all(&codex_home).unwrap();
    fs::write(codex_home.join(AUTH_FILE_NAME), br#"{"account":"work"}"#).unwrap();
    fs::write(codex_home.join("config.toml"), "model = \"default\"\n").unwrap();

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
    assert_eq!(
        fs::read_to_string(codex_home.join("config.toml")).unwrap(),
        "model = \"default\"\n"
    );
}

#[test]
fn imports_codex_toml_gateway_config_as_profile_file() {
    let temp = test_temp_dir("import-codex-toml-config");
    let codex_home = temp.join("codex-home");
    let state_root = temp.join("openmux-state");
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
}

#[test]
fn remove_profile_deletes_codex_profile_file() {
    let temp = test_temp_dir("remove-profile");
    let codex_home = temp.join("codex-home");
    let state_root = temp.join("openmux-state");
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
}

#[test]
fn imports_openai_compatible_kv_without_storing_raw_api_key() {
    let temp = test_temp_dir("import-codex-kv-config");
    let codex_home = temp.join("codex-home");
    let state_root = temp.join("openmux-state");
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
}

#[test]
fn extracts_account_from_account_id_when_jwt_has_no_account_claims() {
    let auth = br#"{"tokens":{"account_id":"account-456"}}"#;
    let metadata = extract_codex_account_metadata(auth);
    assert_eq!(metadata.account_label.as_deref(), Some("account-456"));
    assert_eq!(metadata.plan_label, None);
}

#[test]
fn parses_codex_usage_auth_without_exposing_tokens() {
    let auth = br#"{"tokens":{"access_token":"access-secret","account_id":"account-456"}}"#;
    assert_eq!(
        parse_codex_usage_auth(auth),
        Some(CodexUsageAuth {
            access_token: "access-secret".to_string(),
            account_id: "account-456".to_string(),
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
fn list_accounts_keeps_last_usage_snapshot_when_refresh_fails() {
    let temp = test_temp_dir("usage-refresh-fallback");
    let codex_home = temp.join("codex-home");
    let state_root = temp.join("openmux-state");
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
                diagnostics: Vec::new(),
            },
        )
        .unwrap();

    let status = plugin.list_accounts().unwrap().remove(0);
    let usage = status.usage.unwrap();

    assert_eq!(usage.source, UsageSource::StoredSnapshot);
    assert_eq!(usage.refreshed_at_unix, Some(1_785_000_000));
    assert_eq!(usage.limits[0].remaining_percent_x100, Some(7_200));
    assert_eq!(usage.diagnostics[0].code, "auth");
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
    let path = env::temp_dir().join(format!("openmux-test-{name}-{}", unix_now_nanos()));
    fs::create_dir_all(&path).unwrap();
    path
}
