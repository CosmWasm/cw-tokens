use crate::error::ContractError;
use crate::msg::{
    ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg, ReceiveMsg, StreamResponse,
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

    let owner = msg
        .owner
        .and_then(|s| deps.api.addr_validate(s.as_str()).ok())
        .unwrap_or(info.sender);
    let config = Config {
        owner: owner.clone(),
        cw20_addr: deps.api.addr_validate(msg.cw20_addr.as_str())?,
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
        ExecuteMsg::Withdraw { id } => try_withdraw(env, deps, info, id),
    }
}

pub fn try_create_stream(
    env: Env,
    deps: DepsMut,
    owner: String,
    recipient: String,
    amount: Uint128,
    start_time: u64,
    end_time: u64,
) -> Result<Response, ContractError> {
    let validated_owner = deps.api.addr_validate(owner.as_str())?;
    if validated_owner != owner {
        return Err(ContractError::InvalidOwner {});
    }

    let validated_recipient = deps.api.addr_validate(recipient.as_str())?;
    if validated_recipient != recipient {
        return Err(ContractError::InvalidRecipient {});
    }

    let config = CONFIG.load(deps.storage)?;
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

    let duration: Uint128 = end_time.checked_sub(start_time).unwrap().into();

    if amount < duration {
        return Err(ContractError::InvalidDuration {});
    }

    if amount.u128().checked_rem(duration.u128()).unwrap() != 0 {
        return Err(ContractError::InvalidDuration {});
    }

    let rate_per_second: Uint128 = amount.u128().checked_div(duration.u128()).unwrap().into();

    let stream = Stream {
        owner: validated_owner,
        recipient: validated_recipient,
        amount,
        claimed_amount: Uint128::zero(),
        start_time,
        end_time,
        rate_per_second,
    };
    save_stream(deps, &stream)?;

    Ok(Response::new()
        .add_attribute("method", "try_create_stream")
        .add_attribute("owner", owner)
        .add_attribute("recipient", recipient)
        .add_attribute("amount", amount)
        .add_attribute("start_time", start_time.to_string())
        .add_attribute("end_time", end_time.to_string()))
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
            recipient,
            start_time,
            end_time,
        } => try_create_stream(
            env,
            deps,
            wrapped.sender,
            recipient,
            wrapped.amount,
            start_time,
            end_time,
        ),
    }
}

pub fn try_withdraw(
    env: Env,
    deps: DepsMut,
    info: MessageInfo,
    id: u64,
) -> Result<Response, ContractError> {
    let mut stream = STREAMS.load(deps.storage, id)?;
    if stream.recipient != info.sender {
        return Err(ContractError::NotStreamRecipient {});
    }

    if stream.claimed_amount >= stream.amount {
        return Err(ContractError::StreamFullyClaimed {});
    }

    let block_time = env.block.time.seconds();
    if stream.start_time >= block_time {
        return Err(ContractError::StreamNotStarted {});
    }

    let unclaimed_amount = u128::from(block_time)
        .checked_sub(stream.start_time.into())
        .unwrap()
        .checked_mul(stream.rate_per_second.u128())
        .unwrap()
        .checked_sub(stream.claimed_amount.u128())
        .unwrap();

    stream.claimed_amount = stream
        .claimed_amount
        .u128()
        .checked_add(unclaimed_amount)
        .unwrap()
        .into();

    STREAMS.save(deps.storage, id, &stream)?;

    let config = CONFIG.load(deps.storage)?;
    let cw20 = Cw20Contract(config.cw20_addr);
    let msg = cw20.call(Cw20ExecuteMsg::Transfer {
        recipient: stream.recipient.to_string(),
        amount: unclaimed_amount.into(),
    })?;

    let res = Response::new()
        .add_attribute("method", "try_withdraw")
        .add_attribute("stream_id", id.to_string())
        .add_attribute("amount", Uint128::from(unclaimed_amount))
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
    fn try_withdraw() {
        let mut deps = mock_dependencies();
        let msg = InstantiateMsg {
            owner: None,
            cw20_addr: String::from(MOCK_CONTRACT_ADDR),
        };
        let mut info = mock_info(MOCK_CONTRACT_ADDR, &[]);
        instantiate(deps.as_mut(), mock_env(), info.clone(), msg).unwrap();

        let sender = Addr::unchecked("Alice").to_string();
        let recipient = Addr::unchecked("Bob").to_string();
        let amount = Uint128::new(200);
        let mut env = mock_env();
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
        execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

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

        let msg = ExecuteMsg::Withdraw { id: 1 };
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
        match err {
            ContractError::InvalidStartTime {} => {}
            e => panic!("unexpected error: {}", e),
        }
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

        match err {
            ContractError::Unauthorized {} => {}
            e => panic!("unexpected error: {}", e),
        }
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
        execute(deps.as_mut(), mock_env(), info, msg).unwrap_err();
    }
}
