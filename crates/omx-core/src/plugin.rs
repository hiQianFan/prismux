use crate::{
    AccountRef, AccountStatus, ConfigProfile, DoctorReport, ImportConfigOptions, ImportedConfig,
    LoginOptions, OpenMuxError, PlatformCapabilities, PlatformInstall, PlatformPoolSummary,
    RemoveReport, Result, SaveOptions, SwitchReport, UseReport,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResetCreditOutcome {
    Reset { windows_reset: u32 },
    NothingToReset,
    NoCredit,
    AlreadyRedeemed,
}

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
    fn refresh_accounts(&self) -> Result<Vec<AccountStatus>> {
        let accounts = self.list_accounts()?;
        accounts
            .iter()
            .map(|status| self.refresh_account(&status.account.local_id))
            .collect()
    }
    fn refresh_account(&self, selector: &str) -> Result<AccountStatus> {
        self.list_accounts()?
            .into_iter()
            .find(|status| status.account.local_id == selector)
            .ok_or_else(|| OpenMuxError::AccountNotFound {
                platform: self.id().to_string(),
                account: selector.to_string(),
            })
    }
    fn list_configs(&self) -> Result<Vec<ConfigProfile>> {
        Ok(Vec::new())
    }
    fn login(&self, options: LoginOptions) -> Result<AccountRef>;
    fn save_current(&self, options: SaveOptions) -> Result<AccountRef>;
    fn import_config(&self, options: ImportConfigOptions) -> Result<ImportedConfig>;
    fn use_target(&self, selector: &str) -> Result<UseReport> {
        self.switch_to(selector).map(UseReport::Account)
    }
    fn remove_target(&self, _selector: &str) -> Result<RemoveReport> {
        Err(OpenMuxError::Message(format!(
            "{} does not support removing managed targets yet",
            self.name()
        )))
    }
    fn consume_reset_credit(
        &self,
        _selector: &str,
        _idempotency_key: &str,
    ) -> Result<ResetCreditOutcome> {
        Err(OpenMuxError::Message(format!(
            "{} does not support reset credits",
            self.name()
        )))
    }
    fn switch_to(&self, selector: &str) -> Result<SwitchReport>;
    fn set_alias(&self, selector: &str, alias: &str) -> Result<AccountRef>;
    fn doctor(&self) -> Result<DoctorReport>;
}
