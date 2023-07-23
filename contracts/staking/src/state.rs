use cosmwasm_schema::cw_serde;

use cosmwasm_std::{Addr, StdResult, Storage, Uint128};
use cw_storage_plus::{Item, Map};

#[cw_serde]
pub struct Config {
    pub core_contract: Addr,
    pub axis_denom: String,
    pub es_axis_contract: Addr,
}

pub fn save_config(storage: &mut dyn Storage, config: &Config) -> StdResult<()> {
    CONFIG.save(storage, config)
}
pub fn load_config(storage: &dyn Storage) -> StdResult<Config> {
    CONFIG.load(storage)
}
#[cw_serde]
pub struct State {
    pub pending_staking_total: Uint128,
    pub withdraw_pending_total: Uint128,
    pub staking_total: Uint128,
    pub epoch: u64,
}
pub fn save_state(storage: &mut dyn Storage, state: &State) -> StdResult<()> {
    STATE.save(storage, state)
}
pub fn load_state(storage: &dyn Storage) -> StdResult<State> {
    STATE.load(storage)
}
#[cw_serde]
pub struct StakeInfo {
    pub start_epoch: u64,
    pub staking_amount: Uint128,
}

#[cw_serde]
pub struct UnStakeInfo {
    pub unlock_epoch: u64,
    pub withdraw_pending_amount: Uint128,
}
pub const CONFIG: Item<Config> = Item::new("config");
pub const STATE: Item<State> = Item::new("state");

//epoch,total_staking
pub const EPOCH_STAKING_AMOUNT: Map<u64, Uint128> = Map::new("epoch_total_staking");

pub const STAKING: Map<Addr, Vec<StakeInfo>> = Map::new("staking");
pub const UN_STAKING: Map<Addr, Vec<UnStakeInfo>> = Map::new("unstaking");

pub fn load_stakings(storage: &dyn Storage, staker: Addr) -> StdResult<Vec<StakeInfo>> {
    let stakings = STAKING.load(storage, staker)?;
    Ok(stakings)
}

pub fn load_un_stakings(storage: &dyn Storage, staker: Addr) -> StdResult<Vec<UnStakeInfo>> {
    let stakings = UN_STAKING.load(storage, staker)?;
    Ok(stakings)
}
