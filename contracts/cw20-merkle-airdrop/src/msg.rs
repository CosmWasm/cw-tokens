use crate::ContractError;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{from_slice, Binary, Uint128};
use cw_utils::{Expiration, Scheduled};
use serde::{Deserialize, Serialize};

#[cw_serde]
pub struct InstantiateMsg {
    /// Owner if none set to info.sender.
    pub owner: Option<String>,
    pub cw20_token_address: Option<String>,
    pub native_token: Option<String>,
}

#[cw_serde]
pub enum ExecuteMsg {
    UpdateConfig {
        /// NewOwner if non sent, contract gets locked. Recipients can receive airdrops
        /// but owner cannot register new stages.
        new_owner: Option<String>,
        new_cw20_address: Option<String>,
        new_native_token: Option<String>,
    },
    RegisterMerkleRoot {
        /// MerkleRoot is hex-encoded merkle root.
        merkle_root: String,
        expiration: Option<Expiration>,
        start: Option<Scheduled>,
        total_amount: Option<Uint128>,
        // hrp is the bech32 parameter required for building external network address
        // from signature message during claim action. example "cosmos", "terra", "juno"
        hrp: Option<String>,
    },
    /// Claim does not check if contract has enough funds, owner must ensure it.
    Claim {
        stage: u8,
        amount: Uint128,
        /// Proof is hex-encoded merkle proof.
        proof: Vec<String>,
        /// Enables cross chain airdrops.
        /// Target wallet proves identity by sending a signed [SignedClaimMsg](SignedClaimMsg)
        /// containing the recipient address.
        sig_info: Option<SignatureInfo>,
    },
    /// Burn the remaining tokens after expire time (only owner)
    Burn {
        stage: u8,
    },
    /// Withdraw the remaining tokens after expire time (only owner)
    Withdraw {
        stage: u8,
        address: String,
        amount: Option<Uint128>,
    },
    Pause {
        stage: u8,
    },
    Resume {
        stage: u8,
        new_expiration: Option<Expiration>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ConfigResponse)]
    Config {},
    #[returns(MerkleRootResponse)]
    MerkleRoot { stage: u8 },
    #[returns(LatestStageResponse)]
    LatestStage {},
    #[returns(IsClaimedResponse)]
    IsClaimed { stage: u8, address: String },
    #[returns(TotalClaimedResponse)]
    TotalClaimed { stage: u8 },
    // for cross chain airdrops, maps target account to host account
    #[returns(AccountMapResponse)]
    AccountMap { stage: u8, external_address: String },
    #[returns(AllAccountMapResponse)]
    AllAccountMaps {
        stage: u8,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    #[returns(IsPausedResponse)]
    IsPaused { stage: u8 },
}

#[cw_serde]
pub struct ConfigResponse {
    pub owner: Option<String>,
    pub cw20_token_address: Option<String>,
    pub native_token: Option<String>,
}

#[cw_serde]
pub struct MerkleRootResponse {
    pub stage: u8,
    /// MerkleRoot is hex-encoded merkle root.
    pub merkle_root: String,
    pub expiration: Expiration,
    pub start: Option<Scheduled>,
    pub total_amount: Uint128,
}

#[cw_serde]
pub struct LatestStageResponse {
    pub latest_stage: u8,
}

#[cw_serde]
pub struct IsClaimedResponse {
    pub is_claimed: bool,
}

#[cw_serde]
pub struct IsPausedResponse {
    pub is_paused: bool,
}

#[cw_serde]
pub struct TotalClaimedResponse {
    pub total_claimed: Uint128,
}

#[cw_serde]
pub struct AccountMapResponse {
    pub host_address: String,
    pub external_address: String,
}

#[cw_serde]
pub struct AllAccountMapResponse {
    pub address_maps: Vec<AccountMapResponse>,
}

#[cw_serde]
pub struct MigrateMsg {}

// Signature verification is done on external airdrop claims.
#[cw_serde]
pub struct SignatureInfo {
    pub claim_msg: Binary,
    pub signature: Binary,
}
impl SignatureInfo {
    pub fn extract_addr(&self) -> Result<String, ContractError> {
        let claim_msg = from_slice::<ClaimMsg>(&self.claim_msg)?;
        Ok(claim_msg.address)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ClaimMsg {
    // To provide claiming via ledger, the address is passed in the memo field of a cosmos msg.
    #[serde(rename = "memo")]
    address: String,
}
