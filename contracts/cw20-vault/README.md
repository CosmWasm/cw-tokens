# CW20 - Vault (based on EIP-4626)

This is a basic implementation of a CW20 Vault contract with a yield bearing token. This contract accepts
an "asset" CW20 token and issues a "share" token, representing shares of the underlying assets that a vault 
controls. The inherited CW20 operations balanceOf, transfer, totalSupply, etc. operate on the Vault “shares” 
token. These represent a claim to ownership on a fraction of the Vault’s underlying asset holdings.

This contract is designed to be deployed only after modifications to it's internal `after_deposit` and 
`before_withdraw` functions, as these serve to define and set a vault's strategy for deposited assets. 

If it is desired that a Vault should be non-transferrable, simply revert on calls to transfer or transferFrom. 

Definitions:
- asset: The underlying token managed by the Vault. Has units defined by the corresponding CW20 contract.
- share: The token of the Vault. Has a ratio of underlying assets exchanged on mint/deposit/withdraw/redeem (as defined by the Vault).
- fee: An amount of assets or shares charged to the user by the Vault. Fees can exists for deposits, yield, AUM, withdrawals, or anything else prescribed by the Vault.
- slippage: Any difference between advertised share price and economic realities of deposit to or withdrawal from the Vault, which is not accounted by fees.


Implements:

- [x] CW20 Base
- [ ] Mintable extension
- [x] Allowances extension

## Running this contract

You will need Rust 1.44.1+ with `wasm32-unknown-unknown` target installed.

You can run unit tests on this via: 

`cargo test`

Once you are happy with the content, you can compile it to wasm via:

```
RUSTFLAGS='-C link-arg=-s' cargo wasm
cp ../../target/wasm32-unknown-unknown/release/cw20_vault.wasm .
ls -l cw20_vault.wasm
sha256sum cw20_vault.wasm
```
