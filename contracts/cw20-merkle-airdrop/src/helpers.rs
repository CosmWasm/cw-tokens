use crate::ContractError;
use bech32::ToBase32;
use cosmwasm_std::{from_slice, Binary, Deps};
use ripemd::{Digest as RipDigest, Ripemd160};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest as ShaDigest, Sha256};
use std::convert::TryInto;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CosmosSignature {
    pub pub_key: Binary,
    pub signature: Binary,
}
impl CosmosSignature {
    pub fn verify(&self, deps: Deps, claim_msg: Binary) -> Result<bool, ContractError> {
        let hash = Sha256::digest(&claim_msg);

        deps.api
            .secp256k1_verify(
                hash.as_ref(),
                self.signature.as_slice(),
                self.pub_key.as_slice(),
            )
            .map_err(|_| ContractError::VerificationFailed {})
    }

    pub fn derive_addr_from_pubkey(&self, hrp: &str) -> Result<String, ContractError> {
        // derive external address for merkle proof check
        let sha_hash: [u8; 32] = Sha256::digest(self.pub_key.as_slice())
            .as_slice()
            .try_into()
            .map_err(|_| ContractError::WrongLength {})?;

        let rip_hash = Ripemd160::digest(sha_hash);
        let rip_slice: &[u8] = rip_hash.as_slice();

        let addr: String = bech32::encode(hrp, rip_slice.to_base32(), bech32::Variant::Bech32)
            .map_err(|_| ContractError::VerificationFailed {})?;
        Ok(addr)
    }
}
// Signature verification is done on external airdrop claims.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SignatureInfo {
    pub claim_msg: Binary,
    pub signature: Binary,
}
impl SignatureInfo {
    pub fn extract_addr_from_memo(&self) -> Result<String, ContractError> {
        let claim_msg = from_slice::<ClaimMsg>(&self.claim_msg).unwrap();
        Ok(claim_msg.address)
    }
}
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ClaimMsg {
    #[serde(rename = "memo")]
    address: String,
}
