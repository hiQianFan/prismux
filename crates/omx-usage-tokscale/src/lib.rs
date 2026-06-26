use omx_core::{
    CostStatus, UsageDataQuality, UsageEvent, UsageEventSource, UsageScanBudget,
    UsageScanDiagnostic, UsageScanOptions, UsageScanReport, UsageTokenBreakdown,
};
use std::{
    fs,
    path::PathBuf,
    sync::Arc,
    sync::mpsc::{self, Receiver, RecvTimeoutError},
    thread,
    time::Duration,
};

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

impl TokscaleUsageBackend {
    pub fn scan(&self, options: UsageScanOptions) -> omx_core::Result<UsageScanReport> {
        let (clients, diagnostics) = requested_clients(options.clients);
        if clients.is_empty() {
            return Ok(usage_scan_report(Vec::new(), diagnostics));
        }
        if options.budget.timeout_ms == 0 {
            return Ok(usage_scan_report(
                Vec::new(),
                diagnostics_with_budget_exceeded(diagnostics, &clients),
            ));
        }

        let home_dir = self
            .home_dir
            .as_ref()
            .map(|path| path.to_string_lossy().into_owned());
        if let Some(diagnostic) =
            source_budget_diagnostic(home_dir.as_deref(), &clients, &options.budget)?
        {
            let mut diagnostics = diagnostics;
            diagnostics.push(diagnostic);
            return Ok(usage_scan_report(Vec::new(), diagnostics));
        }
        let Some(parsed) =
            parse_local_clients_with_timeout(home_dir, clients.clone(), options.budget.timeout_ms)?
        else {
            return Ok(usage_scan_report(
                Vec::new(),
                diagnostics_with_budget_exceeded(diagnostics, &clients),
            ));
        };

        let pricing = load_pricing_with_timeout(options.budget.timeout_ms);
        let events = parsed
            .messages
            .into_iter()
            .filter(|message| {
                within_window(
                    tokscale_timestamp_to_unix_seconds(message.timestamp),
                    options.since_unix,
                    options.until_unix,
                )
            })
            .map(|message| usage_event_from_tokscale_message(message, pricing.as_deref()))
            .collect();

        Ok(usage_scan_report(events, diagnostics))
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct SourceInventory {
    file_count: usize,
    total_bytes: u64,
}

fn source_budget_diagnostic(
    home_dir: Option<&str>,
    clients: &[String],
    budget: &UsageScanBudget,
) -> omx_core::Result<Option<UsageScanDiagnostic>> {
    let home_dir = resolved_home_dir(home_dir)?;
    let inventory = source_inventory(&home_dir, clients);
    if inventory.file_count > budget.max_source_files {
        return Ok(Some(UsageScanDiagnostic {
            client: usage_diagnostic_client(clients),
            source_kind: Some("tokscale-local".to_string()),
            code: "budget_exceeded".to_string(),
            message: format!(
                "usage scan source file count exceeds configured budget: {} > {}",
                inventory.file_count, budget.max_source_files
            ),
        }));
    }
    if inventory.total_bytes > budget.max_source_bytes {
        return Ok(Some(UsageScanDiagnostic {
            client: usage_diagnostic_client(clients),
            source_kind: Some("tokscale-local".to_string()),
            code: "budget_exceeded".to_string(),
            message: format!(
                "usage scan source bytes exceed configured budget: {} > {}",
                inventory.total_bytes, budget.max_source_bytes
            ),
        }));
    }
    Ok(None)
}

fn resolved_home_dir(home_dir: Option<&str>) -> omx_core::Result<String> {
    if let Some(home_dir) = home_dir {
        return Ok(home_dir.to_string());
    }
    std::env::var("HOME").map_err(|err| {
        omx_core::OpenMuxError::Message(format!("resolve home directory for usage scan: {err}"))
    })
}

fn source_inventory(home_dir: &str, clients: &[String]) -> SourceInventory {
    let scan_result = tokscale_core::scanner::scan_all_clients_with_scanner_settings(
        home_dir,
        clients,
        true,
        &tokscale_core::scanner::ScannerSettings::default(),
    );
    scan_result.all_files().into_iter().fold(
        SourceInventory::default(),
        |mut inventory, (_, path)| {
            inventory.file_count += 1;
            inventory.total_bytes = inventory
                .total_bytes
                .saturating_add(file_size(&path).unwrap_or(0));
            inventory
        },
    )
}

fn file_size(path: &PathBuf) -> Option<u64> {
    fs::metadata(path).ok().map(|metadata| metadata.len())
}

fn parse_local_clients_with_timeout(
    home_dir: Option<String>,
    clients: Vec<String>,
    timeout_ms: u64,
) -> omx_core::Result<Option<tokscale_core::ParsedMessages>> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let result = tokscale_core::parse_local_clients(tokscale_core::LocalParseOptions {
            home_dir,
            use_env_roots: true,
            clients: Some(clients),
            since: None,
            until: None,
            year: None,
            scanner_settings: tokscale_core::scanner::ScannerSettings::default(),
        });
        let _ = sender.send(result);
    });
    wait_for_parse_result(receiver, timeout_ms)
}

fn wait_for_parse_result(
    receiver: Receiver<Result<tokscale_core::ParsedMessages, String>>,
    timeout_ms: u64,
) -> omx_core::Result<Option<tokscale_core::ParsedMessages>> {
    match receiver.recv_timeout(Duration::from_millis(timeout_ms)) {
        Ok(Ok(parsed)) => Ok(Some(parsed)),
        Ok(Err(err)) => Err(omx_core::OpenMuxError::Message(format!(
            "tokscale scan: {err}"
        ))),
        Err(RecvTimeoutError::Timeout) => Ok(None),
        Err(RecvTimeoutError::Disconnected) => Err(omx_core::OpenMuxError::Message(
            "tokscale scan worker exited before returning a result".to_string(),
        )),
    }
}

fn load_pricing_with_timeout(
    timeout_ms: u64,
) -> Option<Arc<tokscale_core::pricing::PricingService>> {
    if let Some(pricing) = tokscale_core::pricing::PricingService::load_cached_any_age() {
        return Some(Arc::new(pricing));
    }
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_time()
        .enable_io()
        .build()
        .ok()?;
    runtime.block_on(async {
        tokio::time::timeout(
            Duration::from_millis(timeout_ms.min(5_000)),
            tokscale_core::pricing::PricingService::get_or_init(),
        )
        .await
        .ok()
        .and_then(Result::ok)
    })
}

fn usage_scan_report(
    events: Vec<UsageEvent>,
    diagnostics: Vec<UsageScanDiagnostic>,
) -> UsageScanReport {
    UsageScanReport {
        backend: TOKSCALE_BACKEND.to_string(),
        backend_version: TOKSCALE_BACKEND_VERSION.to_string(),
        parser_schema_version: TOKSCALE_PARSER_SCHEMA_VERSION,
        events,
        diagnostics,
    }
}

fn diagnostics_with_budget_exceeded(
    mut diagnostics: Vec<UsageScanDiagnostic>,
    clients: &[String],
) -> Vec<UsageScanDiagnostic> {
    diagnostics.push(UsageScanDiagnostic {
        client: usage_diagnostic_client(clients),
        source_kind: Some("tokscale-local".to_string()),
        code: "budget_exceeded".to_string(),
        message: "usage scan exceeded the configured time budget".to_string(),
    });
    diagnostics
}

fn usage_diagnostic_client(clients: &[String]) -> Option<String> {
    match clients {
        [client] => Some(client.clone()),
        _ => None,
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

fn tokscale_timestamp_to_unix_seconds(timestamp: i64) -> i64 {
    if timestamp.abs() >= 10_000_000_000 {
        timestamp / 1_000
    } else {
        timestamp
    }
}

fn usage_event_from_tokscale_message(
    message: tokscale_core::ParsedMessage,
    pricing: Option<&tokscale_core::pricing::PricingService>,
) -> UsageEvent {
    let occurred_at_unix = tokscale_timestamp_to_unix_seconds(message.timestamp);
    let mut event = UsageEvent {
        client: message.client,
        model_provider: non_empty(message.provider_id),
        model: non_empty(message.model_id),
        session_id: non_empty(message.session_id),
        request_id: None,
        project_path: message.workspace_label.map(PathBuf::from),
        occurred_at_unix,
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
    apply_estimated_cost(&mut event, pricing);
    event.set_generated_event_hash();
    event
}

fn apply_estimated_cost(
    event: &mut UsageEvent,
    pricing: Option<&tokscale_core::pricing::PricingService>,
) {
    let (Some(pricing), Some(model)) = (pricing, event.model.as_deref()) else {
        return;
    };
    let cost = pricing.calculate_cost_with_provider(
        model,
        event.model_provider.as_deref(),
        &tokscale_core::TokenBreakdown {
            input: event.tokens.input.min(i64::MAX as u64) as i64,
            output: event.tokens.output.min(i64::MAX as u64) as i64,
            cache_read: event.tokens.cache_read.min(i64::MAX as u64) as i64,
            cache_write: event.tokens.cache_write.min(i64::MAX as u64) as i64,
            reasoning: event.tokens.reasoning.min(i64::MAX as u64) as i64,
        },
    );
    if cost > 0.0 {
        event.estimated_cost_usd = Some(cost);
        event.cost_status = CostStatus::Estimated;
    }
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
    use std::collections::HashMap;
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
    fn scan_with_only_unsupported_clients_does_not_scan_everything() {
        let home = tempfile::tempdir().unwrap();
        write_codex_session_fixture(home.path());
        let backend = TokscaleUsageBackend::with_home_dir(home.path());

        let report = backend
            .scan(UsageScanOptions {
                clients: vec!["cursor".to_string()],
                since_unix: None,
                until_unix: None,
                budget: UsageScanBudget::default(),
            })
            .unwrap();

        assert!(report.events.is_empty());
        assert_eq!(report.diagnostics.len(), 1);
        assert_eq!(report.diagnostics[0].client.as_deref(), Some("cursor"));
        assert_eq!(report.diagnostics[0].code, "unsupported_client");
    }

    #[test]
    fn scan_zero_timeout_budget_returns_budget_diagnostic_without_events() {
        let home = tempfile::tempdir().unwrap();
        write_codex_session_fixture(home.path());
        let backend = TokscaleUsageBackend::with_home_dir(home.path());

        let report = backend
            .scan(UsageScanOptions {
                clients: vec!["codex".to_string()],
                since_unix: None,
                until_unix: None,
                budget: UsageScanBudget {
                    timeout_ms: 0,
                    ..UsageScanBudget::default()
                },
            })
            .unwrap();

        assert!(report.events.is_empty());
        assert_eq!(report.diagnostics.len(), 1);
        assert_eq!(report.diagnostics[0].client.as_deref(), Some("codex"));
        assert_eq!(report.diagnostics[0].code, "budget_exceeded");
        assert!(!report.diagnostics[0].message.contains("session-1"));
    }

    #[test]
    fn scan_source_file_budget_returns_budget_diagnostic_without_events() {
        let home = tempfile::tempdir().unwrap();
        write_codex_session_fixture(home.path());
        let backend = TokscaleUsageBackend::with_home_dir(home.path());

        let report = backend
            .scan(UsageScanOptions {
                clients: vec!["codex".to_string()],
                since_unix: None,
                until_unix: None,
                budget: UsageScanBudget {
                    max_source_files: 0,
                    ..UsageScanBudget::default()
                },
            })
            .unwrap();

        assert!(report.events.is_empty());
        assert_eq!(report.diagnostics.len(), 1);
        assert_eq!(report.diagnostics[0].client.as_deref(), Some("codex"));
        assert_eq!(report.diagnostics[0].code, "budget_exceeded");
        assert!(report.diagnostics[0].message.contains("source file count"));
        assert!(!report.diagnostics[0].message.contains("session-1"));
    }

    #[test]
    fn scan_source_bytes_budget_returns_budget_diagnostic_without_events() {
        let home = tempfile::tempdir().unwrap();
        write_codex_session_fixture(home.path());
        let backend = TokscaleUsageBackend::with_home_dir(home.path());

        let report = backend
            .scan(UsageScanOptions {
                clients: vec!["codex".to_string()],
                since_unix: None,
                until_unix: None,
                budget: UsageScanBudget {
                    max_source_bytes: 1,
                    ..UsageScanBudget::default()
                },
            })
            .unwrap();

        assert!(report.events.is_empty());
        assert_eq!(report.diagnostics.len(), 1);
        assert_eq!(report.diagnostics[0].client.as_deref(), Some("codex"));
        assert_eq!(report.diagnostics[0].code, "budget_exceeded");
        assert!(report.diagnostics[0].message.contains("source bytes"));
        assert!(!report.diagnostics[0].message.contains("session-1"));
    }

    #[test]
    fn wait_for_parse_result_returns_none_on_timeout() {
        let (_sender, receiver) = mpsc::channel();

        let result = wait_for_parse_result(receiver, 1).unwrap();

        assert!(result.is_none());
    }

    #[test]
    fn scan_maps_codex_tokens_into_openmux_usage_events() {
        let home = tempfile::tempdir().unwrap();
        write_codex_session_fixture(home.path());
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
        assert_eq!(event.occurred_at_unix, 1_777_546_802);
        assert_eq!(event.tokens.input, 8);
        assert_eq!(event.tokens.cache_read, 2);
        assert_eq!(event.tokens.output, 3);
        assert_eq!(event.tokens.reasoning, 4);
        assert!(!event.event_hash.is_empty());
    }

    #[test]
    fn usage_event_uses_cached_pricing_when_available() {
        let mut prices = HashMap::new();
        prices.insert(
            "fixture-model".to_string(),
            tokscale_core::pricing::ModelPricing {
                input_cost_per_token: Some(0.001),
                output_cost_per_token: Some(0.002),
                cache_read_input_token_cost: Some(0.0001),
                cache_creation_input_token_cost: Some(0.0005),
                ..Default::default()
            },
        );
        let pricing = tokscale_core::pricing::PricingService::new(prices, HashMap::new());

        let event = usage_event_from_tokscale_message(
            tokscale_core::ParsedMessage {
                client: "codex".to_string(),
                model_id: "fixture-model".to_string(),
                provider_id: "openai".to_string(),
                session_id: "session-1".to_string(),
                workspace_key: None,
                workspace_label: None,
                timestamp: 1_777_546_802,
                date: "2026-04-29".to_string(),
                input: 10,
                output: 20,
                cache_read: 30,
                cache_write: 40,
                reasoning: 50,
                duration_ms: None,
                message_count: 1,
                agent: Some("fixture-agent".to_string()),
            },
            Some(&pricing),
        );

        assert_eq!(event.cost_status, CostStatus::Estimated);
        let cost = event.estimated_cost_usd.unwrap();
        assert!((cost - 0.173).abs() < f64::EPSILON);
    }

    #[test]
    fn usage_event_keeps_missing_cost_without_pricing() {
        let event = usage_event_from_tokscale_message(
            tokscale_core::ParsedMessage {
                client: "codex".to_string(),
                model_id: "fixture-model".to_string(),
                provider_id: "openai".to_string(),
                session_id: "session-1".to_string(),
                workspace_key: None,
                workspace_label: None,
                timestamp: 1_777_546_802,
                date: "2026-04-29".to_string(),
                input: 10,
                output: 20,
                cache_read: 30,
                cache_write: 40,
                reasoning: 50,
                duration_ms: None,
                message_count: 1,
                agent: Some("fixture-agent".to_string()),
            },
            None,
        );

        assert_eq!(event.cost_status, CostStatus::Missing);
        assert_eq!(event.estimated_cost_usd, None);
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
    fn scan_maps_gemini_cache_only_and_reasoning_only_events() {
        let home = tempfile::tempdir().unwrap();
        let chats_dir = home.path().join(".gemini/tmp/project-1/chats");
        fs::create_dir_all(&chats_dir).unwrap();
        fs::write(
            chats_dir.join("chat-edge-buckets.json"),
            r#"{
  "sessionId": "gemini-session-edge",
  "projectHash": "project-1",
  "startTime": "2026-04-02T10:00:00.000Z",
  "lastUpdated": "2026-04-02T10:00:02.000Z",
  "messages": [
    {
      "id": "msg-cache-only",
      "timestamp": "2026-04-02T10:00:01.000Z",
      "type": "assistant",
      "model": "gemini-2.5-pro",
      "tokens": {
        "cached": 15
      }
    },
    {
      "id": "msg-reasoning-only",
      "timestamp": "2026-04-02T10:00:02.000Z",
      "type": "assistant",
      "model": "gemini-2.5-pro",
      "tokens": {
        "thoughts": 9
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

        assert_eq!(report.events.len(), 2);
        let cache_only = report
            .events
            .iter()
            .find(|event| event.tokens.cache_read == 15)
            .expect("cache-only event");
        assert_eq!(cache_only.tokens.input, 0);
        assert_eq!(cache_only.tokens.output, 0);
        assert_eq!(cache_only.tokens.reasoning, 0);
        assert_eq!(cache_only.normalized_total_tokens(), 15);

        let reasoning_only = report
            .events
            .iter()
            .find(|event| event.tokens.reasoning == 9)
            .expect("reasoning-only event");
        assert_eq!(reasoning_only.tokens.input, 0);
        assert_eq!(reasoning_only.tokens.output, 0);
        assert_eq!(reasoning_only.tokens.cache_read, 0);
        assert_eq!(reasoning_only.normalized_total_tokens(), 9);
    }

    #[test]
    fn scan_preserves_unknown_model_label_from_gemini_source() {
        let home = tempfile::tempdir().unwrap();
        let chats_dir = home.path().join(".gemini/tmp/project-1/chats");
        fs::create_dir_all(&chats_dir).unwrap();
        fs::write(
            chats_dir.join("chat-unknown-model.json"),
            r#"{
  "sessionId": "gemini-session-unknown-model",
  "projectHash": "project-1",
  "startTime": "2026-04-03T10:00:00.000Z",
  "lastUpdated": "2026-04-03T10:00:01.000Z",
  "messages": [
    {
      "id": "msg-unknown-model",
      "timestamp": "2026-04-03T10:00:01.000Z",
      "type": "assistant",
      "model": "unknown-local-model",
      "tokens": {
        "input": 1,
        "output": 2
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
        assert_eq!(
            report.events[0].model.as_deref(),
            Some("unknown-local-model")
        );
        assert_eq!(report.events[0].tokens.input, 1);
        assert_eq!(report.events[0].tokens.output, 2);
    }

    #[test]
    fn scan_keeps_cross_day_gemini_events_and_applies_day_window() {
        let home = tempfile::tempdir().unwrap();
        write_gemini_cross_day_fixture(home.path());
        let backend = TokscaleUsageBackend::with_home_dir(home.path());

        let all = backend
            .scan(UsageScanOptions {
                clients: vec!["gemini".to_string()],
                since_unix: None,
                until_unix: None,
                budget: UsageScanBudget::default(),
            })
            .unwrap();

        assert_eq!(all.events.len(), 2);
        assert!(
            all.events
                .iter()
                .any(|event| event.occurred_at_unix == 1_776_211_199)
        );
        assert!(
            all.events
                .iter()
                .any(|event| event.occurred_at_unix == 1_776_211_201)
        );

        let second_day = backend
            .scan(UsageScanOptions {
                clients: vec!["gemini".to_string()],
                since_unix: Some(1_776_211_200),
                until_unix: Some(1_776_297_600),
                budget: UsageScanBudget::default(),
            })
            .unwrap();

        assert_eq!(second_day.events.len(), 1);
        assert_eq!(second_day.events[0].occurred_at_unix, 1_776_211_201);
        assert_eq!(second_day.events[0].tokens.input, 20);
        assert_eq!(second_day.events[0].tokens.output, 2);
    }

    #[test]
    fn scan_assigns_same_event_hash_to_repeated_gemini_messages() {
        let home = tempfile::tempdir().unwrap();
        let chats_dir = home.path().join(".gemini/tmp/project-1/chats");
        fs::create_dir_all(&chats_dir).unwrap();
        fs::write(
            chats_dir.join("chat-duplicate-message.json"),
            r#"{
  "sessionId": "gemini-session-duplicate",
  "projectHash": "project-1",
  "startTime": "2026-04-04T10:00:00.000Z",
  "lastUpdated": "2026-04-04T10:00:02.000Z",
  "messages": [
    {
      "id": "msg-duplicate",
      "timestamp": "2026-04-04T10:00:01.000Z",
      "type": "assistant",
      "model": "gemini-2.5-pro",
      "tokens": {
        "input": 10,
        "output": 1
      }
    },
    {
      "id": "msg-duplicate",
      "timestamp": "2026-04-04T10:00:01.000Z",
      "type": "assistant",
      "model": "gemini-2.5-pro",
      "tokens": {
        "input": 10,
        "output": 1
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

        assert_eq!(report.events.len(), 2);
        assert_eq!(report.events[0].event_hash, report.events[1].event_hash);
        assert_eq!(report.events[0].tokens.input, 10);
        assert_eq!(report.events[0].tokens.output, 1);
    }

    #[test]
    fn scan_applies_openmux_time_window_after_tokscale_parse() {
        let home = tempfile::tempdir().unwrap();
        write_codex_session_fixture(home.path());
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

    #[test]
    fn tokscale_timestamp_normalization_preserves_seconds_and_converts_millis() {
        assert_eq!(
            tokscale_timestamp_to_unix_seconds(1_777_546_802),
            1_777_546_802
        );
        assert_eq!(
            tokscale_timestamp_to_unix_seconds(1_777_546_802_000),
            1_777_546_802
        );
    }

    fn write_codex_session_fixture(home: &std::path::Path) {
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

    fn write_gemini_cross_day_fixture(home: &std::path::Path) {
        let chats_dir = home.join(".gemini/tmp/project-1/chats");
        fs::create_dir_all(&chats_dir).unwrap();
        fs::write(
            chats_dir.join("chat-cross-day.json"),
            r#"{
  "sessionId": "gemini-session-cross-day",
  "projectHash": "project-1",
  "startTime": "2026-04-14T23:59:59.000Z",
  "lastUpdated": "2026-04-15T00:00:01.000Z",
  "messages": [
    {
      "id": "msg-before-midnight",
      "timestamp": "2026-04-14T23:59:59.000Z",
      "type": "assistant",
      "model": "gemini-2.5-pro",
      "tokens": {
        "input": 10,
        "output": 1
      }
    },
    {
      "id": "msg-after-midnight",
      "timestamp": "2026-04-15T00:00:01.000Z",
      "type": "assistant",
      "model": "gemini-2.5-pro",
      "tokens": {
        "input": 20,
        "output": 2
      }
    }
  ]
}"#,
        )
        .unwrap();
    }
}
