use crate::msg::SignedClaimMsg;
use crate::ContractError;
use cosmwasm_std::{from_binary, to_vec, Binary, Deps};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub fn verify_cosmos(
    deps: Deps,
    claim_msg: &SignedClaimMsg,
    signature: &Binary,
) -> Result<bool, ContractError> {
    let msg_raw = to_vec(claim_msg)?;
    let hash = Sha256::digest(&msg_raw);
    let sig: CosmosSignature = from_binary(signature).unwrap();

    deps.api
        .secp256k1_verify(
            hash.as_ref(),
            sig.signature.as_slice(),
            sig.pub_key.as_slice(),
        )
        .map_err(|_| ContractError::VerificationFailed {})
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CosmosSignature {
    pub_key: Binary,
    signature: Binary,
}
