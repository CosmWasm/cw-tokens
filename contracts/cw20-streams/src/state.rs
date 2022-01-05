use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{Addr, DepsMut, StdResult, Uint128};
use cw_storage_plus::{Item, Map};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: Addr,
    pub cw20_addr: Addr,
}

pub const CONFIG: Item<Config> = Item::new("config");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Stream {
    pub owner: Addr,
    pub recipient: Addr,
    pub amount: Uint128,
    pub claimed_amount: Uint128,
    pub start_time: u64,
    pub end_time: u64,
    pub rate_per_second: Uint128,
}

pub const STREAM_SEQ: Item<u64> = Item::new("stream_seq");
pub const STREAMS: Map<u64, Stream> = Map::new("stream");

pub fn save_stream(deps: DepsMut, stream: &Stream) -> StdResult<u64> {
    let id = STREAM_SEQ.load(deps.storage)?;
    let id = id.checked_add(1).unwrap();
    STREAM_SEQ.save(deps.storage, &id)?;
    STREAMS.save(deps.storage, id, stream)?;
    Ok(id)
}
