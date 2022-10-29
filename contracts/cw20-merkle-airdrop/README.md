# CW20 Merkle Airdrop

This is a merkle airdrop smart contract that works with cw20 token specification Mass airdrop distributions made cheap
and efficient.

Explanation of merkle
airdrop: [Medium Merkle Airdrop: the Basics](https://medium.com/smartz-blog/merkle-airdrop-the-basics-9a0857fcc930)

Traditional and non-efficient airdrops:

- Distributor creates a list of airdrop
- Sends bank send messages to send tokens to recipients

**Or**

- Stores list of recipients on smart contract data
- Recipient claims the airdrop

These two solutions are very ineffective when recipient list is big. First, costly because bank send cost for the
distributor will be costly. Second, whole airdrop list stored in the state, again costly.

Merkle Airdrop is very efficient even when recipient number is massive.

This contract works with multiple airdrop rounds, meaning you can execute several airdrops using same instance.

Uses **SHA256** for merkle root tree construction.

## Procedure

- Distributor of contract prepares a list of addresses with many entries and publishes this list in public static .js
  file in JSON format
- Distributor reads this list, builds the merkle tree structure and writes down the Merkle root of it.
- Distributor creates contract and places calculated Merkle root into it.
- Distributor says to users, that they can claim their tokens, if they owe any of addresses, presented in list,
  published on distributor's site.
- User wants to claim his N tokens, he also builds Merkle tree from public list and prepares Merkle proof, consisting
  from log2N hashes, describing the way to reach Merkle root
- User sends transaction with Merkle proof to contract
- Contract checks Merkle proof, and, if proof is correct, then sender's address is in list of allowed addresses, and
  contract does some action for this use.
- Distributor sends token to the contract, and registers new merkle root for the next distribution round.

## Spec

### Messages

#### InstantiateMsg

`InstantiateMsg` instantiates contract with owner and cw20 token address. Airdrop `stage` is set to 0.

```rust
pub struct InstantiateMsg {
  pub owner: Option<String>,
  pub cw20_token_address: Option<String>,
  pub native_token: Option<String>,
}
```

#### ExecuteMsg

```rust
pub enum ExecuteMsg {
  UpdateConfig {
    new_owner: Option<String>,
    new_cw20_address: Option<String>,
    new_native_token: Option<String>,
  },
  RegisterMerkleRoot {
    merkle_root: String,
    expiration: Option<Expiration>,
    start: Option<Scheduled>,
    total_amount: Option<Uint128>,
    hrp: Option<String>,
  },
  Claim {
    stage: u8,
    amount: Uint128,
    proof: Vec<String>,
    /// Enables cross chain airdrops.
    /// Target wallet proves identity by sending a signed [SignedClaimMsg](SignedClaimMsg)
    /// containing the recipient address.
    sig_info: Option<SignatureInfo>,
  },
  Burn {
      stage: u8,
  },
  /// Withdraw the remaining tokens after expire time (only owner)
  Withdraw {
      stage: u8,
      address: String,
  },
  Pause {
      stage: u8,
  },
  Resume {
      stage: u8,
      new_expiration: Option<Expiration>,
  },
}
```

- `UpdateConfig{owner}` updates configuration.
- `RegisterMerkleRoot {merkle_root}` registers merkle tree root for further claim verification. Airdrop `Stage`
  increased by 1.
- `Claim{stage, amount, proof}` recipient executes for claiming airdrop with `stage`, `amount` and `proof` data built
  using full list.

#### QueryMsg

``` rust
pub enum QueryMsg {
    Config {},
    MerkleRoot { stage: u8 },
    LatestStage {},
    IsClaimed { stage: u8, address: String },
    TotalClaimed { stage: u8 },
    AccountMap { stage: u8, external_address: String },
    AllAccountMaps {
        stage: u8,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    IsPaused { stage: u8 },
}
```

- `{ config: {} }` returns configuration, `{"cw20_token_address": ..., "owner": ...}`.
- `{ merkle_root: { stage: "1" }` returns merkle root of given stage, `{"merkle_root": ... , "stage": ...}`
- `{ latest_stage: {}}` returns current airdrop stage, `{"latest_stage": ...}`
- `{ is_claimed: {stage: "stage", address: "wasm1..."}` returns if address claimed airdrop, `{"is_claimed": "true"}`

## Merkle Airdrop CLI

[Merkle Airdrop CLI](helpers) contains js helpers for generating root, generating and verifying proofs for given airdrop
file.

## Test Vector Generation

Test vector can be generated using commands at [Merkle Airdrop CLI README](helpers/README.md)
