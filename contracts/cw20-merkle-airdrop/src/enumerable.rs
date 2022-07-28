use crate::msg::{AccountMapResponse, AllAccountMapResponse};
use crate::state::STAGE_ACCOUNT_MAP;
use cosmwasm_std::{Deps, Order, StdResult};
use cw_storage_plus::Bound;

// settings for pagination
const MAX_LIMIT: u32 = 1000;
const DEFAULT_LIMIT: u32 = 10;

pub fn query_all_address_map(
    deps: Deps,
    stage: u8,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<AllAccountMapResponse> {
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;
    let start = start_after.map(Bound::exclusive);

    let address_maps = STAGE_ACCOUNT_MAP
        .prefix(stage)
        .range(deps.storage, start, None, Order::Ascending)
        .take(limit)
        .map(|p| {
            p.map(|(external_address, host_address)| AccountMapResponse {
                host_address,
                external_address,
            })
        })
        .collect::<StdResult<_>>()?;

    let resp = AllAccountMapResponse { address_maps };
    Ok(resp)
}
