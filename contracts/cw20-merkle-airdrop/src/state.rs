use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, Uint128};
use cw_storage_plus::{Item, Map};
use cw_utils::{Expiration, Scheduled};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    /// Owner If None set, contract is frozen.
    pub owner: Option<Addr>,
    pub cw20_token_address: Option<Addr>,
    pub native_token: Option<String>,
}

pub const CONFIG_KEY: &str = "config";
pub const CONFIG: Item<Config> = Item::new(CONFIG_KEY);

pub const LATEST_STAGE_KEY: &str = "stage";
pub const LATEST_STAGE: Item<u8> = Item::new(LATEST_STAGE_KEY);

pub const STAGE_EXPIRATION_KEY: &str = "stage_exp";
pub const STAGE_EXPIRATION: Map<u8, Expiration> = Map::new(STAGE_EXPIRATION_KEY);

pub const STAGE_START_KEY: &str = "stage_start";
pub const STAGE_START: Map<u8, Scheduled> = Map::new(STAGE_START_KEY);

pub const STAGE_AMOUNT_KEY: &str = "stage_amount";
pub const STAGE_AMOUNT: Map<u8, Uint128> = Map::new(STAGE_AMOUNT_KEY);

pub const STAGE_AMOUNT_CLAIMED_KEY: &str = "stage_claimed_amount";
pub const STAGE_AMOUNT_CLAIMED: Map<u8, Uint128> = Map::new(STAGE_AMOUNT_CLAIMED_KEY);

// saves external network airdrop accounts
pub const STAGE_ACCOUNT_MAP_KEY: &str = "stage_account_map";
// (stage, external_address) -> host_address
pub const STAGE_ACCOUNT_MAP: Map<(u8, String), String> = Map::new(STAGE_ACCOUNT_MAP_KEY);

pub const MERKLE_ROOT_PREFIX: &str = "merkle_root";
pub const MERKLE_ROOT: Map<u8, String> = Map::new(MERKLE_ROOT_PREFIX);

pub const CLAIM_PREFIX: &str = "claim";
pub const CLAIM: Map<(String, u8), bool> = Map::new(CLAIM_PREFIX);

pub const CLAIMED_AMOUNT_PREFIX: &str = "claimed_amount";
pub const CLAIMED_AMOUNT: Map<(&Addr, u8), bool> = Map::new(CLAIMED_AMOUNT_PREFIX);

pub const HRP_PREFIX: &str = "hrp";
pub const HRP: Map<u8, String> = Map::new(HRP_PREFIX);
