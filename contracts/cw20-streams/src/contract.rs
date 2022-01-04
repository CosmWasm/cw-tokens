use crate::error::ContractError;
use crate::msg::{
    ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg, ReceiveMsg, StreamParams, StreamResponse,
};
use crate::state::{save_stream, Config, Stream, CONFIG, STREAMS, STREAM_SEQ};
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_binary, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128,
};
use cw2::set_contract_version;
use cw20::{Cw20Contract, Cw20ExecuteMsg, Cw20ReceiveMsg};

const CONTRACT_NAME: &str = "crates.io:cw20-streams";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let owner = match msg.owner {
        Some(own) => deps.api.addr_validate(&own)?,
        None => info.sender,
    };

    let config = Config {
        owner: owner.clone(),
        cw20_addr: deps.api.addr_validate(&msg.cw20_addr)?,
    };
    CONFIG.save(deps.storage, &config)?;

    STREAM_SEQ.save(deps.storage, &0u64)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", owner)
        .add_attribute("cw20_addr", msg.cw20_addr))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Receive(msg) => execute_receive(env, deps, info, msg),
        ExecuteMsg::Withdraw { id } => execute_withdraw(env, deps, info, id),
    }
}

pub fn execute_create_stream(
    env: Env,
    deps: DepsMut,
    config: Config,
    params: StreamParams,
) -> Result<Response, ContractError> {
    let StreamParams {
        owner,
        recipient,
        amount,
        start_time,
        end_time,
    } = params;
    let owner = deps.api.addr_validate(&owner)?;
    let recipient = deps.api.addr_validate(&recipient)?;

    if config.owner == recipient {
        return Err(ContractError::InvalidRecipient {});
    }

    if start_time > end_time {
        return Err(ContractError::InvalidStartTime {});
    }

    let block_time = env.block.time.seconds();
    if start_time < block_time {
        return Err(ContractError::InvalidStartTime {});
    }

    let duration: Uint128 = (end_time - start_time).into();

    if amount < duration {
        return Err(ContractError::AmountLessThanDuration {});
    }

    // Duration must divide evenly into amount, so refund remainder
    let refund: u128 = amount
        .u128()
        .checked_rem(duration.u128())
        .ok_or(ContractError::Overflow {})?;

    let amount = amount - Uint128::new(refund);

    let rate_per_second = amount / duration;

    let stream = Stream {
        owner: owner.clone(),
        recipient: recipient.clone(),
        amount,
        claimed_amount: Uint128::zero(),
        start_time,
        end_time,
        rate_per_second,
    };
    let id = save_stream(deps, &stream)?;

    let mut response = Response::new()
        .add_attribute("method", "create_stream")
        .add_attribute("stream_id", id.to_string())
        .add_attribute("owner", owner.to_string())
        .add_attribute("recipient", recipient)
        .add_attribute("amount", amount.to_string())
        .add_attribute("start_time", start_time.to_string())
        .add_attribute("end_time", end_time.to_string());

    if refund > 0 {
        let cw20 = Cw20Contract(config.cw20_addr);
        let msg = cw20.call(Cw20ExecuteMsg::Transfer {
            recipient: owner.into(),
            amount: refund.into(),
        })?;

        response = response.add_message(msg);
    }
    Ok(response)
}

pub fn execute_receive(
    env: Env,
    deps: DepsMut,
    info: MessageInfo,
    wrapped: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    if config.cw20_addr != info.sender {
        return Err(ContractError::Unauthorized {});
    }

    let msg: ReceiveMsg = from_binary(&wrapped.msg)?;
    match msg {
        ReceiveMsg::CreateStream {
            start_time,
            end_time,
            recipient,
        } => execute_create_stream(
            env,
            deps,
            config,
            StreamParams {
                owner: wrapped.sender,
                recipient,
                amount: wrapped.amount,
                start_time,
                end_time,
            },
        ),
    }
}

pub fn execute_withdraw(
    env: Env,
    deps: DepsMut,
    info: MessageInfo,
    id: u64,
) -> Result<Response, ContractError> {
    let mut stream = STREAMS
        .may_load(deps.storage, id)?
        .ok_or(ContractError::StreamNotFound {})?;

    if stream.recipient != info.sender {
        return Err(ContractError::NotStreamRecipient {
            recipient: stream.recipient,
        });
    }

    if stream.claimed_amount >= stream.amount {
        return Err(ContractError::StreamFullyClaimed {});
    }

    let block_time = env.block.time.seconds();
    let time_passed = std::cmp::min(block_time, stream.end_time).saturating_sub(stream.start_time);
    let vested = Uint128::from(time_passed) * stream.rate_per_second;
    let released = vested - stream.claimed_amount;

    if released.u128() == 0 {
        return Err(ContractError::NoFundsToClaim {});
    }

    stream.claimed_amount += released;

    STREAMS.save(deps.storage, id, &stream)?;

    let config = CONFIG.load(deps.storage)?;
    let cw20 = Cw20Contract(config.cw20_addr);
    let msg = cw20.call(Cw20ExecuteMsg::Transfer {
        recipient: stream.recipient.to_string(),
        amount: released,
    })?;

    let res = Response::new()
        .add_attribute("method", "withdraw")
        .add_attribute("stream_id", id.to_string())
        .add_attribute("amount", released)
        .add_attribute("recipient", stream.recipient.to_string())
        .add_message(msg);
    Ok(res)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetConfig {} => to_binary(&query_config(deps)?),
        QueryMsg::GetStream { id } => to_binary(&query_stream(deps, id)?),
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        owner: config.owner.into_string(),
        cw20_addr: config.cw20_addr.into_string(),
    })
}

fn query_stream(deps: Deps, id: u64) -> StdResult<StreamResponse> {
    let stream = STREAMS.load(deps.storage, id)?;
    Ok(StreamResponse {
        owner: stream.owner.into_string(),
        recipient: stream.recipient.into_string(),
        amount: stream.amount,
        claimed_amount: stream.claimed_amount,
        rate_per_second: stream.rate_per_second,
        start_time: stream.start_time,
        end_time: stream.end_time,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info, MOCK_CONTRACT_ADDR};
    use cosmwasm_std::{Addr, CosmosMsg, WasmMsg};

    #[test]
    fn initialization() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            owner: None,
            cw20_addr: String::from(MOCK_CONTRACT_ADDR),
        };

        let info = mock_info("creator", &[]);
        instantiate(deps.as_mut(), mock_env(), info, msg).unwrap();

        let msg = QueryMsg::GetConfig {};
        let res = query(deps.as_ref(), mock_env(), msg).unwrap();
        let config: Config = from_binary(&res).unwrap();

        assert_eq!(
            config,
            Config {
                owner: Addr::unchecked("creator"),
                cw20_addr: Addr::unchecked(MOCK_CONTRACT_ADDR)
            }
        );
    }

    #[test]
    fn execute_withdraw() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            owner: None,
            cw20_addr: String::from(MOCK_CONTRACT_ADDR),
        };
        let info = mock_info(MOCK_CONTRACT_ADDR, &[]);
        instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        let sender = Addr::unchecked("Alice").to_string();
        let recipient = Addr::unchecked("Bob").to_string();
        let amount = Uint128::new(200);
        let env = mock_env();
        let start_time = env.block.time.plus_seconds(100).seconds();
        let end_time = env.block.time.plus_seconds(300).seconds();

        let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender,
            amount,
            msg: to_binary(&ReceiveMsg::CreateStream {
                recipient,
                start_time,
                end_time,
            })
            .unwrap(),
        });
        execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        let msg = QueryMsg::GetStream { id: 1 };
        let res = query(deps.as_ref(), mock_env(), msg).unwrap();
        let stream: Stream = from_binary(&res).unwrap();

        assert_eq!(
            stream,
            Stream {
                owner: Addr::unchecked("Alice"),
                recipient: Addr::unchecked("Bob"),
                amount,
                claimed_amount: Uint128::new(0),
                start_time,
                rate_per_second: Uint128::new(1),
                end_time
            }
        );

        // Stream has not started
        let mut info = mock_info(MOCK_CONTRACT_ADDR, &[]);
        info.sender = Addr::unchecked("Bob");
        let msg = ExecuteMsg::Withdraw { id: 1 };
        let err = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(err, ContractError::NoFundsToClaim {});

        // Stream has started so tokens have vested
        let msg = ExecuteMsg::Withdraw { id: 1 };
        let mut info = mock_info(MOCK_CONTRACT_ADDR, &[]);
        let mut env = mock_env();
        info.sender = Addr::unchecked("Bob");
        env.block.time = env.block.time.plus_seconds(150);
        let res = execute(deps.as_mut(), env, info, msg).unwrap();
        let msg = res.messages[0].clone().msg;

        assert_eq!(
            msg,
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: String::from(MOCK_CONTRACT_ADDR),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: String::from("Bob"),
                    amount: Uint128::new(50)
                })
                .unwrap(),
                funds: vec![]
            })
        );

        let msg = QueryMsg::GetStream { id: 1 };
        let res = query(deps.as_ref(), mock_env(), msg).unwrap();
        let stream: Stream = from_binary(&res).unwrap();
        assert_eq!(
            stream,
            Stream {
                owner: Addr::unchecked("Alice"),
                recipient: Addr::unchecked("Bob"),
                amount,
                claimed_amount: Uint128::new(50),
                start_time,
                rate_per_second: Uint128::new(1),
                end_time
            }
        );

        // Stream has ended so claim remaining tokens

        let mut env = mock_env();
        env.block.time = env.block.time.plus_seconds(500);
        let mut info = mock_info(MOCK_CONTRACT_ADDR, &[]);
        info.sender = Addr::unchecked("Bob");
        let msg = ExecuteMsg::Withdraw { id: 1 };
        let res = execute(deps.as_mut(), env, info, msg).unwrap();
        let msg = res.messages[0].clone().msg;

        assert_eq!(
            msg,
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: String::from(MOCK_CONTRACT_ADDR),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: String::from("Bob"),
                    amount: Uint128::new(150)
                })
                .unwrap(),
                funds: vec![]
            })
        );
    }

    #[test]
    fn create_stream_with_refund() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            owner: None,
            cw20_addr: String::from(MOCK_CONTRACT_ADDR),
        };
        let info = mock_info(MOCK_CONTRACT_ADDR, &[]);
        instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        let sender = Addr::unchecked("Alice").to_string();
        let recipient = Addr::unchecked("Bob").to_string();
        let amount = Uint128::new(350);
        let env = mock_env();
        let start_time = env.block.time.plus_seconds(100).seconds();
        let end_time = env.block.time.plus_seconds(400).seconds();

        let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender,
            amount,
            msg: to_binary(&ReceiveMsg::CreateStream {
                recipient,
                start_time,
                end_time,
            })
            .unwrap(),
        });

        // Make sure remaining funds were refunded if duration didn't divide evenly into amount
        let res = execute(deps.as_mut(), env, info, msg).unwrap();
        let refund_msg = res.messages[0].clone().msg;
        assert_eq!(
            refund_msg,
            CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: String::from(MOCK_CONTRACT_ADDR),
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: String::from("Alice"),
                    amount: Uint128::new(50)
                })
                .unwrap(),
                funds: vec![]
            })
        );

        let msg = QueryMsg::GetStream { id: 1 };
        let res = query(deps.as_ref(), mock_env(), msg).unwrap();
        let stream: Stream = from_binary(&res).unwrap();

        assert_eq!(
            stream,
            Stream {
                owner: Addr::unchecked("Alice"),
                recipient: Addr::unchecked("Bob"),
                amount: Uint128::new(300), // original amount - refund
                claimed_amount: Uint128::new(0),
                start_time,
                rate_per_second: Uint128::new(1),
                end_time
            }
        );
    }

    #[test]
    fn invalid_start_time() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            owner: None,
            cw20_addr: String::from(MOCK_CONTRACT_ADDR),
        };
        let mut info = mock_info("Alice", &[]);
        instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        let sender = Addr::unchecked("Alice").to_string();
        let recipient = Addr::unchecked("Bob").to_string();
        let amount = Uint128::new(100);
        let start_time = mock_env().block.time.plus_seconds(100).seconds();
        let end_time = mock_env().block.time.plus_seconds(20).seconds();

        let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender,
            amount,
            msg: to_binary(&ReceiveMsg::CreateStream {
                recipient,
                start_time,
                end_time,
            })
            .unwrap(),
        });
        info.sender = Addr::unchecked(MOCK_CONTRACT_ADDR);
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(err, ContractError::InvalidStartTime {});
    }

    #[test]
    fn invalid_cw20_addr() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            owner: None,
            cw20_addr: String::from(MOCK_CONTRACT_ADDR),
        };
        let mut info = mock_info("Alice", &[]);
        instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        let sender = Addr::unchecked("Alice").to_string();
        let recipient = Addr::unchecked("Bob").to_string();
        let amount = Uint128::new(100);
        let start_time = mock_env().block.time.plus_seconds(100).seconds();
        let end_time = mock_env().block.time.plus_seconds(200).seconds();

        let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender,
            amount,
            msg: to_binary(&ReceiveMsg::CreateStream {
                recipient,
                start_time,
                end_time,
            })
            .unwrap(),
        });
        info.sender = Addr::unchecked("wrongCw20");
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(err, ContractError::Unauthorized {});
    }

    #[test]
    fn invalid_deposit_amount() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            owner: None,
            cw20_addr: String::from(MOCK_CONTRACT_ADDR),
        };
        let mut info = mock_info("Alice", &[]);
        instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        let sender = Addr::unchecked("Alice").to_string();
        let recipient = Addr::unchecked("Bob").to_string();
        let amount = Uint128::new(3);
        let start_time = mock_env().block.time.plus_seconds(100).seconds();
        let end_time = mock_env().block.time.plus_seconds(200).seconds();

        let msg = ExecuteMsg::Receive(Cw20ReceiveMsg {
            sender,
            amount,
            msg: to_binary(&ReceiveMsg::CreateStream {
                recipient,
                start_time,
                end_time,
            })
            .unwrap(),
        });
        info.sender = Addr::unchecked(MOCK_CONTRACT_ADDR);
        let err = execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
        assert_eq!(err, ContractError::AmountLessThanDuration {});
    }
}
