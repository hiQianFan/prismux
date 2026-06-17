use crate::{
    AccountRef, AccountStatus, ConfigProfile, DoctorReport, ImportConfigOptions, ImportedConfig,
    LoginOptions, PlatformCapabilities, PlatformInstall, PlatformPoolSummary, Result, SaveOptions,
    SwitchReport, UseReport,
};

pub trait PlatformPlugin {
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;

    fn capabilities(&self) -> PlatformCapabilities {
        PlatformCapabilities::account_pool()
    }

    fn detect(&self) -> Result<PlatformInstall>;
    fn pool_summary(&self) -> Result<PlatformPoolSummary>;
    fn current(&self) -> Result<Option<AccountStatus>>;
    fn list_accounts(&self) -> Result<Vec<AccountStatus>>;
    fn list_configs(&self) -> Result<Vec<ConfigProfile>> {
        Ok(Vec::new())
    }
    fn login(&self, options: LoginOptions) -> Result<AccountRef>;
    fn save_current(&self, options: SaveOptions) -> Result<AccountRef>;
    fn import_config(&self, options: ImportConfigOptions) -> Result<ImportedConfig>;
    fn use_target(&self, selector: &str) -> Result<UseReport> {
        self.switch_to(selector).map(UseReport::Account)
    }
    fn switch_to(&self, selector: &str) -> Result<SwitchReport>;
    fn set_alias(&self, selector: &str, alias: &str) -> Result<AccountRef>;
    fn doctor(&self) -> Result<DoctorReport>;
}
