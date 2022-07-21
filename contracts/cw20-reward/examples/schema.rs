use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, remove_schemas, schema_for};

use cw20_reward::msg::{
    AccruedRewardsResponse, ExecuteMsg, HolderResponse, HoldersResponse, InstantiateMsg,
    MigrateMsg, QueryMsg, ReceiveMsg, StateResponse,
};
use cw_controllers::{Claim, ClaimsResponse};

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("../schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(ExecuteMsg), &out_dir);
    export_schema(&schema_for!(ReceiveMsg), &out_dir);
    export_schema(&schema_for!(InstantiateMsg), &out_dir);
    export_schema(&schema_for!(QueryMsg), &out_dir);
    export_schema(&schema_for!(StateResponse), &out_dir);
    export_schema(&schema_for!(AccruedRewardsResponse), &out_dir);
    export_schema(&schema_for!(HolderResponse), &out_dir);
    export_schema(&schema_for!(HoldersResponse), &out_dir);
    export_schema(&schema_for!(MigrateMsg), &out_dir);
    export_schema(&schema_for!(Claim), &out_dir);
    export_schema(&schema_for!(ClaimsResponse), &out_dir);
}
