use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cw_storage_plus::Item;
use cw20::{Balance};
use crate::error::*;

// Config with contract admin
pub const CONFIG: Item<Config> = Item::new("config");

#[cw_serde]
pub struct Config {
    pub admin: Addr,
    pub cw20_wl: Vec<(String, Addr)>,
}

pub fn is_balance_whitelisted(
    balance: &Balance,
    config: &Config,
) -> Result<(), ContractError> {

    // config.cw20_wl has (Token symbol, Token contract address)
    // ex: (NETA, juno168ctmpyppk90d34p3jjy658zf5a5l3w8wk35wht6ccqj4mr0yv8s4j5awr)

    let cw20_whitelist_addrs: Vec<Addr> = config.cw20_wl
    .iter()
    .map(|cw20_token| cw20_token.1.clone())
    .collect();

    if let Balance::Cw20(cw20) = balance.clone() {
        if cw20_whitelist_addrs.contains(&cw20.address) {
            return Ok(());
        }
    }

    Err(ContractError::NotWhitelisted {})
}


