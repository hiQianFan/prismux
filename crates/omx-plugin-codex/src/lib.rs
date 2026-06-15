use omx_core::{
    AccountRef, AccountStatus, DoctorCheck, DoctorReport, OpenMuxError, PlatformInfo,
    PlatformInstall, PlatformPlugin, Result, SwitchReport, platform_info,
};

#[derive(Debug, Default)]
pub struct CodexPlugin;

impl CodexPlugin {
    pub fn new() -> Self {
        Self
    }

    fn info(&self) -> PlatformInfo {
        platform_info(self.id(), self.name())
    }
}

impl PlatformPlugin for CodexPlugin {
    fn id(&self) -> &'static str {
        "codex"
    }

    fn name(&self) -> &'static str {
        "Codex"
    }

    fn detect(&self) -> Result<PlatformInstall> {
        Ok(PlatformInstall {
            platform: self.info(),
            config_path: None,
            auth_path: None,
        })
    }

    fn current(&self) -> Result<Option<AccountStatus>> {
        Ok(None)
    }

    fn list_accounts(&self) -> Result<Vec<AccountStatus>> {
        Ok(Vec::new())
    }

    fn import_current(&self, alias: &str) -> Result<AccountRef> {
        Err(OpenMuxError::Message(format!(
            "import for `{alias}` is not implemented yet"
        )))
    }

    fn switch_to(&self, alias: &str) -> Result<SwitchReport> {
        Err(OpenMuxError::Message(format!(
            "switch to `{alias}` is not implemented yet"
        )))
    }

    fn doctor(&self) -> Result<DoctorReport> {
        Ok(DoctorReport {
            platform: self.id().to_string(),
            checks: vec![DoctorCheck {
                name: "plugin".to_string(),
                ok: true,
                message: "Codex plugin is registered".to_string(),
            }],
        })
    }
}
