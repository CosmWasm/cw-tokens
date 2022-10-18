#[cfg(not(feature = "library"))]
use cosmwasm_std::{
    entry_point, from_binary, to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
};
use cw2::set_contract_version;

use crate::error::ContractError;
use crate::msg::*;
use crate::state::*;

use cw20::{Balance, Cw20CoinVerified, Cw20ReceiveMsg};

const CONTRACT_NAME: &str = "crates.io:cw20_receiver_minimal";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");


// Instantiate entry point
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let admin = info.sender.to_string();

    let cw20_whitelist: Vec<(String, Addr)> = validate_whitelist(deps.as_ref(), msg.whitelist)?;

    CONFIG.save(
        deps.storage,
        &Config {
            admin: deps.api.addr_validate(&admin)?,
            cw20_wl: cw20_whitelist,
        },
    )?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("admin", admin))
}

// Execute entry point
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        // cw20 receive wrapper
        ExecuteMsg::Receive(receive_msg) => execute_receive(deps, info, receive_msg),
    }
}

// "Filter" for cw20 tokens
pub fn execute_receive(
    deps: DepsMut,
    info: MessageInfo,
    wrapper: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    // Message included in Send{contract, amount, **msg**} execute on the cw20 contract
    let msg: ReceiveMsg = from_binary(&wrapper.msg)?;

    // Wallet that executed the "Send" on the cw20 contract
    let user_wallet = deps.api.addr_validate(&wrapper.sender)?;

    // Constructing cw20 balance
    let balance = Balance::Cw20(Cw20CoinVerified {
        // cw20 contract this message was sent from
        address: info.sender.clone(),
        // Send{contract, **amount**, msg}
        amount: wrapper.amount,
    });

    // Load config for whitelist check
    let config = CONFIG.load(deps.storage)?;

    // Check constructed cw20 balance <line 70>, returns contract error if not
    is_balance_whitelisted(&balance, &config)?;

    match msg {
        // Message included in the "Send{contract, amount, **msg**}" call on the cw20 contract,
        ReceiveMsg::AnExecuteMsg {} => {
            execute_business_logic(deps, &user_wallet, &info.sender, balance)
        }
    }
}

pub fn execute_business_logic(
    _deps: DepsMut,
    _user_wallet: &Addr,
    _cw20_contract_addr: &Addr,
    _balance: Balance,
) -> Result<Response, ContractError> {

    // The cw20 received is now accessible under "balance",
    // and you could implement some business logic that uses it here

    Ok(Response::default())
}

// Query entry point
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetAdmin {} => to_binary(&get_admin(deps)?),
    }
}

pub fn get_admin(deps: Deps) -> StdResult<Binary> {
    let config = CONFIG.load(deps.storage)?;
    let admin = config.admin.to_string();
    to_binary(&AdminResponse { admin })
}
