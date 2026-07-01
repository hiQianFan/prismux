use omx_app::{
    ClientDescriptor, ConsumeResetCreditCommand, DashboardQuery, ImportProfileCommand,
    LoginCommand, RefreshCommand, RemoveCommand, SaveExistingLoginCommand, SupportReportCommand,
    SwitchCommand, UpdateSettingsCommand, about_view, activate_target, compatibility_view,
    consume_reset_credit, dashboard_view, import_profile, login_account, refresh_provider,
    remove_target, save_existing_login, settings_view, support_report, update_settings,
};
use omx_core::{
    PlatformPlugin, StateStore, UsageScanBudget, UsageScanOptions,
    storage::{read_file, state_root, write_file_atomic_private},
};
use omx_plugin_claude::ClaudePlugin;
use omx_plugin_codex::CodexPlugin;
use omx_usage_tokscale::TokscaleUsageBackend;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    ffi::{CStr, CString, c_char},
    panic::{AssertUnwindSafe, catch_unwind},
};

const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Deserialize)]
struct RequestEnvelope {
    schema_version: u32,
    op: String,
    #[serde(default)]
    payload: Value,
    #[serde(default)]
    request_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct ResponseEnvelope {
    schema_version: u32,
    control_plane_schema_version: u32,
    state_schema_version: u32,
    minimum_backend_version: String,
    minimum_frontend_version: String,
    ok: bool,
    #[serde(skip_serializing_if = "is_false")]
    data_stale: bool,
    #[serde(skip_serializing_if = "is_false")]
    served_from_snapshot: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<ResponseError>,
    #[serde(skip_serializing_if = "Option::is_none")]
    request_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct ResponseError {
    code: String,
    message: String,
}

struct DispatchSuccess {
    data: Value,
    data_stale: bool,
    served_from_snapshot: bool,
}

impl DispatchSuccess {
    fn fresh(data: Value) -> Self {
        Self {
            data,
            data_stale: false,
            served_from_snapshot: false,
        }
    }

    fn snapshot(data: Value) -> Self {
        Self {
            data,
            data_stale: true,
            served_from_snapshot: true,
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn omx_menubar_call(request_json: *const c_char) -> *mut c_char {
    let response = catch_unwind(AssertUnwindSafe(|| call_from_ptr(request_json)))
        .unwrap_or_else(|_| error_envelope(None, "panic", "internal menubar backend error"));
    cstring(response).into_raw()
}

/// # Safety
///
/// `value` must be a pointer returned by `omx_menubar_call` and must be freed at most once.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn omx_menubar_free(value: *mut c_char) {
    if value.is_null() {
        return;
    }
    unsafe {
        drop(CString::from_raw(value));
    }
}

pub fn call_json(request_json: &str) -> String {
    catch_unwind(AssertUnwindSafe(|| call_request(request_json)))
        .unwrap_or_else(|_| error_envelope(None, "panic", "internal menubar backend error"))
}

fn call_from_ptr(request_json: *const c_char) -> String {
    if request_json.is_null() {
        return error_envelope(None, "null_request", "request pointer was null");
    }
    let request = unsafe { CStr::from_ptr(request_json) };
    match request.to_str() {
        Ok(value) => call_request(value),
        Err(_) => error_envelope(None, "bad_utf8", "request was not valid UTF-8"),
    }
}

fn call_request(request_json: &str) -> String {
    let request: RequestEnvelope = match serde_json::from_str(request_json) {
        Ok(request) => request,
        Err(_) => return error_envelope(None, "bad_json", "request was not valid JSON"),
    };
    if request.schema_version != SCHEMA_VERSION {
        return error_envelope(
            request.request_id,
            "unsupported_schema",
            "unsupported menubar schema_version",
        );
    }
    let request_id = request.request_id.clone();
    match dispatch(request) {
        Ok(success) => encode(ResponseEnvelope {
            schema_version: SCHEMA_VERSION,
            control_plane_schema_version: omx_app::compatibility::CONTROL_PLANE_SCHEMA_VERSION,
            state_schema_version: omx_app::compatibility::STATE_SCHEMA_VERSION,
            minimum_backend_version: omx_app::compatibility::MIN_BACKEND_VERSION.to_string(),
            minimum_frontend_version: omx_app::compatibility::MIN_FRONTEND_VERSION.to_string(),
            ok: true,
            data_stale: success.data_stale,
            served_from_snapshot: success.served_from_snapshot,
            data: Some(success.data),
            error: None,
            request_id,
        }),
        Err(err) => error_envelope(request_id, err.0, err.1),
    }
}

fn dispatch(request: RequestEnvelope) -> Result<DispatchSuccess, (&'static str, String)> {
    #[cfg(test)]
    if request.op == "__panic" {
        panic!("test panic");
    }
    let plugins = default_plugins();
    let store = state_root()
        .ok()
        .and_then(|root| StateStore::open(&root).ok());
    match request.op.as_str() {
        "accounts" => {
            let query: DashboardQuery = payload_or_default(request.payload)?;
            json_value(omx_app::menubar_accounts(&plugins, query)).map(DispatchSuccess::fresh)
        }
        "dashboard" => {
            let query: DashboardQuery = payload_or_default(request.payload)?;
            refresh_usage_cache(
                store.as_ref(),
                query.provider.as_deref(),
                query.usage_period.as_ref(),
            );
            match json_value(dashboard_view(&plugins, query, store.as_ref())) {
                Ok(value) => {
                    persist_dashboard_snapshot(&value);
                    Ok(DispatchSuccess::fresh(value))
                }
                Err(err) => load_dashboard_snapshot()
                    .map(DispatchSuccess::snapshot)
                    .ok_or(err),
            }
        }
        "switch" => {
            let command: SwitchCommand = payload(request.payload)?;
            refresh_usage_cache(
                store.as_ref(),
                Some(&command.provider),
                command.usage_period.as_ref(),
            );
            json_value(activate_target(&plugins, command, store.as_ref()))
                .map(DispatchSuccess::fresh)
        }
        "refresh" => {
            let command: RefreshCommand = payload(request.payload)?;
            refresh_usage_cache(
                store.as_ref(),
                Some(&command.provider),
                command.usage_period.as_ref(),
            );
            json_value(refresh_provider(&plugins, command, store.as_ref()))
                .map(DispatchSuccess::fresh)
        }
        "remove" => {
            let command: RemoveCommand = payload(request.payload)?;
            refresh_usage_cache(
                store.as_ref(),
                Some(&command.provider),
                command.usage_period.as_ref(),
            );
            json_value(remove_target(&plugins, command, store.as_ref())).map(DispatchSuccess::fresh)
        }
        "consume_reset_credit" => {
            let command: ConsumeResetCreditCommand = payload(request.payload)?;
            refresh_usage_cache(
                store.as_ref(),
                Some(&command.provider),
                command.usage_period.as_ref(),
            );
            json_value(consume_reset_credit(&plugins, command, store.as_ref()))
                .map(DispatchSuccess::fresh)
        }
        "login" => {
            let command: LoginCommand = payload(request.payload)?;
            json_value(login_account(&plugins, command, store.as_ref())).map(DispatchSuccess::fresh)
        }
        "save_existing_login" => {
            let command: SaveExistingLoginCommand = payload(request.payload)?;
            json_value(save_existing_login(&plugins, command, store.as_ref()))
                .map(DispatchSuccess::fresh)
        }
        "import_profile" => {
            let command: ImportProfileCommand = payload(request.payload)?;
            json_value(import_profile(&plugins, command, store.as_ref()))
                .map(DispatchSuccess::fresh)
        }
        "cancel_login" => {
            // Runs on its own FFI call/thread while `login` holds the operation
            // lock; only flips the cancel flag so the parked login child dies.
            omx_core::request_login_cancel();
            Ok(DispatchSuccess::fresh(
                serde_json::json!({ "cancelled": true }),
            ))
        }
        "compatibility" => {
            let client: ClientDescriptor = payload_or_default(request.payload)?;
            json_value(Ok(compatibility_view(client))).map(DispatchSuccess::fresh)
        }
        "settings_view" => json_value(settings_view()).map(DispatchSuccess::fresh),
        "update_settings" => {
            let command: UpdateSettingsCommand = payload(request.payload)?;
            json_value(update_settings(command)).map(DispatchSuccess::fresh)
        }
        "about_view" => json_value(about_view()).map(DispatchSuccess::fresh),
        "support_report" => {
            let command: SupportReportCommand = payload_or_default(request.payload)?;
            serde_json::to_value(support_report(command))
                .map(DispatchSuccess::fresh)
                .map_err(|_| ("encode_error", "failed to encode response".to_string()))
        }
        _ => Err(("unknown_op", "unknown menubar operation".to_string())),
    }
}

fn snapshot_path() -> omx_core::Result<std::path::PathBuf> {
    Ok(state_root()?
        .join("control-plane")
        .join("dashboard.last-good.json"))
}

fn persist_dashboard_snapshot(value: &Value) {
    if let Ok(path) = snapshot_path()
        && let Ok(bytes) = serde_json::to_vec(value)
    {
        let _ = write_file_atomic_private(&path, &bytes);
    }
}

fn load_dashboard_snapshot() -> Option<Value> {
    let path = snapshot_path().ok()?;
    let bytes = read_file(&path).ok()?;
    let mut value: Value = serde_json::from_slice(&bytes).ok()?;
    mark_stale(&mut value);
    Some(value)
}

fn mark_stale(value: &mut Value) {
    match value {
        Value::Object(object) => {
            if let Some(freshness) = object.get_mut("freshness")
                && let Value::Object(freshness) = freshness
            {
                freshness.insert("stale".to_string(), Value::Bool(true));
            }
            for value in object.values_mut() {
                mark_stale(value);
            }
        }
        Value::Array(values) => {
            for value in values {
                mark_stale(value);
            }
        }
        _ => {}
    }
}

fn default_plugins() -> Vec<Box<dyn PlatformPlugin>> {
    vec![Box::new(CodexPlugin::new()), Box::new(ClaudePlugin::new())]
}

fn refresh_usage_cache(
    store: Option<&StateStore>,
    provider: Option<&str>,
    period: Option<&omx_core::UsagePeriod>,
) {
    let Some(store) = store else {
        return;
    };
    let now = omx_core::storage::unix_now();
    let scan = TokscaleUsageBackend::new().scan(UsageScanOptions {
        clients: provider
            .map(|value| vec![value.to_string()])
            .unwrap_or_default(),
        since_unix: usage_refresh_since_unix(period, now),
        until_unix: None,
        budget: UsageScanBudget::default(),
    });
    if let Ok(report) = scan {
        let _ = store.ingest_usage_events(&report.events, None, omx_core::storage::unix_now());
    }
}

fn usage_refresh_since_unix(period: Option<&omx_core::UsagePeriod>, now: u64) -> Option<i64> {
    let days = match period.unwrap_or(&omx_core::UsagePeriod::Today) {
        omx_core::UsagePeriod::Today => 1,
        omx_core::UsagePeriod::SevenDays => 7,
        omx_core::UsagePeriod::ThirtyDays => 30,
        omx_core::UsagePeriod::All | omx_core::UsagePeriod::Custom => return None,
    };
    Some(now.saturating_sub(days * 86_400).min(i64::MAX as u64) as i64)
}

fn payload<T: serde::de::DeserializeOwned>(value: Value) -> Result<T, (&'static str, String)> {
    serde_json::from_value(value)
        .map_err(|_| ("bad_payload", "payload did not match operation".to_string()))
}

fn payload_or_default<T>(value: Value) -> Result<T, (&'static str, String)>
where
    T: serde::de::DeserializeOwned + Default,
{
    if value.is_null() || value == json!({}) {
        return Ok(T::default());
    }
    payload(value)
}

fn json_value<T: Serialize>(result: omx_core::Result<T>) -> Result<Value, (&'static str, String)> {
    let value = result.map_err(|err| ("application_error", sanitize(&err.to_string())))?;
    serde_json::to_value(value)
        .map_err(|_| ("encode_error", "failed to encode response".to_string()))
}

fn error_envelope(
    request_id: Option<String>,
    code: impl Into<String>,
    message: impl Into<String>,
) -> String {
    encode(ResponseEnvelope {
        schema_version: SCHEMA_VERSION,
        control_plane_schema_version: omx_app::compatibility::CONTROL_PLANE_SCHEMA_VERSION,
        state_schema_version: omx_app::compatibility::STATE_SCHEMA_VERSION,
        minimum_backend_version: omx_app::compatibility::MIN_BACKEND_VERSION.to_string(),
        minimum_frontend_version: omx_app::compatibility::MIN_FRONTEND_VERSION.to_string(),
        ok: false,
        data_stale: false,
        served_from_snapshot: false,
        data: None,
        error: Some(ResponseError {
            code: code.into(),
            message: sanitize(&message.into()),
        }),
        request_id,
    })
}

fn is_false(value: &bool) -> bool {
    !*value
}

fn encode(response: ResponseEnvelope) -> String {
    serde_json::to_string(&response).unwrap_or_else(|_| {
        "{\"schema_version\":1,\"control_plane_schema_version\":1,\"state_schema_version\":1,\"minimum_backend_version\":\"0.1.0\",\"minimum_frontend_version\":\"0.1.0\",\"ok\":false,\"error\":{\"code\":\"encode_error\",\"message\":\"failed to encode response\"}}".to_string()
    })
}

fn cstring(value: String) -> CString {
    CString::new(value).unwrap_or_else(|_| CString::new("{\"schema_version\":1,\"ok\":false,\"error\":{\"code\":\"nul_byte\",\"message\":\"response contained an invalid nul byte\"}}").unwrap())
}

fn sanitize(message: &str) -> String {
    omx_app::diagnostics::redaction::redact(message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use omx_core::SaveOptions;
    use std::fs;
    use std::ptr;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn bad_json_returns_error_envelope() {
        let payload: Value = serde_json::from_str(&call_json("{")).unwrap();

        assert_eq!(payload["schema_version"], 1);
        assert_eq!(payload["ok"], false);
        assert_eq!(payload["error"]["code"], "bad_json");
    }

    #[test]
    fn bad_utf8_returns_error_envelope() {
        let bytes = [0xff_u8, 0x00];
        let payload: Value = serde_json::from_str(&call_from_ptr(bytes.as_ptr().cast())).unwrap();

        assert_eq!(payload["ok"], false);
        assert_eq!(payload["error"]["code"], "bad_utf8");
    }

    #[test]
    fn panic_returns_error_envelope() {
        let payload: Value = serde_json::from_str(&call_json(
            r#"{"schema_version":1,"op":"__panic","payload":{}}"#,
        ))
        .unwrap();

        assert_eq!(payload["ok"], false);
        assert_eq!(payload["error"]["code"], "panic");
    }

    #[test]
    fn unsupported_schema_returns_safe_error() {
        let payload: Value = serde_json::from_str(&call_json(
            r#"{"schema_version":2,"op":"dashboard","payload":{}}"#,
        ))
        .unwrap();

        assert_eq!(payload["ok"], false);
        assert_eq!(payload["error"]["code"], "unsupported_schema");
    }

    #[test]
    fn unknown_op_returns_safe_error() {
        let payload: Value = serde_json::from_str(&call_json(
            r#"{"schema_version":1,"op":"missing","payload":{}}"#,
        ))
        .unwrap();

        assert_eq!(payload["ok"], false);
        assert_eq!(payload["error"]["code"], "unknown_op");
    }

    #[test]
    fn null_pointer_returns_error_envelope() {
        let raw = omx_menubar_call(ptr::null());
        let response = unsafe { CStr::from_ptr(raw) };
        let payload: Value = serde_json::from_str(response.to_str().unwrap()).unwrap();
        unsafe { omx_menubar_free(raw) };

        assert_eq!(payload["ok"], false);
        assert_eq!(payload["error"]["code"], "null_request");
    }

    #[test]
    fn accounts_returns_versioned_envelope() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|err| err.into_inner());
        let temp = tempfile::tempdir().unwrap();
        unsafe {
            std::env::set_var("OMUX_STATE_ROOT", temp.path());
            std::env::set_var("CODEX_HOME", temp.path().join("codex-home"));
        }
        let payload: Value = serde_json::from_str(&call_json(
            r#"{"schema_version":1,"op":"accounts","payload":{"provider":"codex"},"request_id":"r1"}"#,
        ))
        .unwrap();
        unsafe {
            std::env::remove_var("OMUX_STATE_ROOT");
            std::env::remove_var("CODEX_HOME");
        }

        assert_eq!(payload["schema_version"], 1);
        assert_eq!(payload["ok"], true);
        assert_eq!(payload["request_id"], "r1");
        assert!(payload["data"]["accounts"].is_array());
    }

    #[test]
    fn dashboard_without_provider_returns_all_default_providers() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|err| err.into_inner());
        let temp = tempfile::tempdir().unwrap();
        let previous_home = std::env::var_os("HOME");
        unsafe {
            std::env::set_var("HOME", temp.path());
            std::env::set_var("OMUX_STATE_ROOT", temp.path());
            std::env::set_var("CODEX_HOME", temp.path().join("codex-home"));
            std::env::set_var("CLAUDE_CONFIG_DIR", temp.path().join("claude-home"));
        }

        let payload: Value = serde_json::from_str(&call_json(
            r#"{"schema_version":1,"op":"dashboard","payload":{}}"#,
        ))
        .unwrap();

        unsafe {
            restore_env("HOME", previous_home);
            std::env::remove_var("OMUX_STATE_ROOT");
            std::env::remove_var("CODEX_HOME");
            std::env::remove_var("CLAUDE_CONFIG_DIR");
        }

        assert_eq!(payload["ok"], true);
        assert_eq!(payload["data"]["accounts"]["providers"][0], "codex");
        assert_eq!(payload["data"]["accounts"]["providers"][1], "claude");
    }

    #[test]
    fn ffi_dashboard_matches_control_plane_for_same_state_root() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|err| err.into_inner());
        let temp = tempfile::tempdir().unwrap();
        let previous_home = std::env::var_os("HOME");
        unsafe {
            std::env::set_var("HOME", temp.path());
            std::env::set_var("OMUX_STATE_ROOT", temp.path());
            std::env::set_var("CODEX_HOME", temp.path().join("codex-home"));
            std::env::set_var("CLAUDE_CONFIG_DIR", temp.path().join("claude-home"));
        }

        let plugins = default_plugins();
        let direct =
            omx_app::dashboard_view(&plugins, omx_app::DashboardQuery::default(), None).unwrap();
        let ffi: Value = serde_json::from_str(&call_json(
            r#"{"schema_version":1,"op":"dashboard","payload":{}}"#,
        ))
        .unwrap();

        unsafe {
            restore_env("HOME", previous_home);
            std::env::remove_var("OMUX_STATE_ROOT");
            std::env::remove_var("CODEX_HOME");
            std::env::remove_var("CLAUDE_CONFIG_DIR");
        }

        let ffi_providers = ffi["data"]["accounts"]["providers"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_str().unwrap().to_string())
            .collect::<Vec<_>>();
        assert_eq!(ffi["ok"], true);
        assert_eq!(ffi_providers, direct.accounts.providers);
        assert_eq!(
            ffi["data"]["accounts"]["accounts"]
                .as_array()
                .unwrap()
                .len(),
            direct.accounts.accounts.len()
        );
        assert_eq!(
            ffi["data"]["accounts"]["profiles"]
                .as_array()
                .unwrap()
                .len(),
            direct.accounts.profiles.len()
        );
        // Contract: the FFI surface and a direct control-plane call agree on the
        // schema version and on the full aggregate projection (quota health +
        // usage headline). The aggregate carries no per-call timestamp, so it is
        // safe to compare by value; the top-level report's generated_at_unix is
        // not.
        assert_eq!(
            ffi["data"]["control_plane_schema_version"],
            omx_app::compatibility::CONTROL_PLANE_SCHEMA_VERSION
        );
        assert_eq!(direct.control_plane_schema_version, 4);
        assert_eq!(
            ffi["data"]["aggregate"],
            serde_json::to_value(&direct.aggregate).unwrap()
        );
    }

    #[test]
    fn dashboard_falls_back_to_last_good_snapshot() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|err| err.into_inner());
        let temp = tempfile::tempdir().unwrap();
        unsafe {
            std::env::set_var("OMUX_STATE_ROOT", temp.path());
            std::env::set_var("CODEX_HOME", temp.path().join("codex-home"));
        }

        let live: Value = serde_json::from_str(&call_json(
            r#"{"schema_version":1,"op":"dashboard","payload":{"provider":"codex"}}"#,
        ))
        .unwrap();
        let fallback: Value = serde_json::from_str(&call_json(
            r#"{"schema_version":1,"op":"dashboard","payload":{"provider":"missing"}}"#,
        ))
        .unwrap();

        unsafe {
            std::env::remove_var("OMUX_STATE_ROOT");
            std::env::remove_var("CODEX_HOME");
        }

        assert_eq!(live["ok"], true);
        assert_ne!(live["data_stale"], true);
        assert_ne!(live["served_from_snapshot"], true);
        assert_eq!(fallback["ok"], true);
        assert_eq!(fallback["data_stale"], true);
        assert_eq!(fallback["served_from_snapshot"], true);
        assert_eq!(fallback["data"]["accounts"]["providers"][0], "codex");
    }

    #[test]
    fn dashboard_accepts_usage_period_payload() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|err| err.into_inner());
        let temp = tempfile::tempdir().unwrap();
        unsafe {
            std::env::set_var("OMUX_STATE_ROOT", temp.path());
            std::env::set_var("CODEX_HOME", temp.path().join("codex-home"));
        }

        let payload: Value = serde_json::from_str(&call_json(
            r#"{"schema_version":1,"op":"dashboard","payload":{"usage_period":"Today"}}"#,
        ))
        .unwrap();

        unsafe {
            std::env::remove_var("OMUX_STATE_ROOT");
            std::env::remove_var("CODEX_HOME");
        }

        assert_eq!(payload["ok"], true);
        assert_eq!(payload["data"]["usage"]["period"], "Today");
    }

    #[test]
    fn dashboard_usage_headline_stays_today_when_chart_period_changes() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|err| err.into_inner());
        let temp = tempfile::tempdir().unwrap();
        unsafe {
            std::env::set_var("OMUX_STATE_ROOT", temp.path());
            std::env::set_var("CODEX_HOME", temp.path().join("codex-home"));
        }

        let payload: Value = serde_json::from_str(&call_json(
            r#"{"schema_version":1,"op":"dashboard","payload":{"usage_period":"ThirtyDays"}}"#,
        ))
        .unwrap();

        unsafe {
            std::env::remove_var("OMUX_STATE_ROOT");
            std::env::remove_var("CODEX_HOME");
        }

        assert_eq!(payload["ok"], true);
        assert_eq!(payload["data"]["usage"]["period"], "ThirtyDays");
        assert_eq!(
            payload["data"]["aggregate"]["usage_headline"]["period"],
            "Today"
        );
    }

    #[test]
    fn dashboard_refreshes_usage_cache_from_local_codex_sessions() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|err| err.into_inner());
        let temp = tempfile::tempdir().unwrap();
        let state_root = temp.path().join("openmux-state");
        let codex_home = temp.path().join("codex-home");
        let sessions = codex_home.join("sessions");
        let previous_home = std::env::var_os("HOME");
        fs::create_dir_all(&sessions).unwrap();
        fs::write(
            sessions.join("session-1.jsonl"),
            concat!(
                r#"{"type":"session_meta","payload":{"id":"session-1","source":"interactive","model_provider":"openai","cwd":"/tmp/openmux-project"}}"#,
                "\n",
                r#"{"type":"turn_context","payload":{"model":"gpt-5"}}"#,
                "\n",
                r#"{"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":10,"cached_input_tokens":2,"output_tokens":3,"reasoning_output_tokens":4},"last_token_usage":{"input_tokens":10,"cached_input_tokens":2,"output_tokens":3,"reasoning_output_tokens":4}}}}"#,
                "\n",
            ),
        )
        .unwrap();

        unsafe {
            std::env::set_var("HOME", temp.path());
            std::env::set_var("OMUX_STATE_ROOT", &state_root);
            std::env::set_var("CODEX_HOME", &codex_home);
        }

        let payload: Value = serde_json::from_str(&call_json(
            r#"{"schema_version":1,"op":"dashboard","payload":{"provider":"codex"}}"#,
        ))
        .unwrap();

        unsafe {
            restore_env("HOME", previous_home);
            std::env::remove_var("OMUX_STATE_ROOT");
            std::env::remove_var("CODEX_HOME");
        }

        assert_eq!(payload["ok"], true);
        assert_eq!(payload["data"]["usage"]["total_tokens"], 17);
        assert_eq!(payload["data"]["usage"]["top_client"], "codex");
        assert_eq!(payload["data"]["usage"]["top_model"], "gpt-5");
        assert_eq!(payload["data"]["usage"]["coverage"]["status"], "complete");
    }

    #[test]
    fn usage_refresh_since_matches_selected_period() {
        let now = 4_102_531_200;

        assert_eq!(usage_refresh_since_unix(None, now), Some(4_102_444_800));
        assert_eq!(
            usage_refresh_since_unix(Some(&omx_core::UsagePeriod::SevenDays), now),
            Some(4_101_926_400)
        );
        assert_eq!(
            usage_refresh_since_unix(Some(&omx_core::UsagePeriod::ThirtyDays), now),
            Some(4_099_939_200)
        );
        assert_eq!(
            usage_refresh_since_unix(Some(&omx_core::UsagePeriod::All), now),
            None
        );
    }

    #[test]
    fn ffi_success_response_can_be_freed_once() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|err| err.into_inner());
        let temp = tempfile::tempdir().unwrap();
        unsafe {
            std::env::set_var("OMUX_STATE_ROOT", temp.path());
            std::env::set_var("CODEX_HOME", temp.path().join("codex-home"));
        }
        let request =
            CString::new(r#"{"schema_version":1,"op":"accounts","payload":{"provider":"codex"}}"#)
                .unwrap();
        let raw = omx_menubar_call(request.as_ptr());
        let payload: Value =
            serde_json::from_str(unsafe { CStr::from_ptr(raw) }.to_str().unwrap()).unwrap();
        unsafe { omx_menubar_free(raw) };
        unsafe {
            std::env::remove_var("OMUX_STATE_ROOT");
            std::env::remove_var("CODEX_HOME");
        }

        assert_eq!(payload["ok"], true);
    }

    #[test]
    fn request_contract_ignores_additive_optional_fields() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|err| err.into_inner());
        let temp = tempfile::tempdir().unwrap();
        unsafe {
            std::env::set_var("OMUX_STATE_ROOT", temp.path());
            std::env::set_var("CODEX_HOME", temp.path().join("codex-home"));
        }
        let payload: Value = serde_json::from_str(&call_json(
            r#"{"schema_version":1,"op":"accounts","payload":{"provider":"codex","future_optional":"ignored"},"future_envelope_field":true}"#,
        ))
        .unwrap();
        unsafe {
            std::env::remove_var("OMUX_STATE_ROOT");
            std::env::remove_var("CODEX_HOME");
        }

        assert_eq!(payload["ok"], true);
    }

    #[test]
    fn application_error_is_redacted() {
        let payload: Value = serde_json::from_str(&call_json(
            r#"{"schema_version":1,"op":"switch","payload":{"provider":"access_token-secret","local_id":"missing"}}"#,
        ))
        .unwrap();

        assert_eq!(payload["ok"], false);
        assert_eq!(payload["error"]["code"], "application_error");
        assert_eq!(
            payload["error"]["message"],
            "[redacted sensitive diagnostic]"
        );
    }

    #[test]
    fn accounts_dashboard_switch_and_refresh_match_golden_fixtures() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|err| err.into_inner());
        omx_app::reset_refresh_state_for_tests();
        let temp = tempfile::tempdir().unwrap();
        unsafe {
            std::env::set_var("OMUX_STATE_ROOT", temp.path());
            std::env::set_var("CODEX_HOME", temp.path().join("codex-home"));
        }
        let cases = [
            (
                r#"{"schema_version":1,"op":"accounts","payload":{"provider":"codex"},"request_id":"accounts-fixture"}"#,
                include_str!("../fixtures/menubar/accounts.response.json"),
                "fixtures/menubar/accounts.response.json",
            ),
            (
                r#"{"schema_version":1,"op":"dashboard","payload":{"provider":"codex"},"request_id":"dashboard-fixture"}"#,
                include_str!("../fixtures/menubar/dashboard.response.json"),
                "fixtures/menubar/dashboard.response.json",
            ),
            (
                r#"{"schema_version":1,"op":"switch","payload":{"provider":"codex","local_id":"missing-local-id"},"request_id":"switch-fixture"}"#,
                include_str!("../fixtures/menubar/switch.response.json"),
                "fixtures/menubar/switch.response.json",
            ),
            (
                r#"{"schema_version":1,"op":"refresh","payload":{"provider":"codex","kind":"interactive"},"request_id":"refresh-fixture"}"#,
                include_str!("../fixtures/menubar/refresh.response.json"),
                "fixtures/menubar/refresh.response.json",
            ),
        ];

        for (request, fixture, path) in cases {
            let mut actual: Value = serde_json::from_str(&call_json(request)).unwrap();
            scrub_generated_at(&mut actual);
            if std::env::var_os("OMX_UPDATE_FIXTURES").is_some() {
                std::fs::write(
                    path,
                    format!("{}\n", serde_json::to_string_pretty(&actual).unwrap()),
                )
                .unwrap();
                continue;
            }
            let expected: Value = serde_json::from_str(fixture).unwrap();
            assert_eq!(actual, expected, "{request}");
        }

        unsafe {
            std::env::remove_var("OMUX_STATE_ROOT");
            std::env::remove_var("CODEX_HOME");
        }
    }

    #[test]
    fn ffi_uses_temp_state_and_codex_home_for_accounts_dashboard_refresh_and_switch() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|err| err.into_inner());
        let temp = tempfile::tempdir().unwrap();
        let state_root = temp.path().join("openmux-state");
        let codex_home = temp.path().join("codex-home");
        fs::create_dir_all(&codex_home).unwrap();

        unsafe {
            std::env::set_var("OMUX_STATE_ROOT", &state_root);
            std::env::set_var("CODEX_HOME", &codex_home);
        }

        fs::write(codex_home.join("auth.json"), br#"{"account":"work"}"#).unwrap();
        let plugin = CodexPlugin::new();
        let work = plugin
            .save_current(SaveOptions {
                alias: Some("work".to_string()),
            })
            .unwrap();
        fs::write(codex_home.join("auth.json"), br#"{"account":"personal"}"#).unwrap();
        plugin
            .save_current(SaveOptions {
                alias: Some("personal".to_string()),
            })
            .unwrap();

        let accounts: Value = serde_json::from_str(&call_json(
            r#"{"schema_version":1,"op":"accounts","payload":{"provider":"codex"}}"#,
        ))
        .unwrap();
        assert_eq!(accounts["ok"], true);
        assert_eq!(accounts["data"]["accounts"].as_array().unwrap().len(), 2);

        let dashboard: Value = serde_json::from_str(&call_json(
            r#"{"schema_version":1,"op":"dashboard","payload":{"provider":"codex"}}"#,
        ))
        .unwrap();
        assert_eq!(dashboard["ok"], true);
        assert_eq!(dashboard["data"]["accounts"]["providers"][0], "codex");

        let refresh: Value = serde_json::from_str(&call_json(
            r#"{"schema_version":1,"op":"refresh","payload":{"provider":"codex","kind":"interactive"}}"#,
        ))
        .unwrap();
        assert_eq!(refresh["ok"], true);

        let switch_request = json!({
            "schema_version": 1,
            "op": "switch",
            "payload": {
                "provider": "codex",
                "local_id": work.local_id,
            }
        })
        .to_string();
        let switched: Value = serde_json::from_str(&call_json(&switch_request)).unwrap();
        assert_eq!(switched["ok"], true);
        assert_eq!(
            fs::read(codex_home.join("auth.json")).unwrap(),
            br#"{"account":"work"}"#
        );
        assert_eq!(
            switched["data"]["dashboard"]["active"]["local_id"].as_str(),
            Some(work.local_id.as_str())
        );
        assert!(
            state_root
                .join("platforms")
                .join("codex")
                .join("backups")
                .exists()
        );

        unsafe {
            std::env::remove_var("OMUX_STATE_ROOT");
            std::env::remove_var("CODEX_HOME");
        }
    }

    fn scrub_generated_at(value: &mut Value) {
        match value {
            Value::Object(object) => {
                if object.contains_key("generated_at_unix") {
                    object.insert("generated_at_unix".to_string(), Value::from(0));
                }
                for value in object.values_mut() {
                    scrub_generated_at(value);
                }
            }
            Value::Array(values) => {
                for value in values {
                    scrub_generated_at(value);
                }
            }
            _ => {}
        }
    }

    unsafe fn restore_env(key: &str, value: Option<std::ffi::OsString>) {
        if let Some(value) = value {
            unsafe { std::env::set_var(key, value) };
        } else {
            unsafe { std::env::remove_var(key) };
        }
    }
}
