use prismux_core::{
    PrismuxError, Result,
    storage::{read_file, state_root, write_file_atomic_private},
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const SETTINGS_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SettingsView {
    pub schema_version: u32,
    pub general: GeneralSettings,
    pub providers: Vec<ProviderSettings>,
    pub privacy: PrivacySettings,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GeneralSettings {
    pub refresh_cadence_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PrivacySettings {
    pub hide_personal_identifiers: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderSettings {
    pub provider: String,
    pub display_label: String,
    pub enabled: bool,
    pub status: ProviderSettingsStatus,
    pub diagnostics: Vec<SettingsDiagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderSettingsStatus {
    pub status: String,
    pub status_text: String,
    pub status_tone: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SettingsDiagnostic {
    pub code: String,
    pub message: String,
    pub recovery_action: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpdateSettingsCommand {
    pub view: SettingsView,
}

pub fn default_settings_view() -> SettingsView {
    SettingsView {
        schema_version: SETTINGS_SCHEMA_VERSION,
        general: GeneralSettings {
            refresh_cadence_seconds: 300,
        },
        providers: [
            ("codex", "Codex", true, "ready", "Ready", "success"),
            (
                "claude",
                "Claude",
                true,
                "planned",
                "Planned provider",
                "secondary",
            ),
        ]
        .into_iter()
        .map(
            |(provider, display_label, enabled, status, status_text, status_tone)| {
                ProviderSettings {
                    provider: provider.to_string(),
                    display_label: display_label.to_string(),
                    enabled,
                    status: ProviderSettingsStatus {
                        status: status.to_string(),
                        status_text: status_text.to_string(),
                        status_tone: status_tone.to_string(),
                    },
                    diagnostics: Vec::new(),
                }
            },
        )
        .collect(),
        privacy: PrivacySettings {
            hide_personal_identifiers: false,
        },
    }
}

pub fn settings_view() -> Result<SettingsView> {
    let path = settings_path()?;
    let bytes = match read_file(&path) {
        Ok(bytes) => bytes,
        Err(_) => return Ok(default_settings_view()),
    };
    let view: SettingsView = serde_json::from_slice(&bytes).map_err(|err| {
        PrismuxError::Message(format!(
            "{} contains invalid settings JSON: {err}",
            prismux_core::storage::display_path(&path)
        ))
    })?;
    validate_settings(&view)?;
    Ok(normalize_settings(view))
}

pub fn update_settings(command: UpdateSettingsCommand) -> Result<SettingsView> {
    validate_settings(&command.view)?;
    let view = normalize_settings(command.view);
    let bytes = serde_json::to_vec_pretty(&view)
        .map_err(|err| PrismuxError::Message(format!("failed to encode settings: {err}")))?;
    write_file_atomic_private(&settings_path()?, &bytes)?;
    Ok(view)
}

pub fn settings_storage_path() -> Result<PathBuf> {
    settings_path()
}

fn settings_path() -> Result<PathBuf> {
    Ok(state_root()?.join("control-plane").join("settings.json"))
}

fn normalize_settings(mut view: SettingsView) -> SettingsView {
    let defaults = default_settings_view();
    for provider in &mut view.providers {
        if provider.display_label.trim().is_empty() {
            provider.display_label = provider.provider.clone();
        }
        if provider.status.status.trim().is_empty()
            && let Some(default) = defaults
                .providers
                .iter()
                .find(|default| default.provider == provider.provider)
        {
            provider.status = default.status.clone();
        }
    }
    view
}

fn validate_settings(view: &SettingsView) -> Result<()> {
    if view.schema_version > SETTINGS_SCHEMA_VERSION {
        return Err(PrismuxError::Message(format!(
            "unsupported settings schema version {}",
            view.schema_version
        )));
    }
    if view.general.refresh_cadence_seconds < 30 {
        return Err(PrismuxError::Message(
            "refresh cadence must be at least 30 seconds".to_string(),
        ));
    }
    if view.providers.is_empty() {
        return Err(PrismuxError::Message(
            "settings must include at least one provider".to_string(),
        ));
    }
    let mut seen = std::collections::BTreeSet::new();
    for provider in &view.providers {
        if provider.provider.trim().is_empty() {
            return Err(PrismuxError::Message(
                "settings provider id must not be empty".to_string(),
            ));
        }
        if !seen.insert(provider.provider.as_str()) {
            return Err(PrismuxError::Message(format!(
                "duplicate settings provider `{}`",
                provider.provider
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use prismux_core::storage::unix_now_nanos;
    use std::sync::{Mutex, OnceLock};

    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
    }

    struct EnvGuard {
        previous: Option<std::ffi::OsString>,
    }

    impl EnvGuard {
        fn set_state_root(path: &std::path::Path) -> Self {
            let previous = std::env::var_os("PRISMUX_STATE_ROOT");
            unsafe {
                std::env::set_var("PRISMUX_STATE_ROOT", path);
            }
            Self { previous }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            unsafe {
                match &self.previous {
                    Some(value) => std::env::set_var("PRISMUX_STATE_ROOT", value),
                    None => std::env::remove_var("PRISMUX_STATE_ROOT"),
                }
            }
        }
    }

    #[test]
    fn settings_are_typed_and_default_to_safe_values() {
        let root = std::env::temp_dir().join(format!(
            "prismux-settings-default-test-{}-{}",
            std::process::id(),
            unix_now_nanos()
        ));
        let _lock = env_lock();
        let _guard = EnvGuard::set_state_root(&root);

        let view = settings_view().unwrap();

        assert_eq!(view.schema_version, SETTINGS_SCHEMA_VERSION);
        assert_eq!(view.general.refresh_cadence_seconds, 300);
        assert!(view.providers.iter().all(|provider| provider.enabled));
        assert!(!view.privacy.hide_personal_identifiers);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn update_settings_persists_view() {
        let root = std::env::temp_dir().join(format!(
            "prismux-settings-persist-test-{}-{}",
            std::process::id(),
            unix_now_nanos()
        ));
        let _lock = env_lock();
        let _guard = EnvGuard::set_state_root(&root);
        let mut view = default_settings_view();
        view.general.refresh_cadence_seconds = 900;
        view.privacy.hide_personal_identifiers = true;

        update_settings(UpdateSettingsCommand { view }).unwrap();
        let loaded = settings_view().unwrap();

        assert_eq!(loaded.general.refresh_cadence_seconds, 900);
        assert!(loaded.privacy.hide_personal_identifiers);
        assert!(settings_storage_path().unwrap().exists());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn future_settings_schema_fails_closed() {
        let mut view = default_settings_view();
        view.schema_version = SETTINGS_SCHEMA_VERSION + 1;

        let err = update_settings(UpdateSettingsCommand { view }).unwrap_err();

        assert!(err.to_string().contains("unsupported settings schema"));
    }
}
