use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("InvalidStartTime")]
    InvalidStartTime {},

    #[error("StreamFullyClaimed")]
    StreamFullyClaimed {},

    #[error("StreamNotStarted")]
    StreamNotStarted {},

    #[error("NotStreamRecipient")]
    NotStreamRecipient {},

    #[error("NoFundsToClaim")]
    NoFundsToClaim {},

    #[error("StreamNotFound")]
    StreamNotFound {},

    #[error("InvalidDuration")]
    InvalidDuration {},

    #[error("InvalidOwner")]
    InvalidOwner {},

    #[error("InvalidRecipient")]
    InvalidRecipient {},
}
