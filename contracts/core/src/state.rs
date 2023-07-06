use cosmwasm_schema::cw_serde;

use cosmwasm_std::{Addr, StdResult, Storage, Uint128};
use cw_storage_plus::{Item, Map};

use crate::ContractError;

#[cw_serde]
pub struct Config {
    pub owner: Addr,
    pub accept_stable_denoms: Vec<String>,
    pub treausry_contract_address: Addr,
}

pub fn save_config(storage: &mut dyn Storage, config: &Config) -> StdResult<()> {
    CONFIG.save(storage, config)
}

//Pool 스토리지 읽어오는 함수
pub fn load_config(storage: &dyn Storage) -> StdResult<Config> {
    CONFIG.load(storage)
}
pub fn register_treasury(storage: &mut dyn Storage, treasury_contract: Addr) -> StdResult<()> {
    let mut config = load_config(storage)?;
    config.treausry_contract_address = treasury_contract;
    save_config(storage, &config)?;
    Ok(())
}

pub fn load_pair(
    storage: &dyn Storage,
    asset_denom: &String,
    stable_denom: &String,
) -> StdResult<Addr> {
    PAIR_POOL.load(storage, (asset_denom.to_string(), stable_denom.to_string()))
}
pub fn check_pair(
    storage: &dyn Storage,
    asset_denom: &String,
    stable_denom: &String,
) -> Result<(), ContractError> {
    match PAIR_POOL.load(storage, (asset_denom.to_string(), stable_denom.to_string())) {
        Ok(_) => Err(ContractError::AlreadyExistsPair {}),
        Err(_) => Ok(()),
    }
}

pub fn register_pair(
    storage: &mut dyn Storage,
    asset_denom: String,
    stable_denom: String,
    pool_contract: Addr,
) -> StdResult<()> {
    PAIR_POOL.save(storage, (asset_denom, stable_denom), &pool_contract)
}

pub const CONFIG: Item<Config> = Item::new("config");

pub const PAIR_POOL: Map<(String, String), Addr> = Map::new("pair");
