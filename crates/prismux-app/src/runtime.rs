use crate::dto::RefreshKind;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex},
};

#[doc(hidden)]
pub fn reset_refresh_state_for_tests() {
    REFRESH_STATE
        .lock()
        .unwrap_or_else(|err| err.into_inner())
        .clear();
}

#[derive(Debug, Clone, Default)]
struct RefreshState {
    generation: u64,
    in_flight: bool,
    last_attempt_unix: Option<u64>,
    last_success_unix: Option<u64>,
    last_error_unix: Option<u64>,
}

static REFRESH_STATE: LazyLock<Mutex<HashMap<String, RefreshState>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

const INTERACTIVE_REFRESH_FLOOR_SECONDS: u64 = 30;
const BACKGROUND_REFRESH_FLOOR_SECONDS: u64 = 300;
const REFRESH_ERROR_BACKOFF_SECONDS: u64 = 120;
pub const DEFAULT_REFRESH_TIMEOUT_SECONDS: u64 = 45;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderRuntimeView {
    pub provider: String,
    pub lifecycle: Vec<ProviderRuntimeLifecycle>,
    pub refresh_in_flight: bool,
    pub refresh_eligible: bool,
    pub timeout_seconds: u64,
    pub backoff_until_unix: Option<u64>,
    pub last_success_unix: Option<u64>,
    pub last_failure_unix: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderRuntimeLifecycle {
    Detected,
    Enabled,
    Available,
    RefreshEligible,
    InFlight,
    Cancelled,
    TimedOut,
    Backoff,
    LastSuccess,
    LastFailure,
}

pub fn provider_runtime_view(provider: &str, now: u64) -> ProviderRuntimeView {
    let states = REFRESH_STATE.lock().unwrap_or_else(|err| err.into_inner());
    let state = states.get(provider).cloned().unwrap_or_default();
    let backoff_until_unix = state
        .last_error_unix
        .map(|last_error| last_error.saturating_add(REFRESH_ERROR_BACKOFF_SECONDS))
        .filter(|backoff_until| *backoff_until > now);
    let refresh_eligible = !state.in_flight && backoff_until_unix.is_none();
    let mut lifecycle = vec![
        ProviderRuntimeLifecycle::Detected,
        ProviderRuntimeLifecycle::Enabled,
        ProviderRuntimeLifecycle::Available,
    ];
    if refresh_eligible {
        lifecycle.push(ProviderRuntimeLifecycle::RefreshEligible);
    }
    if state.in_flight {
        lifecycle.push(ProviderRuntimeLifecycle::InFlight);
    }
    if backoff_until_unix.is_some() {
        lifecycle.push(ProviderRuntimeLifecycle::Backoff);
    }
    if state.last_success_unix.is_some() {
        lifecycle.push(ProviderRuntimeLifecycle::LastSuccess);
    }
    if state.last_error_unix.is_some() {
        lifecycle.push(ProviderRuntimeLifecycle::LastFailure);
    }
    ProviderRuntimeView {
        provider: provider.to_string(),
        lifecycle,
        refresh_in_flight: state.in_flight,
        refresh_eligible,
        timeout_seconds: DEFAULT_REFRESH_TIMEOUT_SECONDS,
        backoff_until_unix,
        last_success_unix: state.last_success_unix,
        last_failure_unix: state.last_error_unix,
    }
}

pub fn refresh_failure_gate(provider: &str, now: u64) -> Option<String> {
    let states = REFRESH_STATE.lock().unwrap_or_else(|err| err.into_inner());
    let state = states.get(provider)?;
    state.last_error_unix.and_then(|last_error| {
        (now.saturating_sub(last_error) < REFRESH_ERROR_BACKOFF_SECONDS)
            .then(|| "error_backoff".to_string())
    })
}

pub fn record_refresh_timeout(provider: &str, generation: u64, now: u64) {
    record_refresh_result(provider, generation, now, false);
}

pub(crate) fn refresh_skip_reason(provider: &str, kind: &RefreshKind, now: u64) -> Option<String> {
    if let Some(reason) = refresh_failure_gate(provider, now) {
        return Some(reason);
    }
    let states = REFRESH_STATE.lock().unwrap_or_else(|err| err.into_inner());
    let state = states.get(provider)?;
    let floor = match kind {
        RefreshKind::Interactive => INTERACTIVE_REFRESH_FLOOR_SECONDS,
        RefreshKind::Background => BACKGROUND_REFRESH_FLOOR_SECONDS,
    };
    if let Some(last_success) = state.last_success_unix
        && now.saturating_sub(last_success) < floor
    {
        return Some("fresh_enough".to_string());
    }
    None
}

pub(crate) fn current_refresh_generation(provider: &str) -> u64 {
    let states = REFRESH_STATE.lock().unwrap_or_else(|err| err.into_inner());
    states
        .get(provider)
        .map(|state| state.generation)
        .unwrap_or(0)
}

pub(crate) enum RefreshAdmission {
    Accepted(u64),
    Skipped { generation: u64, reason: String },
}

pub(crate) fn begin_refresh_request(
    provider: &str,
    requested_generation: Option<u64>,
) -> RefreshAdmission {
    let mut states = REFRESH_STATE.lock().unwrap_or_else(|err| err.into_inner());
    let state = states.entry(provider.to_string()).or_default();
    if let Some(requested_generation) = requested_generation
        && requested_generation < state.generation
    {
        return RefreshAdmission::Skipped {
            generation: state.generation,
            reason: "stale_request".to_string(),
        };
    }
    if state.in_flight {
        return RefreshAdmission::Skipped {
            generation: state.generation,
            reason: "refresh_in_flight".to_string(),
        };
    }

    state.generation = requested_generation.unwrap_or(state.generation.saturating_add(1));
    state.in_flight = true;
    RefreshAdmission::Accepted(state.generation)
}

pub(crate) fn record_refresh_result(provider: &str, generation: u64, now: u64, success: bool) {
    let mut states = REFRESH_STATE.lock().unwrap_or_else(|err| err.into_inner());
    let state = states.entry(provider.to_string()).or_default();
    if generation != state.generation {
        return;
    }
    state.in_flight = false;
    state.last_attempt_unix = Some(now);
    if success {
        state.last_success_unix = Some(now);
        state.last_error_unix = None;
    } else {
        state.last_error_unix = Some(now);
    }
}

pub(crate) fn release_refresh_request(provider: &str, generation: u64) {
    let mut states = REFRESH_STATE.lock().unwrap_or_else(|err| err.into_inner());
    let state = states.entry(provider.to_string()).or_default();
    if generation == state.generation {
        state.in_flight = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_view_exposes_backoff_and_failure_gate() {
        reset_refresh_state_for_tests();
        let generation = match begin_refresh_request("codex", None) {
            RefreshAdmission::Accepted(generation) => generation,
            RefreshAdmission::Skipped { .. } => panic!("refresh should start"),
        };
        record_refresh_timeout("codex", generation, 100);

        let view = provider_runtime_view("codex", 120);

        assert_eq!(
            refresh_failure_gate("codex", 120).as_deref(),
            Some("error_backoff")
        );
        assert!(!view.refresh_eligible);
        assert!(view.lifecycle.contains(&ProviderRuntimeLifecycle::Backoff));
        assert!(view.backoff_until_unix.is_some());
    }
}
