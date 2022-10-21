// Migration logic for contracts with version: 0.12.1
pub mod v0_12_1 {
    use crate::state::{LATEST_STAGE, STAGE_PAUSED};
    use crate::ContractError;
    use cosmwasm_std::DepsMut;
    pub fn set_initial_pause_status(deps: DepsMut) -> Result<(), ContractError> {
        let latest_stage = LATEST_STAGE.load(deps.storage)?;
        for stage in 0..=latest_stage {
            STAGE_PAUSED.save(deps.storage, stage, &false)?;
        }
        Ok(())
    }
}
