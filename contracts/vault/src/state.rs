use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, StdResult, Storage, Uint128};
use cw_storage_plus::{Item, Map};

#[cw_serde]
pub struct Config {
    pub owner: Addr,
    pub core_contract: Addr,
    pub epoch: u64,
    pub es_axis_contract: Addr,
    pub es_axis_denom: String,
    pub denom_list: Vec<String>,
}

pub fn save_config(storage: &mut dyn Storage, config: &Config) -> StdResult<()> {
    CONFIG.save(storage, config)
}
pub fn load_config(storage: &dyn Storage) -> StdResult<Config> {
    CONFIG.load(storage)
}

//denom balance
pub const BALANCE: Map<&str, Uint128> = Map::new("vault_balance");

pub fn save_balance(storage: &mut dyn Storage, denom: &String, balance: &Uint128) -> StdResult<()> {
    BALANCE.save(storage, denom, balance)
}
pub fn load_balance(storage: &dyn Storage, denom: &String) -> StdResult<Uint128> {
    match BALANCE.may_load(storage, denom) {
        Ok(Some(balance)) => Ok(balance),
        Ok(None) => Ok(Uint128::zero()),
        Err(e) => Err(e),
    }
}
pub const PENDING_BALANCE: Map<&str, Uint128> = Map::new("pending_balance");
pub fn save_pending_balance(
    storage: &mut dyn Storage,
    denom: &String,
    balance: &Uint128,
) -> StdResult<()> {
    PENDING_BALANCE.save(storage, denom, balance)
}
pub fn load_pending_balance(storage: &dyn Storage, denom: &String) -> StdResult<Uint128> {
    match PENDING_BALANCE.may_load(storage, denom) {
        Ok(Some(balance)) => Ok(balance),
        Ok(None) => Ok(Uint128::zero()),
        Err(e) => Err(e),
    }
}
pub const CONFIG: Item<Config> = Item::new("valut_config");
