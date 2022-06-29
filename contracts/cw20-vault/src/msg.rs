use cosmwasm_std::{Addr, Binary, StdError, StdResult, Uint128};
use cw20::{Cw20ReceiveMsg, Logo};
use cw_utils::Expiration;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::utils::{is_valid_name, is_valid_symbol};

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct InstantiateMarketingInfo {
    pub project: Option<String>,
    pub description: Option<String>,
    pub marketing: Option<String>,
    pub logo: Option<Logo>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct InstantiateMsg {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub marketing: Option<InstantiateMarketingInfo>,
    pub asset: String,
    pub cap: Option<Uint128>,
}

impl InstantiateMsg {
    pub fn validate(&self) -> StdResult<()> {
        // Check name, symbol, decimals
        if !is_valid_name(&self.name) {
            return Err(StdError::generic_err(
                "Name is not in the expected format (3-50 UTF-8 bytes)",
            ));
        }
        if !is_valid_symbol(&self.symbol) {
            return Err(StdError::generic_err(
                "Ticker symbol is not in expected format [a-zA-Z\\-]{3,12}",
            ));
        }
        if self.decimals > 18 {
            return Err(StdError::generic_err("Decimals must not exceed 18"));
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg),
    /// Updates admin address
    UpdateAdmin {
        new_admin: String,
    },
    /// Updates config's global block states
    UpdateGlobalBlock {
        deposit_allowed: bool,
        withdraw_allowed: bool,
    },
    /// Updates specific address blocklist, adding and/or removing addressed given.
    /// list_type arg can be either: "withdraw" or "deposit" to update that list
    /// NOTE: If an address is on both the add & remove lists passed, it will be removed from the blocklist.
    UpdateBlockList {
        add: Vec<Addr>,
        remove: Vec<Addr>,
        list_type: String,
    },
    /// Burns shares from owner and sends exactly assets of underlying tokens to receiver.
    /// MUST support a withdraw flow where the shares are burned from owner directly where owner is msg.sender or msg.sender has CW20 approval over the shares of owner.
    /// MAY support an additional flow in which the shares are transferred to the Vault contract before the withdraw execution, and are accounted for during withdraw.
    /// MUST revert if all of assets cannot be withdrawn (due to withdrawal limit being reached, slippage, the owner not having enough shares, etc).
    /// Note that some implementations will require pre-requesting to the Vault before a withdrawal may be performed. Those methods should be performed separately.
    Withdraw {
        assets: Uint128,
        owner: Option<String>,
        recipient: Option<String>,
    },
    /// Burns exactly shares from owner and sends assets of underlying tokens to receiver.
    /// MUST support a redeem flow where the shares are burned from owner directly where owner is msg.sender or msg.sender has CW20 approval over the shares of owner.
    /// MAY support an additional flow in which the shares are transferred to the Vault contract before the redeem execution, and are accounted for during redeem.
    /// MUST revert if all of shares cannot be redeemed (due to withdrawal limit being reached, slippage, the owner not having enough shares, etc).
    /// Note that some implementations will require pre-requesting to the Vault before a withdrawal may be performed. Those methods should be performed separately.
    Redeem {
        shares: Uint128,
        owner: Option<String>,
        recipient: Option<String>,
    },

    /// Implements CW20. Transfer is a base message to move tokens to another account without triggering actions
    Transfer {
        recipient: String,
        amount: Uint128,
    },
    /// Implements CW20. Send is a base message to transfer tokens to a contract and trigger an action
    /// on the receiving contract.
    Send {
        contract: String,
        amount: Uint128,
        msg: Binary,
    },
    /// Implements CW20. Only with "approval" extension. Transfers amount tokens from owner -> recipient
    /// if `info.sender` has sufficient pre-approval.
    TransferFrom {
        owner: String,
        recipient: String,
        amount: Uint128,
    },
    /// Implements CW20. Only with "approval" extension. Sends amount tokens from owner -> contract
    /// if `info.sender` has sufficient pre-approval.
    SendFrom {
        owner: String,
        contract: String,
        amount: Uint128,
        msg: Binary,
    },
    /// Implements CW20. Only with "approval" extension. Allows spender to access an additional amount tokens
    /// from the owner's (env.sender) account. If expires is Some(), overwrites current allowance
    /// expiration with this one.
    IncreaseAllowance {
        spender: String,
        amount: Uint128,
        expires: Option<Expiration>,
    },
    /// Implements CW20. Only with "approval" extension. Lowers the spender's access of tokens
    /// from the owner's (env.sender) account by amount. If expires is Some(), overwrites current
    /// allowance expiration with this one.
    DecreaseAllowance {
        spender: String,
        amount: Uint128,
        expires: Option<Expiration>,
    },
    /// Implements CW20. Only with the "marketing" extension. If authorized, updates marketing metadata.
    /// Setting None/null for any of these will leave it unchanged.
    /// Setting Some("") will clear this field on the contract storage
    UpdateMarketing {
        /// A URL pointing to the project behind this token.
        project: Option<String>,
        /// A longer description of the token and it's utility. Designed for tooltips or such
        description: Option<String>,
        /// The address (if any) who can update this data structure
        marketing: Option<String>,
    },
    /// Implements CW20. If set as the "marketing" role on the contract, upload a new URL, SVG, or PNG for the token
    UploadLogo(Logo),
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Cw20HookMsg {
    /// Mints shares Vault shares to receiver by depositing exactly amount of underlying tokens.
    /// MUST support CW20 approve / transferFrom on asset as a deposit flow.
    /// MAY support an additional flow in which the underlying tokens are owned by the Vault contract before the deposit execution, and are accounted for during deposit.
    /// MUST revert if all of assets cannot be deposited (due to deposit limit being reached, slippage, the user not approving enough underlying tokens to the Vault contract, etc).
    /// Note that most implementations will require pre-approval of the Vault with the Vault’s underlying asset token.
    Deposit { assets: Uint128, recipient: String },
    // Mints exactly shares Vault shares to receiver by depositing amount of underlying tokens.
    /// MUST support CW20 approve / transferFrom on asset as a mint flow.
    /// MAY support an additional flow in which the underlying tokens are owned by the Vault contract before the mint execution, and are accounted for during mint.
    /// MUST revert if all of shares cannot be minted (due to deposit limit being reached, slippage, the user not approving enough underlying tokens to the Vault contract, etc).
    ///  Note that most implementations will require pre-approval of the Vault with the Vault’s underlying asset token.
    Mint { shares: Uint128, recipient: String },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    /// Returns the current balance of the given address, 0 if unset.
    /// Return type: BalanceResponse.
    Balance {
        address: String,
    },
    /// Returns Config struct info
    ConfigInfo {},
    /// Returns metadata on the contract - name, decimals, supply, etc.
    /// Return type: TokenInfoResponse.
    TokenInfo {},
    /// Returns the total amount of the underlying asset that is “managed” by Vault.
    /// SHOULD include any compounding that occurs from yield.
    /// MUST be inclusive of any fees that are charged against assets in the Vault.
    TotalAssets {},
    /// The address or denom of the underlying coin used for the Vault for accounting, depositing, and withdrawing.
    Asset {},
    // The amount of assets that the Vault would exchange for the amount of shares provided, in an ideal scenario where all the conditions are met.
    // MUST NOT be inclusive of any fees that are charged against assets in the Vault.
    // MUST NOT show any variations depending on the caller.
    // MUST NOT reflect slippage or other on-chain conditions, when performing the actual exchange.
    // MUST NOT revert unless due to integer overflow caused by an unreasonably large input.
    // MUST round down towards 0.
    // MAY NOT reflect the “per-user” price-per-share, and instead should reflect the “average-user’s” price-per-share, meaning what the average user should expect to see when exchanging to and from.
    ConvertToAssets {
        shares: Uint128,
    },
    /// The amount of shares that the Vault would exchange for the amount of assets provided, in an ideal scenario where all the conditions are met.
    /// MUST NOT be inclusive of any fees that are charged against assets in the Vault.
    /// MUST NOT show any variations depending on the caller.
    /// MUST NOT reflect slippage or other on-chain conditions, when performing the actual exchange.
    /// MUST NOT revert unless due to integer overflow caused by an unreasonably large input.
    /// MUST round down towards 0.
    /// MAY NOT reflect the “per-user” price-per-share, and instead should reflect the “average-user’s” price-per-share, meaning what the average user should expect to see when exchanging to and from.
    ConvertToShares {
        assets: Uint128,
    },
    /// Allows an on-chain or off-chain user to simulate the effects of their deposit at the current block, given current on-chain conditions.
    PreviewDeposit {
        assets: Uint128,
    },
    /// Maximum amount of the underlying asset that can be deposited into the Vault for the recipient, through a deposit call.
    MaxDeposit {
        recipient: String,
    },
    /// Allows an on-chain or off-chain user to simulate the effects of their redeemption at the current block, given current on-chain conditions.
    PreviewRedeem {
        shares: Uint128,
    },
    /// Maximum amount of Vault shares that can be redeemed from the owner balance in the Vault, through a redeem call.
    MaxRedeem {
        owner: String,
    },
    /// Allows an on-chain or off-chain user to simulate the effects of their withdrawal at the current block, given current on-chain conditions.
    PreviewWithdraw {
        assets: Uint128,
    },
    /// Maximum amount of the underlying asset that can be withdrawn from the owner balance in the Vault, through a withdraw call.
    /// MUST return the maximum amount of assets that could be transferred from owner through withdraw and not cause a revert, which MUST NOT be higher than the actual maximum that would be accepted (it should underestimate if necessary).
    /// MUST factor in both global and user-specific limits, like if withdrawals are entirely disabled (even temporarily) it MUST return 0.
    MaxWithdraw {
        owner: String,
    },
    /// Only with "allowance" extension.
    /// Returns how much spender can use from owner account, 0 if unset.
    /// Return type: AllowanceResponse.
    Allowance {
        owner: String,
        spender: String,
    },
    /// Only with "enumerable" extension (and "allowances")
    /// Returns all allowances this owner has approved. Supports pagination.
    /// Return type: AllAllowancesResponse.
    AllAllowances {
        owner: String,
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Only with "enumerable" extension
    /// Returns all accounts that have balances. Supports pagination.
    /// Return type: AllAccountsResponse.
    AllAccounts {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    /// Only with "marketing" extension
    /// Returns more metadata on the contract to display in the client:
    /// - description, logo, project url, etc.
    /// Return type: MarketingInfoResponse
    MarketingInfo {},
    /// Only with "marketing" extension
    /// Downloads the embedded logo data (if stored on chain). Errors if no logo data is stored for this
    /// contract.
    /// Return type: DownloadLogoResponse.
    DownloadLogo {},
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
pub struct ConfigResponse {
    pub asset: String,
    pub withdraw_allowed: bool,
    pub withdraw_blocklist: Vec<Addr>,
    pub deposit_allowed: bool,
    pub deposit_blocklist: Vec<Addr>,
}
