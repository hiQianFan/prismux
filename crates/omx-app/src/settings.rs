use omx_core::{OpenMuxError, Result};
use serde::{Deserialize, Serialize};

pub const SETTINGS_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SettingsView {
    pub schema_version: u32,
    pub provider_order: Vec<String>,
    pub providers: Vec<ProviderSettings>,
    pub refresh_cadence_seconds: u64,
    pub display_preference: DisplayPreference,
    pub debug_recovery: DebugRecoverySettings,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderSettings {
    pub provider: String,
    pub enabled: bool,
    pub source_preference: SourcePreference,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourcePreference {
    Auto,
    LocalOnly,
    RemoteOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DisplayPreference {
    Compact,
    Detailed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DebugRecoverySettings {
    pub include_debug_summary_in_support_report: bool,
    pub allow_destructive_recovery_actions: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpdateSettingsCommand {
    pub view: SettingsView,
}

pub fn default_settings_view() -> SettingsView {
    let providers = ["codex", "claude"]
        .into_iter()
        .map(|provider| ProviderSettings {
            provider: provider.to_string(),
            enabled: true,
            source_preference: SourcePreference::Auto,
        })
        .collect::<Vec<_>>();
    SettingsView {
        schema_version: SETTINGS_SCHEMA_VERSION,
        provider_order: providers
            .iter()
            .map(|provider| provider.provider.clone())
            .collect(),
        providers,
        refresh_cadence_seconds: 300,
        display_preference: DisplayPreference::Compact,
        debug_recovery: DebugRecoverySettings {
            include_debug_summary_in_support_report: false,
            allow_destructive_recovery_actions: false,
        },
    }
}

pub fn settings_view() -> Result<SettingsView> {
    Ok(default_settings_view())
}

pub fn update_settings(command: UpdateSettingsCommand) -> Result<SettingsView> {
    validate_settings(&command.view)?;
    Ok(command.view)
}

fn validate_settings(view: &SettingsView) -> Result<()> {
    if view.schema_version > SETTINGS_SCHEMA_VERSION {
        return Err(OpenMuxError::Message(format!(
            "unsupported settings schema version {}",
            view.schema_version
        )));
    }
    if view.refresh_cadence_seconds < 30 {
        return Err(OpenMuxError::Message(
            "refresh cadence must be at least 30 seconds".to_string(),
        ));
    }
    for provider in &view.provider_order {
        if !view
            .providers
            .iter()
            .any(|settings| settings.provider == *provider)
        {
            return Err(OpenMuxError::Message(format!(
                "provider order references unknown provider `{provider}`"
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_are_typed_and_default_to_safe_values() {
        let view = settings_view().unwrap();

        assert_eq!(view.schema_version, SETTINGS_SCHEMA_VERSION);
        assert_eq!(view.display_preference, DisplayPreference::Compact);
        assert!(view.providers.iter().all(|provider| provider.enabled));
        assert!(!view.debug_recovery.allow_destructive_recovery_actions);
    }

    #[test]
    fn future_settings_schema_fails_closed() {
        let mut view = default_settings_view();
        view.schema_version = SETTINGS_SCHEMA_VERSION + 1;

        let err = update_settings(UpdateSettingsCommand { view }).unwrap_err();

        assert!(err.to_string().contains("unsupported settings schema"));
    }

    #[test]
    fn settings_reject_invalid_provider_order() {
        let mut view = default_settings_view();
        view.provider_order.push("missing".to_string());

        let err = update_settings(UpdateSettingsCommand { view }).unwrap_err();

        assert!(err.to_string().contains("unknown provider"));
    }
}
