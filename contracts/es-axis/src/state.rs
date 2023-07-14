use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, StdResult, Storage, Uint128};
use cw_storage_plus::Item;

#[cw_serde]
pub struct Config {
    pub staking_contarct: Addr,
    pub es_axis_total_supply: Uint128,
    pub es_axis_denom: String,
}
pub fn save_config(storage: &mut dyn Storage, config: &Config) -> StdResult<()> {
    CONFIG.save(storage, config)
}
pub fn load_config(storage: &dyn Storage) -> StdResult<Config> {
    CONFIG.load(storage)
}
pub const CONFIG: Item<Config> = Item::new("config");
