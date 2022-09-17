use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::Addr;
use cw_storage_plus::Item;

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct BlockList {
    pub list: Vec<Addr>,
}
impl BlockList {
    pub fn default() -> Self {
        BlockList { list: vec![] }
    }

    /// returns true if the given address is on a blocklist
    pub fn is_blocked(&self, addr: impl AsRef<str>) -> bool {
        let addr = addr.as_ref();
        self.list.iter().any(|a| a.as_ref() == addr)
    }

    // update a blocklist addresses
    pub fn update(&mut self, add: Vec<Addr>, remove: Vec<Addr>) {
        // dedup add list passed in
        let mut add_set: Vec<Addr> = add.into_iter().collect();
        add_set.sort();
        add_set.dedup();
        // add new addresses
        for addr in add_set.into_iter() {
            self.list.push(addr);
        }

        // dedup remove list passed in
        let mut remove_set: Vec<Addr> = remove.into_iter().collect();
        remove_set.sort();
        remove_set.dedup();
        // remove existing addresses
        for addr in remove_set.into_iter() {
            self.list.retain(|item| item != &addr);
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, JsonSchema, Debug)]
#[serde(rename_all = "snake_case")]
pub struct Config {
    pub admin: Addr,
    pub asset: Addr,
    pub withdraw_allowed: bool,
    pub withdraw_blocklist: BlockList,
    pub deposit_allowed: bool,
    pub deposit_blocklist: BlockList,
}

impl Config {
    pub fn default(admin: Addr, asset: Addr) -> Self {
        Config {
            admin,
            asset,
            withdraw_allowed: true,
            withdraw_blocklist: BlockList::default(),
            deposit_allowed: true,
            deposit_blocklist: BlockList::default(),
        }
    }
}

pub const CONFIG: Item<Config> = Item::new("config");
