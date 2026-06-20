use omx_core::{
    CostStatus, UsageBackend, UsageDataQuality, UsageEvent, UsageEventSource, UsageScanDiagnostic,
    UsageScanOptions, UsageScanReport, UsageTokenBreakdown,
};
use std::path::PathBuf;

pub const TOKSCALE_BACKEND: &str = "tokscale-core";
pub const TOKSCALE_BACKEND_VERSION: &str = "3.1.3+cbbd0dff";
pub const TOKSCALE_PARSER_SCHEMA_VERSION: u32 = 1;

const DEFAULT_CLIENTS: &[&str] = &["codex", "claude", "gemini"];

#[derive(Debug, Clone, Default)]
pub struct TokscaleUsageBackend {
    home_dir: Option<PathBuf>,
}

impl TokscaleUsageBackend {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_home_dir(home_dir: impl Into<PathBuf>) -> Self {
        Self {
            home_dir: Some(home_dir.into()),
        }
    }
}

impl UsageBackend for TokscaleUsageBackend {
    fn scan(&self, options: UsageScanOptions) -> omx_core::Result<UsageScanReport> {
        let (clients, diagnostics) = requested_clients(options.clients);
        let home_dir = self
            .home_dir
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned());
        let parsed = tokscale_core::parse_local_clients(tokscale_core::LocalParseOptions {
            home_dir,
            use_env_roots: true,
            clients: Some(clients.clone()),
            since: None,
            until: None,
            year: None,
            scanner_settings: tokscale_core::scanner::ScannerSettings::default(),
        })
        .map_err(|err| omx_core::OpenMuxError::Message(format!("tokscale scan: {err}")))?;

        let events = parsed
            .messages
            .into_iter()
            .filter(|message| {
                within_window(message.timestamp, options.since_unix, options.until_unix)
            })
            .map(usage_event_from_tokscale_message)
            .collect();

        Ok(UsageScanReport {
            backend: TOKSCALE_BACKEND.to_string(),
            backend_version: TOKSCALE_BACKEND_VERSION.to_string(),
            parser_schema_version: TOKSCALE_PARSER_SCHEMA_VERSION,
            events,
            diagnostics,
        })
    }
}

fn requested_clients(clients: Vec<String>) -> (Vec<String>, Vec<UsageScanDiagnostic>) {
    if clients.is_empty() {
        return (
            DEFAULT_CLIENTS
                .iter()
                .map(|client| (*client).to_string())
                .collect(),
            Vec::new(),
        );
    }

    let mut accepted = Vec::new();
    let mut diagnostics = Vec::new();
    for client in clients {
        if DEFAULT_CLIENTS.contains(&client.as_str()) {
            accepted.push(client);
        } else {
            diagnostics.push(UsageScanDiagnostic {
                client: Some(client),
                source_kind: None,
                code: "unsupported_client".to_string(),
                message: "client is not enabled for OpenMux usage scanning".to_string(),
            });
        }
    }
    accepted.sort();
    accepted.dedup();
    (accepted, diagnostics)
}

fn within_window(timestamp: i64, since_unix: Option<i64>, until_unix: Option<i64>) -> bool {
    if let Some(since_unix) = since_unix
        && timestamp < since_unix
    {
        return false;
    }
    if let Some(until_unix) = until_unix
        && timestamp >= until_unix
    {
        return false;
    }
    true
}

fn usage_event_from_tokscale_message(message: tokscale_core::ParsedMessage) -> UsageEvent {
    let mut event = UsageEvent {
        client: message.client,
        model_provider: non_empty(message.provider_id),
        model: non_empty(message.model_id),
        session_id: non_empty(message.session_id),
        request_id: None,
        project_path: message.workspace_label.map(PathBuf::from),
        occurred_at_unix: message.timestamp,
        tokens: UsageTokenBreakdown {
            input: non_negative(message.input),
            output: non_negative(message.output),
            cache_read: non_negative(message.cache_read),
            cache_write: non_negative(message.cache_write),
            cache_write_5m: None,
            cache_write_1h: None,
            reasoning: non_negative(message.reasoning),
            extra: 0,
        },
        provider_total_tokens: None,
        estimated_cost_usd: None,
        cost_status: CostStatus::Missing,
        source: UsageEventSource {
            kind: "tokscale-local".to_string(),
            path: None,
            fingerprint_json: None,
            offset: None,
            record_id: None,
            record_hash: message.agent,
            backend: TOKSCALE_BACKEND.to_string(),
            backend_version: TOKSCALE_BACKEND_VERSION.to_string(),
            parser_schema_version: TOKSCALE_PARSER_SCHEMA_VERSION,
        },
        quality: UsageDataQuality::Parsed,
        event_hash: String::new(),
    };
    event.set_generated_event_hash();
    event
}

fn non_negative(value: i64) -> u64 {
    value.max(0) as u64
}

fn non_empty(value: String) -> Option<String> {
    (!value.trim().is_empty()).then_some(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use omx_core::UsageScanBudget;
    use std::fs;

    #[test]
    fn scan_defaults_to_codex_claude_and_gemini_only() {
        let (clients, diagnostics) = requested_clients(Vec::new());

        assert_eq!(clients, vec!["codex", "claude", "gemini"]);
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn scan_rejects_unsupported_clients_before_calling_tokscale() {
        let (clients, diagnostics) =
            requested_clients(vec!["codex".to_string(), "cursor".to_string()]);

        assert_eq!(clients, vec!["codex"]);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].client.as_deref(), Some("cursor"));
        assert_eq!(diagnostics[0].code, "unsupported_client");
    }

    #[test]
    fn scan_maps_codex_tokens_into_openmux_usage_events() {
        let home = tempfile::tempdir().unwrap();
        let codex_dir = home.path().join(".codex/sessions");
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
        let backend = TokscaleUsageBackend::with_home_dir(home.path());

        let report = backend
            .scan(UsageScanOptions {
                clients: vec!["codex".to_string()],
                since_unix: None,
                until_unix: None,
                budget: UsageScanBudget::default(),
            })
            .unwrap();

        assert_eq!(report.backend, TOKSCALE_BACKEND);
        assert_eq!(report.events.len(), 1);
        let event = &report.events[0];
        assert_eq!(event.client, "codex");
        assert_eq!(event.model_provider.as_deref(), Some("openai"));
        assert_eq!(event.model.as_deref(), Some("gpt-5"));
        assert_eq!(event.tokens.input, 8);
        assert_eq!(event.tokens.cache_read, 2);
        assert_eq!(event.tokens.output, 3);
        assert_eq!(event.tokens.reasoning, 4);
        assert_eq!(event.cost_status, CostStatus::Missing);
        assert!(!event.event_hash.is_empty());
    }

    #[test]
    fn scan_maps_claude_tokens_into_openmux_usage_events() {
        let home = tempfile::tempdir().unwrap();
        let transcripts_dir = home.path().join(".claude/transcripts");
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
        let backend = TokscaleUsageBackend::with_home_dir(home.path());

        let report = backend
            .scan(UsageScanOptions {
                clients: vec!["claude".to_string()],
                since_unix: None,
                until_unix: None,
                budget: UsageScanBudget::default(),
            })
            .unwrap();

        assert_eq!(report.events.len(), 1);
        let event = &report.events[0];
        assert_eq!(event.client, "claude");
        assert_eq!(event.model.as_deref(), Some("claude-sonnet-4"));
        assert_eq!(event.tokens.input, 123);
        assert_eq!(event.tokens.output, 45);
        assert_eq!(event.tokens.cache_read, 67);
        assert_eq!(event.tokens.cache_write, 8);
    }

    #[test]
    fn scan_maps_gemini_tokens_into_openmux_usage_events() {
        let home = tempfile::tempdir().unwrap();
        let chats_dir = home.path().join(".gemini/tmp/project-1/chats");
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
        let backend = TokscaleUsageBackend::with_home_dir(home.path());

        let report = backend
            .scan(UsageScanOptions {
                clients: vec!["gemini".to_string()],
                since_unix: None,
                until_unix: None,
                budget: UsageScanBudget::default(),
            })
            .unwrap();

        assert_eq!(report.events.len(), 1);
        let event = &report.events[0];
        assert_eq!(event.client, "gemini");
        assert_eq!(event.model.as_deref(), Some("gemini-2.5-pro"));
        assert_eq!(event.tokens.input, 100);
        assert_eq!(event.tokens.cache_read, 10);
        assert_eq!(event.tokens.output, 25);
        assert_eq!(event.tokens.reasoning, 5);
    }

    #[test]
    fn scan_applies_openmux_time_window_after_tokscale_parse() {
        let home = tempfile::tempdir().unwrap();
        let codex_dir = home.path().join(".codex/sessions");
        fs::create_dir_all(&codex_dir).unwrap();
        fs::write(
            codex_dir.join("session-1.jsonl"),
            concat!(
                r#"{"timestamp":"2026-04-30T11:00:00Z","type":"session_meta","payload":{"id":"session-1","source":"interactive","model_provider":"openai"}}"#,
                "\n",
                r#"{"timestamp":"2026-04-30T11:00:01Z","type":"turn_context","payload":{"model":"gpt-5"}}"#,
                "\n",
                r#"{"timestamp":"2026-04-30T11:00:02Z","type":"event_msg","payload":{"type":"token_count","info":{"last_token_usage":{"input_tokens":10,"output_tokens":3}}}}"#,
                "\n",
            ),
        )
        .unwrap();
        let backend = TokscaleUsageBackend::with_home_dir(home.path());

        let report = backend
            .scan(UsageScanOptions {
                clients: vec!["codex".to_string()],
                since_unix: Some(1_776_000_000),
                until_unix: Some(1_776_000_100),
                budget: UsageScanBudget::default(),
            })
            .unwrap();

        assert!(report.events.is_empty());
    }
}
