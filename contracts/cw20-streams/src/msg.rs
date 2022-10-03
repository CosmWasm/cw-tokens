// use crate::state::Stream;
use cosmwasm_std::Uint128;
use cw20::Cw20ReceiveMsg;
use cosmwasm_schema::{cw_serde, QueryResponses};

#[cw_serde]
pub struct InstantiateMsg {
    pub owner: Option<String>,
    pub cw20_addr: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg),
    Withdraw {
        id: u64, // Stream id
    },
}

#[cw_serde]
pub enum ReceiveMsg {
    CreateStream {
        recipient: String,
        start_time: u64,
        end_time: u64,
    },
}

#[cw_serde]
pub struct StreamParams {
    pub owner: String,
    pub recipient: String,
    pub amount: Uint128,
    pub start_time: u64,
    pub end_time: u64,
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ConfigResponse)]
    GetConfig {},
    #[returns(StreamResponse)]
    GetStream {
        id: u64,
    },
    #[returns(ListStreamsResponse)]
    ListStreams {
        start: Option<u8>,
        limit: Option<u8>,
    },
}

#[cw_serde]
pub struct ConfigResponse {
    pub owner: String,
    pub cw20_addr: String,
}

#[cw_serde]
pub struct StreamResponse {
    pub id: u64,
    pub owner: String,
    pub recipient: String,
    pub amount: Uint128,
    pub claimed_amount: Uint128,
    pub start_time: u64,
    pub end_time: u64,
    pub rate_per_second: Uint128,
}

#[cw_serde]
pub struct ListStreamsResponse {
    pub streams: Vec<StreamResponse>,
}
