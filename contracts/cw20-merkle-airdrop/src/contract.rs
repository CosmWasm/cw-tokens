use crate::enumerable::query_all_address_map;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    attr, from_binary, to_binary, Addr, BankMsg, Binary, Coin, CosmosMsg, Deps, DepsMut, Env,
    MessageInfo, Response, StdResult, Uint128,
};
use cw2::{get_contract_version, set_contract_version};
use cw20::{Cw20Contract, Cw20ExecuteMsg};
use cw_utils::{Expiration, Scheduled};
use sha2::Digest;
use std::convert::TryInto;

use crate::error::ContractError;
use crate::helpers::CosmosSignature;
use crate::msg::{
    AccountMapResponse, ConfigResponse, ExecuteMsg, InstantiateMsg, IsClaimedResponse,
    LatestStageResponse, MerkleRootResponse, MigrateMsg, QueryMsg, SignatureInfo,
    TotalClaimedResponse,
};
use crate::state::{
    Config, CLAIM, CONFIG, HRP, LATEST_STAGE, MERKLE_ROOT, STAGE_ACCOUNT_MAP, STAGE_AMOUNT,
    STAGE_AMOUNT_CLAIMED, STAGE_EXPIRATION, STAGE_START,
};

// Version info, for migration info
const CONTRACT_NAME: &str = "crates.io:cw20-merkle-airdrop";
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
        .map_or(Ok(info.sender), |o| deps.api.addr_validate(&o))?;

    let stage = 0;
    LATEST_STAGE.save(deps.storage, &stage)?;

    make_config(deps, Some(owner), msg.cw20_token_address, msg.native_token)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::UpdateConfig {
            new_owner,
            new_cw20_address,
            new_native_token,
        } => execute_update_config(
            deps,
            env,
            info,
            new_owner,
            new_cw20_address,
            new_native_token,
        ),
        ExecuteMsg::RegisterMerkleRoot {
            merkle_root,
            expiration,
            start,
            total_amount,
            hrp,
        } => execute_register_merkle_root(
            deps,
            env,
            info,
            merkle_root,
            expiration,
            start,
            total_amount,
            hrp,
        ),
        ExecuteMsg::Claim {
            stage,
            amount,
            proof,
            sig_info,
        } => execute_claim(deps, env, info, stage, amount, proof, sig_info),
        ExecuteMsg::Burn { stage } => execute_burn(deps, env, info, stage),
        ExecuteMsg::Withdraw { stage, address } => {
            execute_withdraw(deps, env, info, stage, address)
        }
    }
}

pub fn make_config(
    deps: DepsMut,
    owner: Option<Addr>,
    cw20_token_address: Option<String>,
    native_token: Option<String>,
) -> Result<Response, ContractError> {
    let config: Config = match (native_token, cw20_token_address) {
        (Some(native), None) => Ok(Config {
            owner,
            cw20_token_address: None,
            native_token: Some(native),
        }),
        (None, Some(cw20_addr)) => Ok(Config {
            owner,
            cw20_token_address: Some(deps.api.addr_validate(&cw20_addr)?),
            native_token: None,
        }),
        _ => Err(ContractError::InvalidTokenType {}),
    }?;
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::default())
}

pub fn execute_update_config(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    new_owner: Option<String>,
    cw20_token_address: Option<String>,
    native_token: Option<String>,
) -> Result<Response, ContractError> {
    // authorize owner
    let cfg = CONFIG.load(deps.storage)?;
    let owner = cfg.owner.ok_or(ContractError::Unauthorized {})?;
    if info.sender != owner {
        return Err(ContractError::Unauthorized {});
    }

    // if owner some validated to addr, otherwise set to none
    let mut tmp_owner = None;
    if let Some(addr) = new_owner {
        tmp_owner = Some(deps.api.addr_validate(&addr)?)
    }

    make_config(deps, tmp_owner, cw20_token_address, native_token)?;

    Ok(Response::new().add_attribute("action", "update_config"))
}

#[allow(clippy::too_many_arguments)]
pub fn execute_register_merkle_root(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    merkle_root: String,
    expiration: Option<Expiration>,
    start: Option<Scheduled>,
    total_amount: Option<Uint128>,
    hrp: Option<String>,
) -> Result<Response, ContractError> {
    let cfg = CONFIG.load(deps.storage)?;

    // if owner set validate, otherwise unauthorized
    let owner = cfg.owner.ok_or(ContractError::Unauthorized {})?;
    if info.sender != owner {
        return Err(ContractError::Unauthorized {});
    }

    // check merkle root length
    let mut root_buf: [u8; 32] = [0; 32];
    hex::decode_to_slice(&merkle_root, &mut root_buf)?;

    let stage = LATEST_STAGE.update(deps.storage, |stage| -> StdResult<_> { Ok(stage + 1) })?;

    MERKLE_ROOT.save(deps.storage, stage, &merkle_root)?;
    LATEST_STAGE.save(deps.storage, &stage)?;

    // save expiration
    let exp = expiration.unwrap_or(Expiration::Never {});
    STAGE_EXPIRATION.save(deps.storage, stage, &exp)?;

    // save start
    if let Some(start) = start {
        STAGE_START.save(deps.storage, stage, &start)?;
    }

    // save hrp
    if let Some(hrp) = hrp {
        HRP.save(deps.storage, stage, &hrp)?;
    }

    // save total airdropped amount
    let amount = total_amount.unwrap_or_else(Uint128::zero);
    STAGE_AMOUNT.save(deps.storage, stage, &amount)?;
    STAGE_AMOUNT_CLAIMED.save(deps.storage, stage, &Uint128::zero())?;

    Ok(Response::new().add_attributes(vec![
        attr("action", "register_merkle_root"),
        attr("stage", stage.to_string()),
        attr("merkle_root", merkle_root),
        attr("total_amount", amount),
    ]))
}

pub fn execute_claim(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    stage: u8,
    amount: Uint128,
    proof: Vec<String>,
    sig_info: Option<SignatureInfo>,
) -> Result<Response, ContractError> {
    // airdrop begun
    let start = STAGE_START.may_load(deps.storage, stage)?;
    if let Some(start) = start {
        if !start.is_triggered(&env.block) {
            return Err(ContractError::StageNotBegun { stage, start });
        }
    }
    // not expired
    let expiration = STAGE_EXPIRATION.load(deps.storage, stage)?;
    if expiration.is_expired(&env.block) {
        return Err(ContractError::StageExpired { stage, expiration });
    }

    // if present verify signature and extract external address or use info.sender as proof
    // if signature is not present in the message, verification will fail since info.sender is not present in the merkle root
    let proof_addr = match sig_info {
        None => info.sender.to_string(),
        Some(sig) => {
            // verify signature
            let cosmos_signature: CosmosSignature = from_binary(&sig.signature)?;
            cosmos_signature.verify(deps.as_ref(), &sig.claim_msg)?;
            // get airdrop stage bech32 prefix and derive proof address from public key
            let hrp = HRP.load(deps.storage, stage)?;
            let proof_addr = cosmos_signature.derive_addr_from_pubkey(hrp.as_str())?;

            if sig.extract_addr()? != info.sender {
                return Err(ContractError::VerificationFailed {});
            }

            // Save external address index
            STAGE_ACCOUNT_MAP.save(
                deps.storage,
                (stage, proof_addr.clone()),
                &info.sender.to_string(),
            )?;

            proof_addr
        }
    };

    // verify not claimed
    let claimed = CLAIM.may_load(deps.storage, (proof_addr.clone(), stage))?;
    if claimed.is_some() {
        return Err(ContractError::Claimed {});
    }

    // verify merkle root
    let config = CONFIG.load(deps.storage)?;
    let merkle_root = MERKLE_ROOT.load(deps.storage, stage)?;

    let user_input = format!("{}{}", proof_addr, amount);
    let hash = sha2::Sha256::digest(user_input.as_bytes())
        .as_slice()
        .try_into()
        .map_err(|_| ContractError::WrongLength {})?;

    let hash = proof.into_iter().try_fold(hash, |hash, p| {
        let mut proof_buf = [0; 32];
        hex::decode_to_slice(p, &mut proof_buf)?;
        let mut hashes = [hash, proof_buf];
        hashes.sort_unstable();
        sha2::Sha256::digest(&hashes.concat())
            .as_slice()
            .try_into()
            .map_err(|_| ContractError::WrongLength {})
    })?;

    let mut root_buf: [u8; 32] = [0; 32];
    hex::decode_to_slice(merkle_root, &mut root_buf)?;
    if root_buf != hash {
        return Err(ContractError::VerificationFailed {});
    }

    // Update claim index to the current stage
    CLAIM.save(deps.storage, (proof_addr, stage), &true)?;

    // Update total claimed to reflect
    let mut claimed_amount = STAGE_AMOUNT_CLAIMED.load(deps.storage, stage)?;
    claimed_amount += amount;
    STAGE_AMOUNT_CLAIMED.save(deps.storage, stage, &claimed_amount)?;

    let message: CosmosMsg = match (config.cw20_token_address, config.native_token) {
        (Some(cw20_addr), None) => {
            let msg = Cw20ExecuteMsg::Transfer {
                recipient: info.sender.to_string(),
                amount,
            };
            Cw20Contract(cw20_addr)
                .call(msg)
                .map_err(ContractError::Std)
        }
        (None, Some(native)) => {
            let balance = deps
                .querier
                .query_balance(env.contract.address, native.clone())?;
            if balance.amount < amount {
                return Err(ContractError::InsufficientFunds {
                    balance: balance.amount,
                    amount,
                });
            }
            let msg = BankMsg::Send {
                to_address: info.sender.to_string(),
                amount: vec![Coin {
                    denom: native,
                    amount,
                }],
            };
            Ok(CosmosMsg::Bank(msg))
        }
        _ => Err(ContractError::InvalidTokenType {}),
    }?;
    let res = Response::new().add_message(message).add_attributes(vec![
        attr("action", "claim"),
        attr("stage", stage.to_string()),
        attr("address", info.sender.to_string()),
        attr("amount", amount),
    ]);
    Ok(res)
}

pub fn execute_burn(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    stage: u8,
) -> Result<Response, ContractError> {
    // authorize owner
    let cfg = CONFIG.load(deps.storage)?;
    let owner = cfg.owner.ok_or(ContractError::Unauthorized {})?;
    if info.sender != owner {
        return Err(ContractError::Unauthorized {});
    }

    // make sure is expired
    let expiration = STAGE_EXPIRATION.load(deps.storage, stage)?;
    if !expiration.is_expired(&env.block) {
        return Err(ContractError::StageNotExpired { stage, expiration });
    }

    // Get total amount per stage and total claimed
    let total_amount = STAGE_AMOUNT.load(deps.storage, stage)?;
    let claimed_amount = STAGE_AMOUNT_CLAIMED.load(deps.storage, stage)?;

    // impossible but who knows
    if claimed_amount > total_amount {
        return Err(ContractError::Unauthorized {});
    }

    // Get balance
    let balance_to_burn = total_amount - claimed_amount;

    // Burn the tokens and response
    let message: CosmosMsg = match (cfg.cw20_token_address, cfg.native_token) {
        (Some(cw20_addr), None) => {
            let msg = Cw20ExecuteMsg::Burn {
                amount: balance_to_burn,
            };
            Cw20Contract(cw20_addr)
                .call(msg)
                .map_err(ContractError::Std)
        }
        (None, Some(native)) => {
            let balance = deps
                .querier
                .query_balance(env.contract.address, native.clone())?;
            if balance.amount < balance_to_burn {
                return Err(ContractError::InsufficientFunds {
                    balance: balance.amount,
                    amount: balance_to_burn,
                });
            }
            let msg = BankMsg::Burn {
                amount: vec![Coin {
                    denom: native,
                    amount: balance_to_burn,
                }],
            };
            Ok(CosmosMsg::Bank(msg))
        }
        _ => Err(ContractError::InvalidTokenType {}),
    }?;
    let res = Response::new().add_message(message).add_attributes(vec![
        attr("action", "burn"),
        attr("stage", stage.to_string()),
        attr("address", info.sender),
        attr("amount", balance_to_burn),
    ]);
    Ok(res)
}

pub fn execute_withdraw(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    stage: u8,
    address: String,
) -> Result<Response, ContractError> {
    // authorize owner
    let cfg = CONFIG.load(deps.storage)?;
    let owner = cfg.owner.ok_or(ContractError::Unauthorized {})?;
    if info.sender != owner {
        return Err(ContractError::Unauthorized {});
    }

    // make sure is expired
    let expiration = STAGE_EXPIRATION.load(deps.storage, stage)?;
    if !expiration.is_expired(&env.block) {
        return Err(ContractError::StageNotExpired { stage, expiration });
    }

    // Get total amount per stage and total claimed
    let total_amount = STAGE_AMOUNT.load(deps.storage, stage)?;
    let claimed_amount = STAGE_AMOUNT_CLAIMED.load(deps.storage, stage)?;

    // impossible but who knows
    if claimed_amount > total_amount {
        return Err(ContractError::Unauthorized {});
    }

    // Get balance
    let balance_to_withdraw = total_amount - claimed_amount;

    // Validate address
    let recipient = deps.api.addr_validate(&address)?;

    // Withdraw the tokens and response
    let message: CosmosMsg = match (cfg.cw20_token_address, cfg.native_token) {
        (Some(cw20_addr), None) => {
            let msg = Cw20ExecuteMsg::Transfer {
                recipient: recipient.into(),
                amount: balance_to_withdraw,
            };
            Cw20Contract(cw20_addr)
                .call(msg)
                .map_err(ContractError::Std)
        }
        (None, Some(native)) => {
            let balance = deps
                .querier
                .query_balance(env.contract.address, native.clone())?;
            if balance.amount < balance_to_withdraw {
                return Err(ContractError::InsufficientFunds {
                    balance: balance.amount,
                    amount: balance_to_withdraw,
                });
            }
            let msg = BankMsg::Send {
                to_address: recipient.into(),
                amount: vec![Coin {
                    denom: native,
                    amount: balance_to_withdraw,
                }],
            };
            Ok(CosmosMsg::Bank(msg))
        }
        _ => Err(ContractError::InvalidTokenType {}),
    }?;
    let res = Response::new().add_message(message).add_attributes(vec![
        attr("action", "withdraw"),
        attr("stage", stage.to_string()),
        attr("address", info.sender),
        attr("amount", balance_to_withdraw),
        attr("recipient", address),
    ]);
    Ok(res)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?),
        QueryMsg::MerkleRoot { stage } => to_binary(&query_merkle_root(deps, stage)?),
        QueryMsg::LatestStage {} => to_binary(&query_latest_stage(deps)?),
        QueryMsg::IsClaimed { stage, address } => {
            to_binary(&query_is_claimed(deps, stage, address)?)
        }
        QueryMsg::TotalClaimed { stage } => to_binary(&query_total_claimed(deps, stage)?),
        QueryMsg::AccountMap {
            stage,
            external_address,
        } => to_binary(&query_address_map(deps, stage, external_address)?),
        QueryMsg::AllAccountMaps {
            stage,
            start_after,
            limit,
        } => to_binary(&query_all_address_map(deps, stage, start_after, limit)?),
    }
}

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let cfg = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        owner: cfg.owner.map(|o| o.to_string()),
        cw20_token_address: cfg.cw20_token_address.map(|o| o.to_string()),
        native_token: cfg.native_token,
    })
}

pub fn query_merkle_root(deps: Deps, stage: u8) -> StdResult<MerkleRootResponse> {
    let merkle_root = MERKLE_ROOT.load(deps.storage, stage)?;
    let expiration = STAGE_EXPIRATION.load(deps.storage, stage)?;
    let start = STAGE_START.may_load(deps.storage, stage)?;
    let total_amount = STAGE_AMOUNT.load(deps.storage, stage)?;

    let resp = MerkleRootResponse {
        stage,
        merkle_root,
        expiration,
        start,
        total_amount,
    };

    Ok(resp)
}

pub fn query_latest_stage(deps: Deps) -> StdResult<LatestStageResponse> {
    let latest_stage = LATEST_STAGE.load(deps.storage)?;
    let resp = LatestStageResponse { latest_stage };

    Ok(resp)
}

pub fn query_is_claimed(deps: Deps, stage: u8, address: String) -> StdResult<IsClaimedResponse> {
    let is_claimed = CLAIM
        .may_load(deps.storage, (address, stage))?
        .unwrap_or(false);
    let resp = IsClaimedResponse { is_claimed };

    Ok(resp)
}

pub fn query_total_claimed(deps: Deps, stage: u8) -> StdResult<TotalClaimedResponse> {
    let total_claimed = STAGE_AMOUNT_CLAIMED.load(deps.storage, stage)?;
    let resp = TotalClaimedResponse { total_claimed };

    Ok(resp)
}

pub fn query_address_map(
    deps: Deps,
    stage: u8,
    external_address: String,
) -> StdResult<AccountMapResponse> {
    let host_address = STAGE_ACCOUNT_MAP.load(deps.storage, (stage, external_address.clone()))?;
    let resp = AccountMapResponse {
        host_address,
        external_address,
    };

    Ok(resp)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    let version = get_contract_version(deps.storage)?;
    if version.contract != CONTRACT_NAME {
        return Err(ContractError::CannotMigrate {
            previous_contract: version.contract,
        });
    }
    Ok(Response::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::msg::SignatureInfo;
    use cosmwasm_std::testing::{
        mock_dependencies, mock_dependencies_with_balance, mock_env, mock_info,
    };
    use cosmwasm_std::{from_binary, from_slice, CosmosMsg, SubMsg, WasmMsg};
    use serde::Deserialize;

    #[test]
    fn proper_instantiation_cw20() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            owner: Some("owner0000".to_string()),
            cw20_token_address: Some("anchor0000".to_string()),
            native_token: None,
        };

        let env = mock_env();
        let info = mock_info("addr0000", &[]);

        // we can just call .unwrap() to assert this was a success
        let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        // it worked, let's query the state
        let res = query(deps.as_ref(), env.clone(), QueryMsg::Config {}).unwrap();
        let config: ConfigResponse = from_binary(&res).unwrap();
        assert_eq!("owner0000", config.owner.unwrap().as_str());
        assert_eq!("anchor0000", config.cw20_token_address.unwrap().as_str());
        assert_eq!(None, config.native_token);

        let res = query(deps.as_ref(), env, QueryMsg::LatestStage {}).unwrap();
        let latest_stage: LatestStageResponse = from_binary(&res).unwrap();
        assert_eq!(0u8, latest_stage.latest_stage);
    }

    #[test]
    fn proper_instantiation_native() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            owner: Some("owner0000".to_string()),
            cw20_token_address: None,
            native_token: Some(String::from("ujunox")),
        };

        let env = mock_env();
        let info = mock_info("addr0000", &[]);

        // we can just call .unwrap() to assert this was a success
        let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        // it worked, let's query the state
        let res = query(deps.as_ref(), env.clone(), QueryMsg::Config {}).unwrap();
        let config: ConfigResponse = from_binary(&res).unwrap();
        assert_eq!("owner0000", config.owner.unwrap().as_str());
        assert_eq!("ujunox", config.native_token.unwrap().as_str());
        assert_eq!(None, config.cw20_token_address);

        let res = query(deps.as_ref(), env, QueryMsg::LatestStage {}).unwrap();
        let latest_stage: LatestStageResponse = from_binary(&res).unwrap();
        assert_eq!(0u8, latest_stage.latest_stage);
    }

    #[test]
    fn failed_instantiation_native_and_cw20() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            owner: Some("owner0000".to_string()),
            cw20_token_address: Some("anchor0000".to_string()),
            native_token: Some(String::from("ujunox")),
        };

        let env = mock_env();
        let info = mock_info("addr0000", &[]);

        assert_eq!(
            Err(ContractError::InvalidTokenType {}),
            instantiate(deps.as_mut(), env, info, msg)
        );
    }

    #[test]
    fn update_config() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            owner: None,
            cw20_token_address: Some("anchor0000".to_string()),
            native_token: None,
        };

        let env = mock_env();
        let info = mock_info("owner0000", &[]);
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        // update owner
        let env = mock_env();
        let info = mock_info("owner0000", &[]);
        let msg = ExecuteMsg::UpdateConfig {
            new_owner: Some("owner0001".to_string()),
            new_cw20_address: Some("cw20_0000".to_string()),
            new_native_token: None,
        };

        let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // it worked, let's query the state
        let res = query(deps.as_ref(), env, QueryMsg::Config {}).unwrap();
        let config: ConfigResponse = from_binary(&res).unwrap();
        assert_eq!("owner0001", config.owner.unwrap().as_str());
        assert_eq!("cw20_0000", config.cw20_token_address.unwrap().as_str());

        // Unauthorized err
        let env = mock_env();
        let info = mock_info("owner0000", &[]);
        let msg = ExecuteMsg::UpdateConfig {
            new_owner: None,
            new_cw20_address: Some("cw20_0001".to_string()),
            new_native_token: None,
        };

        let res = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(res, ContractError::Unauthorized {});

        //update with native token
        let env = mock_env();
        let info = mock_info("owner0001", &[]);
        let msg = ExecuteMsg::UpdateConfig {
            new_owner: Some("owner0001".to_string()),
            new_cw20_address: None,
            new_native_token: Some("ujunox".to_string()),
        };

        let _res = execute(deps.as_mut(), env.clone(), info, msg).ok();

        let query_result = query(deps.as_ref(), env, QueryMsg::Config {}).unwrap();
        let config: ConfigResponse = from_binary(&query_result).unwrap();
        assert_eq!("owner0001", config.owner.unwrap().as_str());
        assert_eq!("ujunox", config.native_token.unwrap().as_str());

        //update cw20_address and native token together
        let env = mock_env();
        let info = mock_info("owner0001", &[]);
        let msg = ExecuteMsg::UpdateConfig {
            new_owner: Some("owner0001".to_string()),
            new_cw20_address: Some("cw20_0001".to_string()),
            new_native_token: Some("ujunox".to_string()),
        };

        let res = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(res, ContractError::InvalidTokenType {});
    }

    #[test]
    fn register_merkle_root() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            owner: Some("owner0000".to_string()),
            cw20_token_address: Some("anchor0000".to_string()),
            native_token: None,
        };

        let env = mock_env();
        let info = mock_info("addr0000", &[]);
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        // register new merkle root
        let env = mock_env();
        let info = mock_info("owner0000", &[]);
        let msg = ExecuteMsg::RegisterMerkleRoot {
            merkle_root: "634de21cde1044f41d90373733b0f0fb1c1c71f9652b905cdf159e73c4cf0d37"
                .to_string(),
            expiration: None,
            start: None,
            total_amount: None,
            hrp: None,
        };

        let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
        assert_eq!(
            res.attributes,
            vec![
                attr("action", "register_merkle_root"),
                attr("stage", "1"),
                attr(
                    "merkle_root",
                    "634de21cde1044f41d90373733b0f0fb1c1c71f9652b905cdf159e73c4cf0d37",
                ),
                attr("total_amount", "0"),
            ]
        );

        let res = query(deps.as_ref(), env.clone(), QueryMsg::LatestStage {}).unwrap();
        let latest_stage: LatestStageResponse = from_binary(&res).unwrap();
        assert_eq!(1u8, latest_stage.latest_stage);

        let res = query(
            deps.as_ref(),
            env,
            QueryMsg::MerkleRoot {
                stage: latest_stage.latest_stage,
            },
        )
        .unwrap();
        let merkle_root: MerkleRootResponse = from_binary(&res).unwrap();
        assert_eq!(
            "634de21cde1044f41d90373733b0f0fb1c1c71f9652b905cdf159e73c4cf0d37".to_string(),
            merkle_root.merkle_root
        );
    }

    const TEST_DATA_1: &[u8] = include_bytes!("../testdata/airdrop_stage_1_test_data.json");
    const TEST_DATA_2: &[u8] = include_bytes!("../testdata/airdrop_stage_2_test_data.json");

    #[derive(Deserialize, Debug)]
    struct Encoded {
        account: String,
        amount: Uint128,
        root: String,
        proofs: Vec<String>,
        signed_msg: Option<SignatureInfo>,
        hrp: Option<String>,
    }

    #[test]
    fn claim_cw20() {
        // Run test 1
        let mut deps = mock_dependencies();
        let test_data: Encoded = from_slice(TEST_DATA_1).unwrap();

        let msg = InstantiateMsg {
            owner: Some("owner0000".to_string()),
            cw20_token_address: Some("token0000".to_string()),
            native_token: None,
        };

        let env = mock_env();
        let info = mock_info("addr0000", &[]);
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        let env = mock_env();
        let info = mock_info("owner0000", &[]);
        let msg = ExecuteMsg::RegisterMerkleRoot {
            merkle_root: test_data.root,
            expiration: None,
            start: None,
            total_amount: None,
            hrp: None,
        };
        let _res = execute(deps.as_mut(), env, info, msg).unwrap();

        let msg = ExecuteMsg::Claim {
            amount: test_data.amount,
            stage: 1u8,
            proof: test_data.proofs,
            sig_info: None,
        };

        let env = mock_env();
        let info = mock_info(test_data.account.as_str(), &[]);
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
        let expected = SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "token0000".to_string(),
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: test_data.account.clone(),
                amount: test_data.amount,
            })
            .unwrap(),
        }));
        assert_eq!(res.messages, vec![expected]);

        assert_eq!(
            res.attributes,
            vec![
                attr("action", "claim"),
                attr("stage", "1"),
                attr("address", test_data.account.clone()),
                attr("amount", test_data.amount),
            ]
        );

        // Check total claimed on stage 1
        assert_eq!(
            from_binary::<TotalClaimedResponse>(
                &query(
                    deps.as_ref(),
                    env.clone(),
                    QueryMsg::TotalClaimed { stage: 1 },
                )
                .unwrap()
            )
            .unwrap()
            .total_claimed,
            test_data.amount
        );

        // Check address is claimed
        assert!(
            from_binary::<IsClaimedResponse>(
                &query(
                    deps.as_ref(),
                    env.clone(),
                    QueryMsg::IsClaimed {
                        stage: 1,
                        address: test_data.account,
                    },
                )
                .unwrap()
            )
            .unwrap()
            .is_claimed
        );

        // check error on double claim
        let res = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(res, ContractError::Claimed {});

        // Second test
        let test_data: Encoded = from_slice(TEST_DATA_2).unwrap();

        // register new drop
        let env = mock_env();
        let info = mock_info("owner0000", &[]);
        let msg = ExecuteMsg::RegisterMerkleRoot {
            merkle_root: test_data.root,
            expiration: None,
            start: None,
            total_amount: None,
            hrp: None,
        };
        let _res = execute(deps.as_mut(), env, info, msg).unwrap();

        // Claim next airdrop
        let msg = ExecuteMsg::Claim {
            amount: test_data.amount,
            stage: 2u8,
            proof: test_data.proofs,
            sig_info: None,
        };

        let env = mock_env();
        let info = mock_info(test_data.account.as_str(), &[]);
        let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
        let expected: SubMsg<_> = SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "token0000".to_string(),
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: test_data.account.clone(),
                amount: test_data.amount,
            })
            .unwrap(),
        }));
        assert_eq!(res.messages, vec![expected]);

        assert_eq!(
            res.attributes,
            vec![
                attr("action", "claim"),
                attr("stage", "2"),
                attr("address", test_data.account),
                attr("amount", test_data.amount),
            ]
        );

        // Check total claimed on stage 2
        assert_eq!(
            from_binary::<TotalClaimedResponse>(
                &query(deps.as_ref(), env, QueryMsg::TotalClaimed { stage: 2 }).unwrap()
            )
            .unwrap()
            .total_claimed,
            test_data.amount
        );

        // Drop stage three with external sigs
    }

    #[test]
    fn claim_native() {
        // Run test 1
        let mut deps = mock_dependencies_with_balance(&[Coin {
            denom: "ujunox".to_string(),
            amount: Uint128::new(1234567),
        }]);
        let test_data: Encoded = from_slice(TEST_DATA_1).unwrap();

        let msg = InstantiateMsg {
            owner: Some("owner0000".to_string()),
            cw20_token_address: None,
            native_token: Some("ujunox".to_string()),
        };

        let env = mock_env();
        let info = mock_info("addr0000", &[]);
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        let env = mock_env();
        let info = mock_info("owner0000", &[]);
        let msg = ExecuteMsg::RegisterMerkleRoot {
            merkle_root: test_data.root,
            expiration: None,
            start: None,
            total_amount: None,
            hrp: None,
        };
        let _res = execute(deps.as_mut(), env, info, msg).unwrap();

        let msg = ExecuteMsg::Claim {
            amount: test_data.amount,
            stage: 1u8,
            proof: test_data.proofs,
            sig_info: None,
        };

        let env = mock_env();
        let info = mock_info(test_data.account.as_str(), &[]);
        let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
        let expected = SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: test_data.account.clone(),
            amount: vec![Coin {
                denom: "ujunox".to_string(),
                amount: test_data.amount,
            }],
        }));
        assert_eq!(res.messages, vec![expected]);

        assert_eq!(
            res.attributes,
            vec![
                attr("action", "claim"),
                attr("stage", "1"),
                attr("address", test_data.account.clone()),
                attr("amount", test_data.amount),
            ]
        );

        // Check total claimed on stage 1
        assert_eq!(
            from_binary::<TotalClaimedResponse>(
                &query(
                    deps.as_ref(),
                    env.clone(),
                    QueryMsg::TotalClaimed { stage: 1 },
                )
                .unwrap()
            )
            .unwrap()
            .total_claimed,
            test_data.amount
        );

        // Check address is claimed
        assert!(
            from_binary::<IsClaimedResponse>(
                &query(
                    deps.as_ref(),
                    env.clone(),
                    QueryMsg::IsClaimed {
                        stage: 1,
                        address: test_data.account,
                    },
                )
                .unwrap()
            )
            .unwrap()
            .is_claimed
        );

        // check error on double claim
        let res = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(res, ContractError::Claimed {});

        // Second test
        let test_data: Encoded = from_slice(TEST_DATA_2).unwrap();

        // register new drop
        let env = mock_env();
        let info = mock_info("owner0000", &[]);
        let msg = ExecuteMsg::RegisterMerkleRoot {
            merkle_root: test_data.root,
            expiration: None,
            start: None,
            total_amount: None,
            hrp: None,
        };
        let _res = execute(deps.as_mut(), env, info, msg).unwrap();

        // Claim next airdrop
        let msg = ExecuteMsg::Claim {
            amount: test_data.amount,
            stage: 2u8,
            proof: test_data.proofs,
            sig_info: None,
        };

        let env = mock_env();
        let info = mock_info(test_data.account.as_str(), &[]);
        let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
        let expected = SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: test_data.account.clone(),
            amount: vec![Coin {
                denom: "ujunox".to_string(),
                amount: test_data.amount,
            }],
        }));
        assert_eq!(res.messages, vec![expected]);

        assert_eq!(
            res.attributes,
            vec![
                attr("action", "claim"),
                attr("stage", "2"),
                attr("address", test_data.account),
                attr("amount", test_data.amount),
            ]
        );

        // Check total claimed on stage 2
        assert_eq!(
            from_binary::<TotalClaimedResponse>(
                &query(deps.as_ref(), env, QueryMsg::TotalClaimed { stage: 2 }).unwrap()
            )
            .unwrap()
            .total_claimed,
            test_data.amount
        );
    }

    #[test]
    fn claim_native_insufficient_funds() {
        // Run test 1
        let mut deps = mock_dependencies_with_balance(&[Coin {
            denom: "ujunox".to_string(),
            amount: Uint128::zero(),
        }]);
        let test_data: Encoded = from_slice(TEST_DATA_1).unwrap();

        let msg = InstantiateMsg {
            owner: Some("owner0000".to_string()),
            cw20_token_address: None,
            native_token: Some("ujunox".to_string()),
        };

        let env = mock_env();
        let info = mock_info("addr0000", &[]);
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        let env = mock_env();
        let info = mock_info("owner0000", &[]);
        let msg = ExecuteMsg::RegisterMerkleRoot {
            merkle_root: test_data.root,
            expiration: None,
            start: None,
            total_amount: None,
            hrp: None,
        };
        let _res = execute(deps.as_mut(), env, info, msg).unwrap();

        let msg = ExecuteMsg::Claim {
            amount: test_data.amount,
            stage: 1u8,
            proof: test_data.proofs,
            sig_info: None,
        };

        let env = mock_env();
        let info = mock_info(test_data.account.as_str(), &[]);
        let res = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(
            ContractError::InsufficientFunds {
                balance: Uint128::zero(),
                amount: test_data.amount
            },
            res
        );
    }

    const TEST_DATA_1_MULTI: &[u8] =
        include_bytes!("../testdata/airdrop_stage_1_test_multi_data.json");

    #[derive(Deserialize, Debug)]
    struct Proof {
        account: String,
        amount: Uint128,
        proofs: Vec<String>,
    }

    #[derive(Deserialize, Debug)]
    struct MultipleData {
        total_claimed_amount: Uint128,
        root: String,
        accounts: Vec<Proof>,
    }

    #[test]
    fn multiple_claim_cw20() {
        // Run test 1
        let mut deps = mock_dependencies();
        let test_data: MultipleData = from_slice(TEST_DATA_1_MULTI).unwrap();

        let msg = InstantiateMsg {
            owner: Some("owner0000".to_string()),
            cw20_token_address: Some("token0000".to_string()),
            native_token: None,
        };

        let env = mock_env();
        let info = mock_info("addr0000", &[]);
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        let env = mock_env();
        let info = mock_info("owner0000", &[]);
        let msg = ExecuteMsg::RegisterMerkleRoot {
            merkle_root: test_data.root,
            expiration: None,
            start: None,
            total_amount: None,
            hrp: None,
        };
        let _res = execute(deps.as_mut(), env, info, msg).unwrap();

        // Loop accounts and claim
        for account in test_data.accounts.iter() {
            let msg = ExecuteMsg::Claim {
                amount: account.amount,
                stage: 1u8,
                proof: account.proofs.clone(),
                sig_info: None,
            };

            let env = mock_env();
            let info = mock_info(account.account.as_str(), &[]);
            let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
            let expected = SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "token0000".to_string(),
                funds: vec![],
                msg: to_binary(&Cw20ExecuteMsg::Transfer {
                    recipient: account.account.clone(),
                    amount: account.amount,
                })
                .unwrap(),
            }));
            assert_eq!(res.messages, vec![expected]);

            assert_eq!(
                res.attributes,
                vec![
                    attr("action", "claim"),
                    attr("stage", "1"),
                    attr("address", account.account.clone()),
                    attr("amount", account.amount),
                ]
            );
        }

        // Check total claimed on stage 1
        let env = mock_env();
        assert_eq!(
            from_binary::<TotalClaimedResponse>(
                &query(deps.as_ref(), env, QueryMsg::TotalClaimed { stage: 1 }).unwrap()
            )
            .unwrap()
            .total_claimed,
            test_data.total_claimed_amount
        );
    }

    #[test]
    fn multiple_claim_native() {
        // Run test 1
        let mut deps = mock_dependencies_with_balance(&[Coin {
            denom: "ujunox".to_string(),
            amount: Uint128::new(1234567),
        }]);
        let test_data: MultipleData = from_slice(TEST_DATA_1_MULTI).unwrap();

        let msg = InstantiateMsg {
            owner: Some("owner0000".to_string()),
            cw20_token_address: None,
            native_token: Some("ujunox".to_string()),
        };

        let env = mock_env();
        let info = mock_info("addr0000", &[]);
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        let env = mock_env();
        let info = mock_info("owner0000", &[]);
        let msg = ExecuteMsg::RegisterMerkleRoot {
            merkle_root: test_data.root,
            expiration: None,
            start: None,
            total_amount: None,
            hrp: None,
        };
        let _res = execute(deps.as_mut(), env, info, msg).unwrap();

        // Loop accounts and claim
        for account in test_data.accounts.iter() {
            let msg = ExecuteMsg::Claim {
                amount: account.amount,
                stage: 1u8,
                proof: account.proofs.clone(),
                sig_info: None,
            };

            let env = mock_env();
            let info = mock_info(account.account.as_str(), &[]);
            let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
            let expected = SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: account.account.clone(),
                amount: vec![Coin {
                    denom: "ujunox".to_string(),
                    amount: account.amount,
                }],
            }));
            assert_eq!(res.messages, vec![expected]);

            assert_eq!(
                res.attributes,
                vec![
                    attr("action", "claim"),
                    attr("stage", "1"),
                    attr("address", account.account.clone()),
                    attr("amount", account.amount),
                ]
            );
        }

        // Check total claimed on stage 1
        let env = mock_env();
        assert_eq!(
            from_binary::<TotalClaimedResponse>(
                &query(deps.as_ref(), env, QueryMsg::TotalClaimed { stage: 1 }).unwrap()
            )
            .unwrap()
            .total_claimed,
            test_data.total_claimed_amount
        );
    }

    // Check expiration. Chain height in tests is 12345
    #[test]
    fn stage_expires() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            owner: Some("owner0000".to_string()),
            cw20_token_address: Some("token0000".to_string()),
            native_token: None,
        };

        let env = mock_env();
        let info = mock_info("addr0000", &[]);
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        // can register merkle root
        let env = mock_env();
        let info = mock_info("owner0000", &[]);
        let msg = ExecuteMsg::RegisterMerkleRoot {
            merkle_root: "5d4f48f147cb6cb742b376dce5626b2a036f69faec10cd73631c791780e150fc"
                .to_string(),
            expiration: Some(Expiration::AtHeight(100)),
            start: None,
            total_amount: None,
            hrp: None,
        };
        execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        // can't claim expired
        let msg = ExecuteMsg::Claim {
            amount: Uint128::new(5),
            stage: 1u8,
            proof: vec![],
            sig_info: None,
        };

        let res = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(
            res,
            ContractError::StageExpired {
                stage: 1,
                expiration: Expiration::AtHeight(100),
            }
        )
    }

    #[test]
    fn cant_burn() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            owner: Some("owner0000".to_string()),
            cw20_token_address: Some("token0000".to_string()),
            native_token: None,
        };

        let env = mock_env();
        let info = mock_info("addr0000", &[]);
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        // can register merkle root
        let env = mock_env();
        let info = mock_info("owner0000", &[]);
        let msg = ExecuteMsg::RegisterMerkleRoot {
            merkle_root: "5d4f48f147cb6cb742b376dce5626b2a036f69faec10cd73631c791780e150fc"
                .to_string(),
            expiration: Some(Expiration::AtHeight(12346)),
            start: None,
            total_amount: Some(Uint128::new(100000)),
            hrp: None,
        };
        execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        // Can't burn not expired stage
        let msg = ExecuteMsg::Burn { stage: 1u8 };

        let res = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(
            res,
            ContractError::StageNotExpired {
                stage: 1,
                expiration: Expiration::AtHeight(12346),
            }
        )
    }

    #[test]
    fn can_burn_cw20() {
        let mut deps = mock_dependencies();
        let test_data: Encoded = from_slice(TEST_DATA_1).unwrap();

        let msg = InstantiateMsg {
            owner: Some("owner0000".to_string()),
            cw20_token_address: Some("token0000".to_string()),
            native_token: None,
        };

        let mut env = mock_env();
        let info = mock_info("addr0000", &[]);
        let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        let info = mock_info("owner0000", &[]);
        let msg = ExecuteMsg::RegisterMerkleRoot {
            merkle_root: test_data.root,
            expiration: Some(Expiration::AtHeight(12500)),
            start: None,
            total_amount: Some(Uint128::new(10000)),
            hrp: None,
        };
        execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        // Claim some tokens
        let msg = ExecuteMsg::Claim {
            amount: test_data.amount,
            stage: 1u8,
            proof: test_data.proofs,
            sig_info: None,
        };

        let info = mock_info(test_data.account.as_str(), &[]);
        let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
        let expected = SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "token0000".to_string(),
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: test_data.account.clone(),
                amount: test_data.amount,
            })
            .unwrap(),
        }));
        assert_eq!(res.messages, vec![expected]);

        assert_eq!(
            res.attributes,
            vec![
                attr("action", "claim"),
                attr("stage", "1"),
                attr("address", test_data.account.clone()),
                attr("amount", test_data.amount),
            ]
        );

        // makes the stage expire
        env.block.height = 12501;

        // Can burn after expired stage
        let msg = ExecuteMsg::Burn { stage: 1u8 };

        let info = mock_info("owner0000", &[]);
        let res = execute(deps.as_mut(), env, info, msg).unwrap();

        let expected = SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "token0000".to_string(),
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::Burn {
                amount: Uint128::new(9900),
            })
            .unwrap(),
        }));
        assert_eq!(res.messages, vec![expected]);

        assert_eq!(
            res.attributes,
            vec![
                attr("action", "burn"),
                attr("stage", "1"),
                attr("address", "owner0000"),
                attr("amount", Uint128::new(9900)),
            ]
        );
    }

    #[test]
    fn can_burn_native() {
        let mut deps = mock_dependencies_with_balance(&[Coin {
            denom: "ujunox".to_string(),
            amount: Uint128::new(10000),
        }]);

        let test_data: Encoded = from_slice(TEST_DATA_1).unwrap();

        let msg = InstantiateMsg {
            owner: Some("owner0000".to_string()),
            cw20_token_address: None,
            native_token: Some("ujunox".to_string()),
        };

        let mut env = mock_env();
        let info = mock_info("addr0000", &[]);
        let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        let info = mock_info("owner0000", &[]);
        let msg = ExecuteMsg::RegisterMerkleRoot {
            merkle_root: test_data.root,
            expiration: Some(Expiration::AtHeight(12500)),
            start: None,
            total_amount: Some(Uint128::new(10000)),
            hrp: None,
        };
        execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        // Claim some tokens
        let msg = ExecuteMsg::Claim {
            amount: test_data.amount,
            stage: 1u8,
            proof: test_data.proofs,
            sig_info: None,
        };

        let info = mock_info(test_data.account.as_str(), &[]);
        let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
        let expected = SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: test_data.account.clone(),
            amount: vec![Coin {
                denom: "ujunox".to_string(),
                amount: test_data.amount,
            }],
        }));
        assert_eq!(res.messages, vec![expected]);

        assert_eq!(
            res.attributes,
            vec![
                attr("action", "claim"),
                attr("stage", "1"),
                attr("address", test_data.account.clone()),
                attr("amount", test_data.amount),
            ]
        );

        // makes the stage expire
        env.block.height = 12501;

        // Can burn after expired stage
        let msg = ExecuteMsg::Burn { stage: 1u8 };

        let info = mock_info("owner0000", &[]);
        let res = execute(deps.as_mut(), env, info, msg).unwrap();

        let expected = SubMsg::new(CosmosMsg::Bank(BankMsg::Burn {
            amount: vec![Coin {
                denom: "ujunox".to_string(),
                amount: Uint128::new(9900),
            }],
        }));
        assert_eq!(res.messages, vec![expected]);

        assert_eq!(
            res.attributes,
            vec![
                attr("action", "burn"),
                attr("stage", "1"),
                attr("address", "owner0000"),
                attr("amount", Uint128::new(9900)),
            ]
        );
    }

    #[test]
    fn cant_withdraw() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            owner: Some("owner0000".to_string()),
            cw20_token_address: Some("token0000".to_string()),
            native_token: None,
        };

        let env = mock_env();
        let info = mock_info("addr0000", &[]);
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        // can register merkle root
        let env = mock_env();
        let info = mock_info("owner0000", &[]);
        let msg = ExecuteMsg::RegisterMerkleRoot {
            merkle_root: "5d4f48f147cb6cb742b376dce5626b2a036f69faec10cd73631c791780e150fc"
                .to_string(),
            expiration: Some(Expiration::AtHeight(12346)),
            start: None,
            total_amount: Some(Uint128::new(100000)),
            hrp: None,
        };
        execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        // Can't withdraw not expired stage
        let msg = ExecuteMsg::Withdraw {
            stage: 1u8,
            address: "addr0005".to_string(),
        };

        let res = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(
            res,
            ContractError::StageNotExpired {
                stage: 1,
                expiration: Expiration::AtHeight(12346),
            }
        )
    }

    #[test]
    fn can_withdraw_cw20() {
        let mut deps = mock_dependencies();
        let test_data: Encoded = from_slice(TEST_DATA_1).unwrap();

        let msg = InstantiateMsg {
            owner: Some("owner0000".to_string()),
            cw20_token_address: Some("token0000".to_string()),
            native_token: None,
        };

        let mut env = mock_env();
        let info = mock_info("addr0000", &[]);
        let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        let info = mock_info("owner0000", &[]);
        let msg = ExecuteMsg::RegisterMerkleRoot {
            merkle_root: test_data.root,
            expiration: Some(Expiration::AtHeight(12500)),
            start: None,
            total_amount: Some(Uint128::new(10000)),
            hrp: None,
        };
        execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        // Claim some tokens
        let msg = ExecuteMsg::Claim {
            amount: test_data.amount,
            stage: 1u8,
            proof: test_data.proofs,
            sig_info: None,
        };

        let info = mock_info(test_data.account.as_str(), &[]);
        let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
        let expected = SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "token0000".to_string(),
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                recipient: test_data.account.clone(),
                amount: test_data.amount,
            })
            .unwrap(),
        }));
        assert_eq!(res.messages, vec![expected]);

        assert_eq!(
            res.attributes,
            vec![
                attr("action", "claim"),
                attr("stage", "1"),
                attr("address", test_data.account.clone()),
                attr("amount", test_data.amount),
            ]
        );

        // makes the stage expire
        env.block.height = 12501;

        // Can withdraw after expired stage
        let msg = ExecuteMsg::Withdraw {
            stage: 1u8,
            address: "addr0005".to_string(),
        };

        let info = mock_info("owner0000", &[]);
        let res = execute(deps.as_mut(), env, info, msg).unwrap();

        let expected = SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: "token0000".to_string(),
            funds: vec![],
            msg: to_binary(&Cw20ExecuteMsg::Transfer {
                amount: Uint128::new(9900),
                recipient: "addr0005".to_string(),
            })
            .unwrap(),
        }));
        assert_eq!(res.messages, vec![expected]);

        assert_eq!(
            res.attributes,
            vec![
                attr("action", "withdraw"),
                attr("stage", "1"),
                attr("address", "owner0000"),
                attr("amount", Uint128::new(9900)),
                attr("recipient", "addr0005"),
            ]
        );
    }

    #[test]
    fn can_withdraw_native() {
        let mut deps = mock_dependencies_with_balance(&[Coin {
            denom: "ujunox".to_string(),
            amount: Uint128::new(10000),
        }]);
        let test_data: Encoded = from_slice(TEST_DATA_1).unwrap();

        let msg = InstantiateMsg {
            owner: Some("owner0000".to_string()),
            cw20_token_address: None,
            native_token: Some("ujunox".to_string()),
        };

        let mut env = mock_env();
        let info = mock_info("addr0000", &[]);
        let _res = instantiate(deps.as_mut(), env.clone(), info, msg).unwrap();

        let info = mock_info("owner0000", &[]);
        let msg = ExecuteMsg::RegisterMerkleRoot {
            merkle_root: test_data.root,
            expiration: Some(Expiration::AtHeight(12500)),
            start: None,
            total_amount: Some(Uint128::new(10000)),
            hrp: None,
        };
        execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        // Claim some tokens
        let msg = ExecuteMsg::Claim {
            amount: test_data.amount,
            stage: 1u8,
            proof: test_data.proofs,
            sig_info: None,
        };

        let info = mock_info(test_data.account.as_str(), &[]);
        let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap();
        let expected = SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: test_data.account.clone(),
            amount: vec![Coin {
                denom: "ujunox".to_string(),
                amount: test_data.amount,
            }],
        }));
        assert_eq!(res.messages, vec![expected]);

        assert_eq!(
            res.attributes,
            vec![
                attr("action", "claim"),
                attr("stage", "1"),
                attr("address", test_data.account.clone()),
                attr("amount", test_data.amount),
            ]
        );

        // makes the stage expire
        env.block.height = 12501;

        // Can withdraw after expired stage
        let msg = ExecuteMsg::Withdraw {
            stage: 1u8,
            address: "addr0005".to_string(),
        };

        let info = mock_info("owner0000", &[]);
        let res = execute(deps.as_mut(), env, info, msg).unwrap();

        let expected = SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: "addr0005".to_string(),
            amount: vec![Coin {
                denom: "ujunox".to_string(),
                amount: Uint128::new(9900),
            }],
        }));
        assert_eq!(res.messages, vec![expected]);

        assert_eq!(
            res.attributes,
            vec![
                attr("action", "withdraw"),
                attr("stage", "1"),
                attr("address", "owner0000"),
                attr("amount", Uint128::new(9900)),
                attr("recipient", "addr0005"),
            ]
        );
    }

    #[test]
    fn stage_starts() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            owner: Some("owner0000".to_string()),
            cw20_token_address: Some("token0000".to_string()),
            native_token: None,
        };

        let env = mock_env();
        let info = mock_info("addr0000", &[]);
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        // can register merkle root
        let env = mock_env();
        let info = mock_info("owner0000", &[]);
        let msg = ExecuteMsg::RegisterMerkleRoot {
            merkle_root: "5d4f48f147cb6cb742b376dce5626b2a036f69faec10cd73631c791780e150fc"
                .to_string(),
            expiration: None,
            start: Some(Scheduled::AtHeight(200_000)),
            total_amount: None,
            hrp: None,
        };
        execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        // can't claim stage has not started yet
        let msg = ExecuteMsg::Claim {
            amount: Uint128::new(5),
            stage: 1u8,
            proof: vec![],
            sig_info: None,
        };

        let res = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(
            res,
            ContractError::StageNotBegun {
                stage: 1,
                start: Scheduled::AtHeight(200_000),
            }
        )
    }

    #[test]
    fn owner_freeze() {
        let mut deps = mock_dependencies();

        let msg = InstantiateMsg {
            owner: Some("owner0000".to_string()),
            cw20_token_address: Some("token0000".to_string()),
            native_token: None,
        };

        let env = mock_env();
        let info = mock_info("addr0000", &[]);
        let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

        // can register merkle root
        let env = mock_env();
        let info = mock_info("owner0000", &[]);
        let msg = ExecuteMsg::RegisterMerkleRoot {
            merkle_root: "5d4f48f147cb6cb742b376dce5626b2a036f69faec10cd73631c791780e150fc"
                .to_string(),
            expiration: None,
            start: None,
            total_amount: None,
            hrp: None,
        };
        let _res = execute(deps.as_mut(), env, info, msg).unwrap();

        // can update owner
        let env = mock_env();
        let info = mock_info("owner0000", &[]);
        let msg = ExecuteMsg::UpdateConfig {
            new_owner: Some("owner0001".to_string()),
            new_cw20_address: Some("cw20_0001".to_string()),
            new_native_token: None,
        };

        let res = execute(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // freeze contract
        let env = mock_env();
        let info = mock_info("owner0001", &[]);
        let msg = ExecuteMsg::UpdateConfig {
            new_owner: None,
            new_cw20_address: Some("cw20_0001".to_string()),
            new_native_token: None,
        };

        let res = execute(deps.as_mut(), env, info, msg).unwrap();
        assert_eq!(0, res.messages.len());

        // cannot register new drop
        let env = mock_env();
        let info = mock_info("owner0001", &[]);
        let msg = ExecuteMsg::RegisterMerkleRoot {
            merkle_root: "ebaa83c7eaf7467c378d2f37b5e46752d904d2d17acd380b24b02e3b398b3e5a"
                .to_string(),
            expiration: None,
            start: None,
            total_amount: None,
            hrp: None,
        };
        let res = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(res, ContractError::Unauthorized {});

        // cannot update config
        let env = mock_env();
        let info = mock_info("owner0001", &[]);
        let msg = ExecuteMsg::UpdateConfig {
            new_owner: Some("owner0001".to_string()),
            new_cw20_address: Some("cw20_0001".to_string()),
            new_native_token: None,
        };
        let res = execute(deps.as_mut(), env, info, msg).unwrap_err();
        assert_eq!(res, ContractError::Unauthorized {});
    }

    mod external_sig {
        use super::*;
        use crate::msg::SignatureInfo;

        const TEST_DATA_EXTERNAL_SIG: &[u8] =
            include_bytes!("../testdata/airdrop_external_sig_test_data.json");

        #[test]
        fn test_cosmos_sig_verify() {
            let deps = mock_dependencies();
            let signature_raw = Binary::from_base64("eyJwdWJfa2V5IjoiQWhOZ2UxV01aVXl1ODZ5VGx5ZWpEdVVxUFZTdURONUJhQzArdkw4b3RkSnYiLCJzaWduYXR1cmUiOiJQY1FPczhXSDVPMndXL3Z3ZzZBTElqaW9VNGorMUZYNTZKU1R1MzdIb2lGbThJck5aem5HaGlIRFV1R1VTUmlhVnZRZ2s4Q0tURmNyeVpuYjZLNVhyQT09In0=");

            let sig = SignatureInfo {
                claim_msg: Binary::from_base64("eyJhY2NvdW50X251bWJlciI6IjExMjM2IiwiY2hhaW5faWQiOiJwaXNjby0xIiwiZmVlIjp7ImFtb3VudCI6W3siYW1vdW50IjoiMTU4MTIiLCJkZW5vbSI6InVsdW5hIn1dLCJnYXMiOiIxMDU0MDcifSwibWVtbyI6Imp1bm8xMHMydXU5MjY0ZWhscWw1ZnB5cmg5dW5kbmw1bmxhdzYzdGQwaGgiLCJtc2dzIjpbeyJ0eXBlIjoiY29zbW9zLXNkay9Nc2dTZW5kIiwidmFsdWUiOnsiYW1vdW50IjpbeyJhbW91bnQiOiIxIiwiZGVub20iOiJ1bHVuYSJ9XSwiZnJvbV9hZGRyZXNzIjoidGVycmExZmV6NTlzdjh1cjk3MzRmZnJwdndwY2phZHg3bjB4Nno2eHdwN3oiLCJ0b19hZGRyZXNzIjoidGVycmExZmV6NTlzdjh1cjk3MzRmZnJwdndwY2phZHg3bjB4Nno2eHdwN3oifX1dLCJzZXF1ZW5jZSI6IjAifQ==").unwrap(),
                signature: signature_raw.unwrap(),
            };
            let cosmos_signature: CosmosSignature = from_binary(&sig.signature).unwrap();
            let res = cosmos_signature
                .verify(deps.as_ref(), &sig.claim_msg)
                .unwrap();
            assert!(res);
        }

        #[test]
        fn test_derive_addr_from_pubkey() {
            let test_data: Encoded = from_slice(TEST_DATA_EXTERNAL_SIG).unwrap();
            let cosmos_signature: CosmosSignature =
                from_binary(&test_data.signed_msg.unwrap().signature).unwrap();
            let derived_addr = cosmos_signature
                .derive_addr_from_pubkey(&test_data.hrp.unwrap())
                .unwrap();
            assert_eq!(test_data.account, derived_addr);
        }

        #[test]
        fn claim_with_external_sigs() {
            let mut deps = mock_dependencies_with_balance(&[Coin {
                denom: "ujunox".to_string(),
                amount: Uint128::new(1234567),
            }]);
            let test_data: Encoded = from_slice(TEST_DATA_EXTERNAL_SIG).unwrap();
            let claim_addr = test_data
                .signed_msg
                .clone()
                .unwrap()
                .extract_addr()
                .unwrap();

            let msg = InstantiateMsg {
                owner: Some("owner0000".to_string()),
                cw20_token_address: None,
                native_token: Some("ujunox".to_string()),
            };

            let env = mock_env();
            let info = mock_info("addr0000", &[]);
            let _res = instantiate(deps.as_mut(), env, info, msg).unwrap();

            let env = mock_env();
            let info = mock_info("owner0000", &[]);
            let msg = ExecuteMsg::RegisterMerkleRoot {
                merkle_root: test_data.root,
                expiration: None,
                start: None,
                total_amount: None,
                hrp: Some(test_data.hrp.unwrap()),
            };
            let _res = execute(deps.as_mut(), env, info, msg).unwrap();

            // cant claim without sig, info.sender is not present in the root
            let msg = ExecuteMsg::Claim {
                amount: test_data.amount,
                stage: 1u8,
                proof: test_data.proofs.clone(),
                sig_info: None,
            };

            let env = mock_env();
            let info = mock_info(claim_addr.as_str(), &[]);
            let res = execute(deps.as_mut(), env, info, msg).unwrap_err();
            assert_eq!(res, ContractError::VerificationFailed {});

            // stage account map is not saved

            // can claim with sig
            let msg = ExecuteMsg::Claim {
                amount: test_data.amount,
                stage: 1u8,
                proof: test_data.proofs,
                sig_info: test_data.signed_msg,
            };

            let env = mock_env();
            let info = mock_info(claim_addr.as_str(), &[]);
            let res = execute(deps.as_mut(), env.clone(), info.clone(), msg.clone()).unwrap();
            let expected = SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
                to_address: claim_addr.clone(),
                amount: vec![Coin {
                    denom: "ujunox".to_string(),
                    amount: test_data.amount,
                }],
            }));

            assert_eq!(res.messages, vec![expected]);
            assert_eq!(
                res.attributes,
                vec![
                    attr("action", "claim"),
                    attr("stage", "1"),
                    attr("address", claim_addr.clone()),
                    attr("amount", test_data.amount),
                ]
            );

            // Check total claimed on stage 1
            assert_eq!(
                from_binary::<TotalClaimedResponse>(
                    &query(
                        deps.as_ref(),
                        env.clone(),
                        QueryMsg::TotalClaimed { stage: 1 },
                    )
                    .unwrap()
                )
                .unwrap()
                .total_claimed,
                test_data.amount
            );

            // Check address is claimed
            assert!(
                from_binary::<IsClaimedResponse>(
                    &query(
                        deps.as_ref(),
                        env.clone(),
                        QueryMsg::IsClaimed {
                            stage: 1,
                            address: test_data.account.clone(),
                        },
                    )
                    .unwrap()
                )
                .unwrap()
                .is_claimed
            );

            // check error on double claim
            let res = execute(deps.as_mut(), env.clone(), info, msg).unwrap_err();
            assert_eq!(res, ContractError::Claimed {});

            // query map

            let map = from_binary::<AccountMapResponse>(
                &query(
                    deps.as_ref(),
                    env,
                    QueryMsg::AccountMap {
                        stage: 1,
                        external_address: test_data.account.clone(),
                    },
                )
                .unwrap(),
            )
            .unwrap();
            assert_eq!(map.external_address, test_data.account);
            assert_eq!(map.host_address, claim_addr);
        }
    }
}
