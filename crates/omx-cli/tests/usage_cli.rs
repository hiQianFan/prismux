use serde_json::Value;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

static TEMP_ROOT_COUNTER: AtomicU64 = AtomicU64::new(0);

#[test]
fn usage_no_scan_json_returns_empty_versioned_payload() {
    let output = run_omx_usage(&["usage", "--no-scan", "--json"]);
    let payload = parse_json_stdout(output);

    assert_eq!(payload["schema_version"], 1);
    assert_eq!(
        payload["notes"]["usage"],
        "parsed local usage; not provider billing or exact quota accounting"
    );
    assert_eq!(
        payload["notes"]["cost"],
        "cost is optional and may be missing or estimated unless reported by the source"
    );
    assert_eq!(payload["scan"]["enabled"], false);
    assert_eq!(payload["scan"]["scanned_events"], 0);
    assert_eq!(payload["scan"]["inserted_events"], 0);
    assert_eq!(payload["quality"], "parsed");
    assert_eq!(payload["clients"].as_array().unwrap().len(), 0);
    assert_eq!(payload["diagnostics"].as_array().unwrap().len(), 0);
    assert!(
        payload["timezone"]
            .as_str()
            .is_some_and(|value| !value.is_empty())
    );
}

#[test]
fn usage_no_scan_json_accepts_client_and_local_date_range() {
    let output = run_omx_usage(&[
        "usage",
        "codex",
        "--since",
        "2026-06-23",
        "--until",
        "2026-06-23",
        "--no-scan",
        "--json",
    ]);
    let payload = parse_json_stdout(output);

    assert_eq!(payload["filter"]["client"], "codex");
    assert_eq!(
        payload["window"]["until_unix"].as_i64().unwrap()
            - payload["window"]["since_unix"].as_i64().unwrap(),
        86_400
    );
    assert_eq!(payload["clients"].as_array().unwrap().len(), 0);
}

#[test]
fn usage_json_reports_unsupported_client_as_safe_diagnostic() {
    let output = run_omx_usage(&["usage", "cursor", "--json"]);
    let payload = parse_json_stdout(output);
    let diagnostics = payload["diagnostics"].as_array().unwrap();

    assert_eq!(payload["filter"]["client"], "cursor");
    assert_eq!(payload["scan"]["enabled"], true);
    assert_eq!(payload["clients"].as_array().unwrap().len(), 0);
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0]["client"], "cursor");
    assert_eq!(diagnostics[0]["code"], "unsupported_client");
    assert_eq!(
        diagnostics[0]["message"],
        "client is not enabled for OpenMux usage scanning"
    );
}

#[test]
fn usage_json_missing_local_source_returns_empty_summary_without_fake_usage() {
    let home = unique_temp_root("openmux-usage-cli-empty-home");
    fs::create_dir_all(&home).unwrap();

    let output = run_omx_usage_with_home(
        &[
            "usage",
            "codex",
            "--since",
            "2026-04-30",
            "--until",
            "2026-04-30",
            "--json",
        ],
        Some(&home),
    );
    let payload = parse_json_stdout(output);

    assert_eq!(payload["scan"]["enabled"], true);
    assert_eq!(payload["scan"]["scanned_events"], 0);
    assert_eq!(payload["scan"]["inserted_events"], 0);
    assert_eq!(payload["clients"].as_array().unwrap().len(), 0);
    assert_eq!(payload["diagnostics"].as_array().unwrap().len(), 0);

    fs::remove_dir_all(&home).unwrap();
}

#[test]
fn usage_json_scans_codex_fixture_ingests_and_summarizes() {
    let home = unique_temp_root("openmux-usage-cli-home");
    write_codex_session_fixture(&home);

    let output = run_omx_usage_with_home(
        &[
            "usage",
            "codex",
            "--since",
            "2026-04-30",
            "--until",
            "2026-04-30",
            "--json",
        ],
        Some(&home),
    );
    let payload = parse_json_stdout(output);
    let clients = payload["clients"].as_array().unwrap();

    assert_eq!(payload["scan"]["enabled"], true, "{payload}");
    assert_eq!(payload["scan"]["scanned_events"], 1, "{payload}");
    assert_eq!(payload["scan"]["inserted_events"], 1);
    assert_eq!(clients.len(), 1);
    assert_eq!(clients[0]["client"], "codex");
    assert_eq!(clients[0]["model_provider"], "openai");
    assert_eq!(clients[0]["model"], "gpt-5");
    assert_eq!(clients[0]["tokens"]["input"], 8);
    assert_eq!(clients[0]["tokens"]["cache_read"], 2);
    assert_eq!(clients[0]["tokens"]["output"], 3);
    assert_eq!(clients[0]["tokens"]["reasoning"], 4);
    assert_eq!(clients[0]["tokens"]["normalized_total"], 17);
    assert_eq!(clients[0]["cost"]["status"], "missing");
    assert_eq!(clients[0]["quality"], "parsed");
    assert_eq!(clients[0]["event_count"], 1);
    assert_eq!(payload["diagnostics"].as_array().unwrap().len(), 0);

    fs::remove_dir_all(&home).unwrap();
}

#[test]
fn usage_json_scans_claude_fixture_ingests_and_summarizes() {
    let home = unique_temp_root("openmux-usage-cli-home");
    write_claude_session_fixture(&home);

    let output = run_omx_usage_with_home(
        &[
            "usage",
            "claude",
            "--since",
            "2026-04-01",
            "--until",
            "2026-04-01",
            "--json",
        ],
        Some(&home),
    );
    let payload = parse_json_stdout(output);
    let clients = payload["clients"].as_array().unwrap();

    assert_eq!(payload["scan"]["enabled"], true, "{payload}");
    assert_eq!(payload["scan"]["scanned_events"], 1, "{payload}");
    assert_eq!(payload["scan"]["inserted_events"], 1);
    assert_eq!(clients.len(), 1);
    assert_eq!(clients[0]["client"], "claude");
    assert_eq!(clients[0]["model"], "claude-sonnet-4");
    assert_eq!(clients[0]["tokens"]["input"], 123);
    assert_eq!(clients[0]["tokens"]["cache_read"], 67);
    assert_eq!(clients[0]["tokens"]["cache_write"], 8);
    assert_eq!(clients[0]["tokens"]["output"], 45);
    assert_eq!(clients[0]["tokens"]["normalized_total"], 243);
    assert_eq!(clients[0]["cost"]["status"], "missing");
    assert_eq!(clients[0]["quality"], "parsed");
    assert_eq!(clients[0]["event_count"], 1);
    assert_eq!(payload["diagnostics"].as_array().unwrap().len(), 0);

    fs::remove_dir_all(&home).unwrap();
}

#[test]
fn usage_json_scans_gemini_fixture_ingests_and_summarizes() {
    let home = unique_temp_root("openmux-usage-cli-home");
    write_gemini_session_fixture(&home);

    let output = run_omx_usage_with_home(
        &[
            "usage",
            "gemini",
            "--since",
            "2026-04-01",
            "--until",
            "2026-04-01",
            "--json",
        ],
        Some(&home),
    );
    let payload = parse_json_stdout(output);
    let clients = payload["clients"].as_array().unwrap();

    assert_eq!(payload["scan"]["enabled"], true, "{payload}");
    assert_eq!(payload["scan"]["scanned_events"], 1, "{payload}");
    assert_eq!(payload["scan"]["inserted_events"], 1);
    assert_eq!(clients.len(), 1);
    assert_eq!(clients[0]["client"], "gemini");
    assert_eq!(clients[0]["model"], "gemini-2.5-pro");
    assert_eq!(clients[0]["tokens"]["input"], 100);
    assert_eq!(clients[0]["tokens"]["cache_read"], 10);
    assert_eq!(clients[0]["tokens"]["output"], 25);
    assert_eq!(clients[0]["tokens"]["reasoning"], 5);
    assert_eq!(clients[0]["tokens"]["normalized_total"], 140);
    assert_eq!(clients[0]["cost"]["status"], "missing");
    assert_eq!(clients[0]["quality"], "parsed");
    assert_eq!(clients[0]["event_count"], 1);
    assert_eq!(payload["diagnostics"].as_array().unwrap().len(), 0);

    fs::remove_dir_all(&home).unwrap();
}

#[test]
fn usage_json_scan_does_not_leak_raw_prompt_response_or_api_key() {
    let home = unique_temp_root("openmux-usage-cli-home");
    write_sensitive_claude_session_fixture(&home);
    let state_root = unique_temp_state_root();

    let output = run_omx_usage_with_home_and_state(
        &[
            "usage",
            "claude",
            "--since",
            "2026-04-01",
            "--until",
            "2026-04-01",
            "--json",
        ],
        Some(&home),
        &state_root,
    );
    assert!(
        output.status.success(),
        "omx failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(!stdout.contains("raw prompt secret"));
    assert!(!stdout.contains("raw response secret"));
    assert!(!stdout.contains("sk-openmux-secret"));
    assert!(!stderr.contains("raw prompt secret"));
    assert!(!stderr.contains("raw response secret"));
    assert!(!stderr.contains("sk-openmux-secret"));

    let payload: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(payload["scan"]["scanned_events"], 1);
    assert_eq!(payload["diagnostics"].as_array().unwrap().len(), 0);

    let sqlite_path = state_root.join("omx-state.sqlite");
    let sqlite_bytes = fs::read(&sqlite_path).unwrap();
    assert!(!contains_bytes(&sqlite_bytes, b"raw prompt secret"));
    assert!(!contains_bytes(&sqlite_bytes, b"raw response secret"));
    assert!(!contains_bytes(&sqlite_bytes, b"sk-openmux-secret"));

    fs::remove_dir_all(&home).unwrap();
    fs::remove_dir_all(&state_root).unwrap();
}

#[test]
fn usage_json_repeated_scan_is_idempotent_and_keeps_summary() {
    let home = unique_temp_root("openmux-usage-cli-home");
    let state_root = unique_temp_state_root();
    write_codex_session_fixture(&home);
    let args = [
        "usage",
        "codex",
        "--since",
        "2026-04-30",
        "--until",
        "2026-04-30",
        "--json",
    ];

    let first = parse_json_stdout(run_omx_usage_with_home_and_state(
        &args,
        Some(&home),
        &state_root,
    ));
    let second = parse_json_stdout(run_omx_usage_with_home_and_state(
        &args,
        Some(&home),
        &state_root,
    ));

    assert_eq!(first["scan"]["scanned_events"], 1);
    assert_eq!(first["scan"]["inserted_events"], 1);
    assert_eq!(first["clients"][0]["event_count"], 1);
    assert_eq!(first["clients"][0]["tokens"]["normalized_total"], 17);

    assert_eq!(second["scan"]["scanned_events"], 1);
    assert_eq!(second["scan"]["inserted_events"], 0);
    assert_eq!(second["clients"][0]["event_count"], 1);
    assert_eq!(second["clients"][0]["tokens"]["normalized_total"], 17);

    fs::remove_dir_all(&home).unwrap();
    fs::remove_dir_all(&state_root).unwrap();
}

#[test]
fn usage_scan_diagnostic_preserves_existing_history() {
    let home = unique_temp_root("openmux-usage-cli-home");
    let state_root = unique_temp_state_root();
    write_codex_session_fixture(&home);

    let first = parse_json_stdout(run_omx_usage_with_home_and_state(
        &[
            "usage",
            "codex",
            "--since",
            "2026-04-30",
            "--until",
            "2026-04-30",
            "--json",
        ],
        Some(&home),
        &state_root,
    ));
    assert_eq!(first["scan"]["inserted_events"], 1);
    assert_eq!(first["clients"][0]["tokens"]["normalized_total"], 17);

    let unsupported = parse_json_stdout(run_omx_usage_with_home_and_state(
        &["usage", "cursor", "--json"],
        Some(&home),
        &state_root,
    ));
    assert_eq!(unsupported["diagnostics"][0]["code"], "unsupported_client");

    let cached = parse_json_stdout(run_omx_usage_with_home_and_state(
        &[
            "usage",
            "codex",
            "--since",
            "2026-04-30",
            "--until",
            "2026-04-30",
            "--no-scan",
            "--json",
        ],
        Some(&home),
        &state_root,
    ));
    assert_eq!(cached["scan"]["enabled"], false);
    assert_eq!(cached["clients"][0]["event_count"], 1);
    assert_eq!(cached["clients"][0]["tokens"]["normalized_total"], 17);

    fs::remove_dir_all(&home).unwrap();
    fs::remove_dir_all(&state_root).unwrap();
}

#[test]
fn usage_scan_diagnostic_does_not_break_account_commands() {
    let home = unique_temp_root("openmux-usage-cli-home");
    let state_root = unique_temp_state_root();
    write_codex_auth(&home, r#"{"account":"work"}"#);

    let save_work = run_omx_usage_with_home_and_state(
        &["save", "codex", "--alias", "work"],
        Some(&home),
        &state_root,
    );
    assert_command_success(&save_work);

    write_codex_auth(&home, r#"{"account":"personal"}"#);
    let save_personal =
        run_omx_usage_with_home_and_state(&["save", "codex"], Some(&home), &state_root);
    assert_command_success(&save_personal);

    let usage = parse_json_stdout(run_omx_usage_with_home_and_state(
        &["usage", "cursor", "--json"],
        Some(&home),
        &state_root,
    ));
    assert_eq!(usage["diagnostics"][0]["code"], "unsupported_client");

    let use_work =
        run_omx_usage_with_home_and_state(&["use", "codex", "work"], Some(&home), &state_root);
    assert_command_success(&use_work);
    let use_stdout = String::from_utf8_lossy(&use_work.stdout);
    assert!(use_stdout.contains("Using Codex account #1 work"));

    let current =
        run_omx_usage_with_home_and_state(&["current", "codex"], Some(&home), &state_root);
    assert_command_success(&current);
    let current_stdout = String::from_utf8_lossy(&current.stdout);
    assert!(current_stdout.contains("#1 work"));

    let list = run_omx_usage_with_home_and_state(&["list", "codex"], Some(&home), &state_root);
    assert_command_success(&list);
    let list_stdout = String::from_utf8_lossy(&list.stdout);
    assert!(list_stdout.contains("work"));
    assert!(list_stdout.contains("Codex"));

    fs::remove_dir_all(&home).unwrap();
    fs::remove_dir_all(&state_root).unwrap();
}

#[test]
fn codex_account_and_provider_remain_active_through_same_use_command() {
    let home = unique_temp_root("openmux-codex-account-provider-home");
    let state_root = unique_temp_state_root();
    write_codex_auth(&home, r#"{"account":"work"}"#);
    fs::write(
        home.join(".codex/config.toml"),
        "# user preference\n[plugins.\"ponytail@ponytail\"]\nenabled = true\n",
    )
    .unwrap();

    assert_command_success(&run_omx_usage_with_home_and_state(
        &["save", "codex", "--alias", "work"],
        Some(&home),
        &state_root,
    ));
    assert_command_success(&run_omx_usage_with_home_and_state(
        &[
            "import",
            "codex",
            "--name",
            "gateway",
            "OPENAI_BASE_URL=https://gateway.example/v1",
        ],
        Some(&home),
        &state_root,
    ));
    assert_command_success(&run_omx_usage_with_home_and_state(
        &["use", "codex", "2"],
        Some(&home),
        &state_root,
    ));
    let provider_config = fs::read(home.join(".codex/config.toml")).unwrap();

    assert_command_success(&run_omx_usage_with_home_and_state(
        &["use", "codex", "1"],
        Some(&home),
        &state_root,
    ));

    assert_eq!(
        fs::read(home.join(".codex/config.toml")).unwrap(),
        provider_config
    );
    let current =
        run_omx_usage_with_home_and_state(&["current", "codex"], Some(&home), &state_root);
    assert_command_success(&current);
    let stdout = String::from_utf8_lossy(&current.stdout);
    assert!(stdout.contains("#1 work"));
    assert!(stdout.contains("gateway"));

    fs::remove_dir_all(&home).unwrap();
    fs::remove_dir_all(&state_root).unwrap();
}

fn run_omx_usage(args: &[&str]) -> std::process::Output {
    run_omx_usage_with_home(args, None)
}

fn run_omx_usage_with_home(args: &[&str], home: Option<&Path>) -> std::process::Output {
    let state_root = unique_temp_state_root();
    let output = run_omx_usage_with_home_and_state(args, home, &state_root);
    fs::remove_dir_all(&state_root).unwrap();
    output
}

fn run_omx_usage_with_home_and_state(
    args: &[&str],
    home: Option<&Path>,
    state_root: &Path,
) -> std::process::Output {
    fs::create_dir_all(state_root).unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_omx"));
    command.args(args).env("OMUX_STATE_ROOT", state_root);
    if let Some(home) = home {
        command
            .env("HOME", home)
            .env("CODEX_HOME", home.join(".codex"))
            .env("GEMINI_CLI_HOME", home.join(".gemini"))
            .env_remove("XDG_DATA_HOME");
    }
    command.output().unwrap()
}

fn parse_json_stdout(output: std::process::Output) -> Value {
    assert_command_success(&output);
    serde_json::from_slice(&output.stdout).unwrap()
}

fn assert_command_success(output: &std::process::Output) {
    assert!(
        output.status.success(),
        "omx failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn unique_temp_state_root() -> PathBuf {
    unique_temp_root("openmux-usage-cli-test")
}

fn unique_temp_root(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let counter = TEMP_ROOT_COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!("{prefix}-{}-{nanos}-{counter}", std::process::id()))
}

fn write_codex_session_fixture(home: &Path) {
    let codex_dir = home.join(".codex/sessions");
    fs::create_dir_all(&codex_dir).unwrap();
    fs::write(
        codex_dir.join("session-1.jsonl"),
        concat!(
            r#"{"timestamp":"2026-04-30T11:00:00Z","type":"session_meta","payload":{"id":"session-1","source":"interactive","model_provider":"openai","cwd":"/tmp/openmux-project"}}"#,
            "\n",
            r#"{"timestamp":"2026-04-30T11:00:01Z","type":"turn_context","payload":{"model":"gpt-5"}}"#,
            "\n",
            r#"{"timestamp":"2026-04-30T11:00:02Z","type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":10,"cached_input_tokens":2,"output_tokens":3,"reasoning_output_tokens":4},"last_token_usage":{"input_tokens":10,"cached_input_tokens":2,"output_tokens":3,"reasoning_output_tokens":4}}}}"#,
            "\n",
        ),
    )
    .unwrap();
}

fn write_codex_auth(home: &Path, auth: &str) {
    let codex_home = home.join(".codex");
    fs::create_dir_all(&codex_home).unwrap();
    fs::write(codex_home.join("auth.json"), auth).unwrap();
}

fn write_claude_session_fixture(home: &Path) {
    let transcripts_dir = home.join(".claude/transcripts");
    fs::create_dir_all(&transcripts_dir).unwrap();
    fs::write(
        transcripts_dir.join("ses_123456789012345678901234567.jsonl"),
        concat!(
            r#"{"type":"user","timestamp":"2026-04-01T10:00:00.000Z","message":{"content":"Wrapped prompt"}}"#,
            "\n",
            r#"{"type":"assistant","timestamp":"2026-04-01T10:00:01.000Z","requestId":"req_wrapper","message":{"id":"msg_wrapper","model":"claude-sonnet-4","usage":{"input_tokens":123,"output_tokens":45,"cache_read_input_tokens":67,"cache_creation_input_tokens":8}}}"#,
            "\n",
        ),
    )
    .unwrap();
}

fn write_sensitive_claude_session_fixture(home: &Path) {
    let transcripts_dir = home.join(".claude/transcripts");
    fs::create_dir_all(&transcripts_dir).unwrap();
    fs::write(
        transcripts_dir.join("ses_223456789012345678901234567.jsonl"),
        concat!(
            r#"{"type":"user","timestamp":"2026-04-01T10:00:00.000Z","message":{"content":"raw prompt secret sk-openmux-secret"}}"#,
            "\n",
            r#"{"type":"assistant","timestamp":"2026-04-01T10:00:01.000Z","requestId":"req_sensitive","message":{"id":"msg_sensitive","model":"claude-sonnet-4","content":[{"type":"text","text":"raw response secret"}],"usage":{"input_tokens":11,"output_tokens":7,"cache_read_input_tokens":0,"cache_creation_input_tokens":0}}}"#,
            "\n",
        ),
    )
    .unwrap();
}

fn write_gemini_session_fixture(home: &Path) {
    let chats_dir = home.join(".gemini/tmp/project-1/chats");
    fs::create_dir_all(&chats_dir).unwrap();
    fs::write(
        chats_dir.join("chat-1.json"),
        r#"{
  "sessionId": "gemini-session-1",
  "projectHash": "project-1",
  "startTime": "2026-04-01T10:00:00.000Z",
  "lastUpdated": "2026-04-01T10:00:01.000Z",
  "messages": [
    {
      "id": "msg-1",
      "timestamp": "2026-04-01T10:00:01.000Z",
      "type": "assistant",
      "model": "gemini-2.5-pro",
      "tokens": {
        "input": 100,
        "output": 25,
        "cached": 10,
        "thoughts": 5
      }
    }
  ]
}"#,
    )
    .unwrap();
}

fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}
