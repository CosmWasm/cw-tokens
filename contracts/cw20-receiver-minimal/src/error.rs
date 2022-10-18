use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Address not whitelisted")]
    NotWhitelisted {},

    #[error("Invalid address format: {address}")]
    InvalidAddressFormat {address: String},

}
