use cosmwasm_schema::cw_serde;

use cosmwasm_std::{Addr, Decimal, Response, StdResult, Storage, Uint128};
use cw_storage_plus::Item;

use crate::error::ContractError;

#[cw_serde]
pub struct Pool {
    pub base_amount: Uint128,
    pub price_amount: Uint128,
    pub base_borrow_amount: Uint128,
    pub price_borrow_amount: Uint128,
    pub lp_total_supply: Uint128,
}

pub fn save_pool(storage: &mut dyn Storage, pool: &Pool) -> StdResult<()> {
    POOL.save(storage, pool)
}

//Pool 스토리지 읽어오는 함수
pub fn load_pool(storage: &dyn Storage) -> StdResult<Pool> {
    POOL.load(storage)
}

#[cw_serde]
pub struct Config {
    pub base_denom: String,
    pub base_decimal: u8,
    pub price_denom: String,
    pub price_decimal: u8,
    pub core_contract: Addr,
    pub lock: bool,
    pub market_contract: Addr,
    pub lp_denom: String,
    pub lp_decimal: u8,
    pub maximum_borrow_rate: u8,
    pub lp_staking_contract: Addr,
    pub withdraw_fee_rate: Decimal,
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

pub const POOL: Item<Pool> = Item::new("pool");
pub const CONFIG: Item<Config> = Item::new("config");
