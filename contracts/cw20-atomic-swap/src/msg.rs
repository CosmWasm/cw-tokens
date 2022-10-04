use cosmwasm_schema::{cw_serde, QueryResponses};

use cosmwasm_std::Coin;
use cw20::{Cw20Coin, Cw20ReceiveMsg, Expiration};

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
    Create(CreateMsg),
    /// Release sends all tokens to the recipient.
    Release {
        id: String,
        /// This is the preimage, must be exactly 32 bytes in hex (64 chars)
        /// to release: sha256(from_hex(preimage)) == from_hex(hash)
        preimage: String,
    },
    /// Refund returns all remaining tokens to the original sender,
    Refund {
        id: String,
    },
    /// This accepts a properly-encoded ReceiveMsg from a cw20 contract
    Receive(Cw20ReceiveMsg),
}

#[cw_serde]
pub enum ReceiveMsg {
    Create(CreateMsg),
}

#[cw_serde]
pub struct CreateMsg {
    /// id is a human-readable name for the swap to use later.
    /// 3-20 bytes of utf-8 text
    pub id: String,
    /// This is hex-encoded sha-256 hash of the preimage (must be 32*2 = 64 chars)
    pub hash: String,
    /// If approved, funds go to the recipient
    pub recipient: String,
    /// You can set expiration at time or at block height the contract is valid at.
    /// After the contract is expired, it can be returned to the original funder.
    pub expires: Expiration,
}

pub fn is_valid_name(name: &str) -> bool {
    let bytes = name.as_bytes();
    if bytes.len() < 3 || bytes.len() > 20 {
        return false;
    }
    true
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Show all open swaps. Return type is ListResponse.
    #[returns(ListResponse)]
    List {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Returns the details of the named swap, error if not created.
    /// Return type: DetailsResponse.
    #[returns(DetailsResponse)]
    Details { id: String },
}

#[cw_serde]
pub struct ListResponse {
    /// List all open swap ids
    pub swaps: Vec<String>,
}

#[cw_serde]
pub struct DetailsResponse {
    /// Id of this swap
    pub id: String,
    /// This is hex-encoded sha-256 hash of the preimage (must be 32*2 = 64 chars)
    pub hash: String,
    /// If released, funds go to the recipient
    pub recipient: String,
    /// If refunded, funds go to the source
    pub source: String,
    /// Once a swap is expired, it can be returned to the original source (via "refund").
    pub expires: Expiration,
    /// Balance in native tokens or cw20 token, with human-readable address
    pub balance: BalanceHuman,
}

#[cw_serde]
pub enum BalanceHuman {
    Native(Vec<Coin>),
    Cw20(Cw20Coin),
}
