use cosmwasm_schema::write_api;
use cw20_atomic_swap::msg::ExecuteMsg;
use cw20_atomic_swap::msg::InstantiateMsg;
use cw20_atomic_swap::msg::QueryMsg;

fn main() {
    write_api! {
        instantiate: InstantiateMsg,
        query: QueryMsg,
        execute: ExecuteMsg,
    }
}
