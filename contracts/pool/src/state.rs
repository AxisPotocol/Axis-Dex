use cosmwasm_schema::cw_serde;

use cosmwasm_std::{Addr, Response, StdResult, Storage, Uint128};
use cw_storage_plus::{Item, Map};

use crate::error::ContractError;

#[cw_serde]
pub struct Pool {
    pub base_denom: String,
    pub base_amount: Uint128,
    pub base_decimal: u8,
    pub price_denom: String,
    pub price_amount: Uint128,
    pub price_decimal: u8,
    pub base_borrow_amount: Uint128,
    pub price_borrow_amount: Uint128,
    pub lp_total_supply: Uint128,
    pub lp_decimal: u8,
    pub lp_denom: String,
}

pub fn save_pool(storage: &mut dyn Storage, pool: &Pool) -> StdResult<()> {
    POOL.save(storage, pool)
}

//Pool 스토리지 읽어오는 함수
pub fn load_pool(storage: &dyn Storage) -> StdResult<Pool> {
    POOL.load(storage)
}

pub fn save_add_amount_pool(
    storage: &mut dyn Storage,
    pool: &mut Pool,
    base_amount: Uint128,
    price_amount: Uint128,
) -> StdResult<()> {
    pool.base_amount += base_amount;
    pool.price_amount += price_amount;
    save_pool(storage, &pool)
}
pub fn save_remove_amount_pool(
    storage: &mut dyn Storage,
    pool: &mut Pool,
    base_amount: Uint128,
    price_amount: Uint128,
) -> StdResult<()> {
    pool.base_amount -= base_amount;
    pool.price_amount -= price_amount;
    save_pool(storage, &pool)
}
pub fn save_add_total_supply(
    storage: &mut dyn Storage,
    pool: &mut Pool,
    lp_mint_amount: Uint128,
) -> StdResult<()> {
    pool.lp_total_supply += lp_mint_amount;
    save_pool(storage, pool)
}

#[cw_serde]
pub struct Config {
    pub core_contract: Addr,
    pub lock: bool,
    pub market_contract: Addr,
    pub maximum_borrow_rate: u8,
    pub lp_staking_contract: Addr,
}
pub fn register_market_contract(
    storage: &mut dyn Storage,
    market_addr: Addr,
) -> Result<Response, ContractError> {
    let mut config = load_config(storage)?;
    if config.market_contract != Addr::unchecked("") {
        return Err(ContractError::Unauthorized {});
    }

    config.market_contract = market_addr;
    save_config(storage, &config)?;

    Ok(Response::new())
}

pub fn save_config(storage: &mut dyn Storage, config: &Config) -> StdResult<()> {
    CONFIG.save(storage, config)
}

//Pool 스토리지 읽어오는 함수
pub fn load_config(storage: &dyn Storage) -> StdResult<Config> {
    CONFIG.load(storage)
}

pub fn load_market_contract(storage: &dyn Storage) -> StdResult<Addr> {
    let config = load_config(storage)?;
    Ok(config.market_contract)
}

pub fn load_maximum_borrow_rate(storage: &dyn Storage) -> StdResult<u8> {
    let config = load_config(storage)?;
    Ok(config.maximum_borrow_rate)
}

pub const POOL: Item<Pool> = Item::new("pool");
pub const CONFIG: Item<Config> = Item::new("config");
