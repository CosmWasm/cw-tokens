use crate::error::ContractError;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Deps};
use cw20::Cw20ReceiveMsg;

#[cw_serde]
pub struct InstantiateMsg {
    pub whitelist: Vec<(String, String)>,
}

pub fn validate_whitelist(
    deps: Deps,
    whitelist: Vec<(String, String)>,
) -> Result<Vec<(String, Addr)>, ContractError> {
    let validated_whitelist: Result<Vec<_>, ContractError> = whitelist
        .iter()
        .map(|unchecked| {
            let checked = deps.api.addr_validate(&unchecked.1)?;
            Ok((unchecked.0.clone(), checked))
        })
        .collect();

    validated_whitelist
}

#[cw_serde]
pub enum ExecuteMsg {
    // Receive Filter
    Receive(Cw20ReceiveMsg),
}

#[cw_serde]
pub enum ReceiveMsg {
    AnExecuteMsg {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(AdminResponse)]
    GetAdmin {},
}

#[cw_serde]
pub struct AdminResponse {
    pub admin: String,
}
