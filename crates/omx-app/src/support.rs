use crate::{
    compatibility::{CONTROL_PLANE_SCHEMA_VERSION, STATE_SCHEMA_VERSION},
    diagnostics::redaction,
    settings::{SETTINGS_SCHEMA_VERSION, settings_view},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SupportReportCommand {
    #[serde(default)]
    pub include_debug_summary: bool,
    #[serde(default)]
    pub recent_diagnostics: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SupportReport {
    pub schema_version: u32,
    pub app_version: String,
    pub control_plane_schema_version: u32,
    pub state_schema_version: u32,
    pub settings_schema_version: u32,
    pub redaction_status: String,
    pub diagnostics: Vec<SupportDiagnostic>,
    pub debug_summary: Option<SupportDebugSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SupportDiagnostic {
    pub code: String,
    pub severity: DiagnosticSeverity,
    pub user_message: String,
    pub recovery_action: Option<String>,
    pub source: String,
    pub redaction_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SupportDebugSummary {
    pub refresh_cadence_seconds: u64,
    pub enabled_provider_count: usize,
    pub privacy_hide_personal_identifiers: bool,
    pub provider_source_preferences: Vec<ProviderSourcePreferenceSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderSourcePreferenceSummary {
    pub provider: String,
    pub source_preference: String,
}

pub fn support_report(command: SupportReportCommand) -> SupportReport {
    let settings = settings_view().ok();
    let diagnostics = command
        .recent_diagnostics
        .iter()
        .map(|message| SupportDiagnostic {
            code: "recent_diagnostic".to_string(),
            severity: DiagnosticSeverity::Warning,
            user_message: redaction::redact(message),
            recovery_action: Some("Run `omx doctor` for provider-specific checks.".to_string()),
            source: "control_plane".to_string(),
            redaction_status: "redacted".to_string(),
        })
        .collect();
    let debug_summary = command.include_debug_summary.then(|| {
        let settings = settings.unwrap_or_else(crate::settings::default_settings_view);
        SupportDebugSummary {
            refresh_cadence_seconds: settings.general.refresh_cadence_seconds,
            enabled_provider_count: settings
                .providers
                .iter()
                .filter(|provider| provider.enabled)
                .count(),
            privacy_hide_personal_identifiers: settings.privacy.hide_personal_identifiers,
            provider_source_preferences: settings
                .providers
                .iter()
                .map(|provider| ProviderSourcePreferenceSummary {
                    provider: provider.provider.clone(),
                    source_preference: match provider.source_preference {
                        crate::settings::SourcePreference::Auto => "auto",
                        crate::settings::SourcePreference::LocalOnly => "local_only",
                        crate::settings::SourcePreference::RemoteOnly => "remote_only",
                    }
                    .to_string(),
                })
                .collect(),
        }
    });
    SupportReport {
        schema_version: 1,
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        control_plane_schema_version: CONTROL_PLANE_SCHEMA_VERSION,
        state_schema_version: STATE_SCHEMA_VERSION,
        settings_schema_version: SETTINGS_SCHEMA_VERSION,
        redaction_status: "redacted".to_string(),
        diagnostics,
        debug_summary,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn support_report_redacts_recent_diagnostics() {
        let report = support_report(SupportReportCommand {
            include_debug_summary: true,
            recent_diagnostics: vec!["Authorization: Bearer secret-token".to_string()],
        });

        assert_eq!(report.redaction_status, "redacted");
        assert!(report.debug_summary.is_some());
        assert_eq!(
            report.diagnostics[0].user_message,
            "[redacted sensitive diagnostic]"
        );
    }
}
