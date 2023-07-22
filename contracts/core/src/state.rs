use cosmwasm_schema::cw_serde;

use cosmwasm_std::{Addr, StdResult, Storage, Timestamp, Uint128};
use cw_storage_plus::{Item, Map};

use crate::ContractError;

#[cw_serde]
pub struct Config {
    pub owner: Addr,
    pub epoch: u64,
    pub accept_price_denoms: Vec<String>,
    pub axis_contract: Addr,
    pub staking_contract: Addr,
    pub vault_contract: Addr,
    pub next_update_timestamp: Timestamp,
}

pub fn save_config(storage: &mut dyn Storage, config: &Config) -> StdResult<()> {
    CONFIG.save(storage, config)
}

//Pool 스토리지 읽어오는 함수
pub fn load_config(storage: &dyn Storage) -> StdResult<Config> {
    CONFIG.load(storage)
}
pub fn register_axis_contract(storage: &mut dyn Storage, axis_contract: Addr) -> StdResult<()> {
    let mut config = load_config(storage)?;
    config.axis_contract = axis_contract;
    save_config(storage, &config)?;
    Ok(())
}

pub fn load_pair(
    storage: &dyn Storage,
    base_denom: &String,
    price_denom: &String,
) -> StdResult<Addr> {
    PAIR_POOL.load(storage, (base_denom, price_denom))
}
pub fn check_pair(
    storage: &dyn Storage,
    base_denom: &String,
    price_denom: &String,
) -> Result<(), ContractError> {
    match PAIR_POOL.load(storage, (base_denom, price_denom)) {
        Ok(_) => Err(ContractError::AlreadyExistsPair {}),
        Err(_) => Ok(()),
    }
}

pub fn register_pair_pool_contract(
    storage: &mut dyn Storage,
    base_denom: &String,
    price_denom: &String,
    pool_contract: Addr,
) -> StdResult<()> {
    PAIR_POOL.save(storage, (base_denom, price_denom), &pool_contract)
}

pub const CONFIG: Item<Config> = Item::new("config");

pub const PAIR_POOL: Map<(&String, &String), Addr> = Map::new("pair");
pub const PAIR_POOL_LP_STAKING_CONTRACT: Map<(&String, &String), Addr> =
    Map::new("pair_lp_contract");
pub const PAIR_MARKET_CONTRACT: Map<(&String, &String), Addr> = Map::new("pair_market_contract");
