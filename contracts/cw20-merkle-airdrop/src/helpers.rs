use crate::msg::{SignatureInfo, SignedClaimMsg};
use crate::ContractError;
use bech32::ToBase32;
use cosmwasm_std::{from_binary, to_vec, Binary, Deps, DepsMut, MessageInfo};
use ripemd::{Digest as RipDigest, Ripemd160};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest as ShaDigest, Sha256};
use std::convert::TryInto;

pub fn verify_cosmos(
    deps: Deps,
    claim_msg: &SignedClaimMsg,
    sig: &CosmosSignature,
) -> Result<bool, ContractError> {
    let msg_raw = to_vec(claim_msg)?;
    let hash = Sha256::digest(&msg_raw);

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
    pub pub_key: Binary,
    pub signature: Binary,
}

pub fn verify_external_address(
    deps: &DepsMut,
    info: &MessageInfo,
    hrp: String,
    sig: &SignatureInfo,
) -> Result<String, ContractError> {
    // check if signature is correct
    let signature: CosmosSignature = from_binary(&sig.signature).unwrap();
    verify_cosmos(deps.as_ref(), &sig.claim_msg, &signature)?;
    // check claiming address is in signed msg
    if sig.claim_msg.addr != info.sender {
        return Err(ContractError::VerificationFailed {});
    }

    // derive external address for merkle proof check
    let sha_hash: [u8; 32] = Sha256::digest(signature.pub_key.as_slice())
        .as_slice()
        .try_into()
        .map_err(|_| ContractError::WrongLength {})?;

    let addr_hash_raw = Ripemd160::digest(sha_hash);
    let addr_hash: &[u8] = addr_hash_raw.as_slice();

    let addr: String = bech32::encode(hrp.as_str(), addr_hash.to_base32(), bech32::Variant::Bech32)
        .map_err(|_| ContractError::VerificationFailed {})?;
    Ok(addr)
}
