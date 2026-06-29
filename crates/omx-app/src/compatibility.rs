use serde::{Deserialize, Serialize};

pub const CONTROL_PLANE_SCHEMA_VERSION: u32 = 3;
pub const STATE_SCHEMA_VERSION: u32 = 1;
pub const MIN_BACKEND_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const MIN_FRONTEND_VERSION: &str = "0.1.0";

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClientDescriptor {
    #[serde(default)]
    pub frontend_name: Option<String>,
    #[serde(default)]
    pub frontend_version: Option<String>,
    #[serde(default)]
    pub control_plane_schema_version: Option<u32>,
    #[serde(default)]
    pub state_schema_version: Option<u32>,
    #[serde(default)]
    pub artifact: Option<DistributionArtifactKind>,
    #[serde(default)]
    pub runtime_binding: Option<BackendRuntimeKind>,
    #[serde(default)]
    pub unavailable_modules: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompatibilityResult {
    pub compatible: bool,
    pub control_plane_schema_version: u32,
    pub state_schema_version: u32,
    pub minimum_backend_version: String,
    pub minimum_frontend_version: String,
    pub supported_artifacts: Vec<ArtifactCapability>,
    pub backend_runtime_options: Vec<BackendRuntimeOption>,
    pub optional_modules: Vec<OptionalModuleStatus>,
    pub provider_capabilities: Vec<ProviderCapabilityMatrix>,
    pub diagnostics: Vec<CompatibilityDiagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompatibilityDiagnostic {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DistributionArtifactKind {
    CliOnly,
    MenubarOnly,
    FullBundle,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArtifactCapability {
    pub artifact: DistributionArtifactKind,
    pub capabilities: Vec<String>,
    pub platform: String,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BackendRuntimeKind {
    EmbeddedStaticlib,
    HelperBinary,
    InstalledCli,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BackendRuntimeOption {
    pub kind: BackendRuntimeKind,
    pub supported: bool,
    pub guidance: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModuleAvailability {
    Available,
    Unavailable,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OptionalModuleStatus {
    pub module: String,
    pub availability: ModuleAvailability,
    pub unavailable_view_title: Option<String>,
    pub install_guidance: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderCapabilityMatrix {
    pub provider: String,
    pub capabilities: Vec<String>,
}

pub fn compatibility_view(client: ClientDescriptor) -> CompatibilityResult {
    let mut diagnostics = Vec::new();
    let mut compatible = true;

    if let Some(schema) = client.control_plane_schema_version
        && schema != CONTROL_PLANE_SCHEMA_VERSION
    {
        compatible = false;
        diagnostics.push(CompatibilityDiagnostic {
            code: "unsupported_control_plane_schema".to_string(),
            message: "Unsupported control-plane schema version.".to_string(),
        });
    }

    if let Some(schema) = client.state_schema_version
        && schema > STATE_SCHEMA_VERSION
    {
        compatible = false;
        diagnostics.push(CompatibilityDiagnostic {
            code: "unsupported_future_state_schema".to_string(),
            message: "Unsupported future state schema version.".to_string(),
        });
    }

    let optional_modules = optional_module_statuses(&client);
    for module in optional_modules
        .iter()
        .filter(|module| module.availability == ModuleAvailability::Unavailable)
    {
        diagnostics.push(CompatibilityDiagnostic {
            code: format!("optional_module_unavailable:{}", module.module),
            message: module.install_guidance.clone().unwrap_or_else(|| {
                "Install the missing optional module to enable this surface.".to_string()
            }),
        });
    }

    CompatibilityResult {
        compatible,
        control_plane_schema_version: CONTROL_PLANE_SCHEMA_VERSION,
        state_schema_version: STATE_SCHEMA_VERSION,
        minimum_backend_version: MIN_BACKEND_VERSION.to_string(),
        minimum_frontend_version: MIN_FRONTEND_VERSION.to_string(),
        supported_artifacts: supported_artifacts(),
        backend_runtime_options: backend_runtime_options(),
        optional_modules,
        provider_capabilities: provider_capabilities(),
        diagnostics,
    }
}

fn supported_artifacts() -> Vec<ArtifactCapability> {
    vec![
        ArtifactCapability {
            artifact: DistributionArtifactKind::CliOnly,
            capabilities: vec![
                "account_save".to_string(),
                "account_switch".to_string(),
                "profile_import".to_string(),
                "doctor".to_string(),
                "usage_summary".to_string(),
            ],
            platform: "macos".to_string(),
            dependencies: vec!["installed_ai_tool_cli".to_string()],
        },
        ArtifactCapability {
            artifact: DistributionArtifactKind::MenubarOnly,
            capabilities: vec![
                "dashboard".to_string(),
                "explicit_activation".to_string(),
                "refresh".to_string(),
                "last_good_snapshot".to_string(),
                "cli_handoff".to_string(),
            ],
            platform: "macos_14_apple_silicon".to_string(),
            dependencies: vec!["embedded_staticlib_or_helper".to_string()],
        },
        ArtifactCapability {
            artifact: DistributionArtifactKind::FullBundle,
            capabilities: vec![
                "cli".to_string(),
                "menubar".to_string(),
                "shared_state_root".to_string(),
                "contract_compatibility_gate".to_string(),
            ],
            platform: "macos".to_string(),
            dependencies: vec!["installed_ai_tool_cli".to_string()],
        },
    ]
}

fn backend_runtime_options() -> Vec<BackendRuntimeOption> {
    vec![
        BackendRuntimeOption {
            kind: BackendRuntimeKind::EmbeddedStaticlib,
            supported: true,
            guidance: "Preferred for Menubar: call omx-menubar-ffi directly and gate by schema before mutations."
                .to_string(),
        },
        BackendRuntimeOption {
            kind: BackendRuntimeKind::HelperBinary,
            supported: true,
            guidance: "Allowed for future packaging: helper must share the same state root and control-plane contract."
                .to_string(),
        },
        BackendRuntimeOption {
            kind: BackendRuntimeKind::InstalledCli,
            supported: true,
            guidance: "Allowed fallback: invoke installed omx only for documented machine-readable commands."
                .to_string(),
        },
    ]
}

fn optional_module_statuses(client: &ClientDescriptor) -> Vec<OptionalModuleStatus> {
    let known_modules = ["cli", "menubar", "helper", "serve"];
    known_modules
        .iter()
        .map(|module| {
            let unavailable = client
                .unavailable_modules
                .iter()
                .any(|unavailable| unavailable == module);
            OptionalModuleStatus {
                module: (*module).to_string(),
                availability: if unavailable {
                    ModuleAvailability::Unavailable
                } else {
                    ModuleAvailability::Available
                },
                unavailable_view_title: unavailable.then(|| format!("{module} module unavailable")),
                install_guidance: unavailable.then(|| install_guidance(module)),
            }
        })
        .collect()
}

fn install_guidance(module: &str) -> String {
    match module {
        "cli" => "Install the omx CLI or use a full bundle before running advanced account management commands.",
        "menubar" => "Install OpenMux.app or use the CLI-only workflow for account management.",
        "helper" => "Install the helper runtime or switch Menubar to the embedded staticlib backend.",
        "serve" => "The serve module is not required; keep using CLI or Menubar control-plane calls.",
        _ => "Install the missing optional module to enable this surface.",
    }
    .to_string()
}

fn provider_capabilities() -> Vec<ProviderCapabilityMatrix> {
    vec![
        ProviderCapabilityMatrix {
            provider: "codex".to_string(),
            capabilities: vec![
                "detected_only".to_string(),
                "account_switchable".to_string(),
                "profile_switchable".to_string(),
                "quota_readable".to_string(),
                "account_scoped_refreshable".to_string(),
                "local_usage_readable".to_string(),
                "menubar_ready".to_string(),
            ],
        },
        ProviderCapabilityMatrix {
            provider: "claude".to_string(),
            capabilities: vec![
                "detected_only".to_string(),
                "account_switchable".to_string(),
                "profile_switchable".to_string(),
                "local_usage_readable".to_string(),
            ],
        },
        ProviderCapabilityMatrix {
            provider: "gemini".to_string(),
            capabilities: vec!["detected_only".to_string()],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn future_state_schema_fails_closed() {
        let result = compatibility_view(ClientDescriptor {
            state_schema_version: Some(STATE_SCHEMA_VERSION + 1),
            ..ClientDescriptor::default()
        });

        assert!(!result.compatible);
        assert_eq!(
            result.diagnostics[0].code,
            "unsupported_future_state_schema"
        );
    }

    #[test]
    fn compatibility_exposes_distribution_and_provider_matrix() {
        let result = compatibility_view(ClientDescriptor::default());

        assert!(result.compatible);
        assert_eq!(result.supported_artifacts.len(), 3);
        assert!(
            result
                .backend_runtime_options
                .iter()
                .any(|option| option.kind == BackendRuntimeKind::EmbeddedStaticlib)
        );
        assert!(result.provider_capabilities.iter().any(|provider| {
            provider.provider == "codex"
                && provider
                    .capabilities
                    .contains(&"account_scoped_refreshable".to_string())
        }));
    }

    #[test]
    fn unavailable_optional_modules_keep_contract_compatible_with_guidance() {
        let result = compatibility_view(ClientDescriptor {
            unavailable_modules: vec!["cli".to_string()],
            ..ClientDescriptor::default()
        });

        assert!(result.compatible);
        let cli = result
            .optional_modules
            .iter()
            .find(|module| module.module == "cli")
            .expect("cli module status");
        assert_eq!(cli.availability, ModuleAvailability::Unavailable);
        assert!(cli.install_guidance.as_deref().unwrap().contains("omx CLI"));
        assert_eq!(
            result.diagnostics[0].code,
            "optional_module_unavailable:cli"
        );
    }
}
