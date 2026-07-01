pub type Result<T> = std::result::Result<T, PrismuxError>;

#[derive(Debug, thiserror::Error)]
pub enum PrismuxError {
    #[error("platform `{0}` is not installed or could not be detected")]
    PlatformNotDetected(String),
    #[error("account `{account}` was not found for platform `{platform}`")]
    AccountNotFound { platform: String, account: String },
    #[error("{0}")]
    Message(String),
}
