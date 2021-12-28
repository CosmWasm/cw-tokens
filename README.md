# CosmWasm Tokens

[![CircleCI](https://circleci.com/gh/CosmWasm/cw-plus/tree/master.svg?style=shield)](https://circleci.com/gh/CosmWasm/cw-plus/tree/master)

This is a collection of [cw20-related](https://github.com/CosmWasm/cw-plus/blob/main/packages/cw20/README.md) contracts
extracted from [cw-plus](https://github.com/CosmWasm/cw-plus). These serve as examples of what is possible to build
and as starting points for your own CosmWasm token contracts.

None of these have been audited or are considered ready-for-production as is. Contributions may come from many
community members. Please do your own due dilligence on them before using on any production site, and please
[raise Github issues](https://github.com/CosmWasm/cw-tokens/issues) for any bugs you find.

You are more than welcome to [create a PR](https://github.com/CosmWasm/cw-tokens/pulls) to add any cw20-related
contract you have written that you would like to share with the community.


| Contracts               | Download                                                                                                                      | Docs                                                                     |
| ----------------------- | ----------------------------------------------------------------------------------------------------------------------------  | -------------------------------------------------------------------------|
| cw20-atomic-swap        | [Release v0.11.2](https://github.com/CosmWasm/cw-tokens/releases/download/v0.11.2/cw20_atomic_swap.wasm)          | [![Docs](https://docs.rs/cw20-atomic-swap/badge.svg)](https://docs.rs/cw20-atomic-swap)    |
| cw20-bonding            | [Release v0.11.2](https://github.com/CosmWasm/cw-tokens/releases/download/v0.11.2/cw20_bonding.wasm)          | [![Docs](https://docs.rs/cw20-bonding/badge.svg)](https://docs.rs/cw20-bonding)    |
| cw20-escrow             | [Release v0.11.2](https://github.com/CosmWasm/cw-tokens/releases/download/v0.11.2/cw20_escrow.wasm)          | [![Docs](https://docs.rs/cw20-escrow/badge.svg)](https://docs.rs/cw20-escrow)    |
| cw20-staking            | [Release v0.11.2](https://github.com/CosmWasm/cw-tokens/releases/download/v0.11.2/cw20_staking.wasm)          | [![Docs](https://docs.rs/cw20-staking/badge.svg)](https://docs.rs/cw20-staking)    |
| cw20-merkle-airdrop     | [Release v0.11.2](https://github.com/CosmWasm/cw-tokens/releases/download/v0.11.2/cw20_merkle_airdrop.wasm)          | [![Docs](https://docs.rs/cw20-merkle-airdrop/badge.svg)](https://docs.rs/cw20-merkle-airdrop)    |

**Warning** None of these contracts have been audited and no liability is
assumed for the use of this code. They are provided to turbo-start
your projects.


## Contracts

All contracts add functionality around the CW20 Fungible Token standard:

* [`cw20-atomic-swap`](./contracts/cw20-atomic-swap) an implementation of atomic swaps for
both native and cw20 tokens.
* [`cw20-bonding`](./contracts/cw20-bonding) a smart contract implementing arbitrary bonding curves,
which can use native and cw20 tokens as reserve tokens.
* [`cw20-staking`](./contracts/cw20-staking) provides staking derivatives,
staking native tokens on your behalf and minting cw20 tokens that can
be used to claim them. It uses `cw20-base` for all the cw20 logic and
only implements the interactions with the staking module and accounting
for prices.
* [`cw20-escrow`](./contracts/cw20-escrow) is a basic escrow contract
(arbiter can release or refund tokens) that is compatible with all native
and cw20 tokens. This is a good example to show how to interact with
cw20 tokens.
* [`cw20-merkle-airdrop`](./contracts/cw20-merkle-airdrop) is a contract
  for efficient cw20 token airdrop distribution.

## Compiling

To compile all the contracts, run the following in the repo root:

```
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/workspace-optimizer:0.12.4
```

This will compile all packages in the `contracts` directory and output the
stripped and optimized wasm code under the `artifacts` directory as output,
along with a `checksums.txt` file.

If you hit any issues there and want to debug, you can try to run the
following in each contract dir:
`RUSTFLAGS="-C link-arg=-s" cargo build --release --target=wasm32-unknown-unknown --locked`

## Licenses

All code in this repo will always be licensed under [Apache-2.0](./LICENSE).
