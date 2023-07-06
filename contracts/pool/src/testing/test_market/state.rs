use cosmwasm_schema::cw_serde;

use cosmwasm_std::{Addr, Decimal, StdResult, Storage, Uint128};
use cw_storage_plus::Item;

use crate::position::Position;

//@@Config
#[cw_serde]
pub struct Config {
    pub owner: Addr,

    pub asset_denom: String,
    pub stable_denom: String,
    pub asset_decimal: u8,
    pub stable_decimal: u8,
    //최대 레버리지
    pub max_leverage: u8,

    //asset/total * 0.01
    //open 0.1 //close 0.1 open 시 0.2 공제
    //stop limit fee 0.1 + open/close fee
    //최소 금액
    pub pool_contract: Addr,
    pub fee_vault_contract: Addr,
    pub treasury_contract: Addr,
    //총 fee open 시 pool에 전송
    pub asset_total_fee: Uint128,
    pub stable_total_fee: Uint128,
    pub past_price: Decimal, //@@
}
pub fn save_config(storage: &mut dyn Storage, config: &Config) -> StdResult<()> {
    CONFIG.save(storage, config)
}

pub fn load_config(storage: &dyn Storage) -> StdResult<Config> {
    CONFIG.load(storage)
}
pub fn save_config_fee(
    storage: &mut dyn Storage,
    config: &mut Config,
    position: &Position,
    fee: Uint128,
) -> StdResult<()> {
    match position {
        Position::Long => config.asset_total_fee += fee,
        Position::Short => config.stable_total_fee += fee,
    }
    save_config(storage, config)
}
pub fn config_send_fee(config: &mut Config) -> (Uint128, Uint128) {
    let asset_total_fee = config.asset_total_fee;
    let stable_total_fee = config.stable_total_fee;
    config.asset_total_fee = Uint128::zero();
    config.stable_total_fee = Uint128::zero();

    (asset_total_fee, stable_total_fee)
}
pub fn config_fee_zero_reset(config: &mut Config) {
    config.asset_total_fee = Uint128::zero();
    config.stable_total_fee = Uint128::zero();
}
pub fn get_config_pool_contract(storage: &mut dyn Storage) -> StdResult<Addr> {
    let config = load_config(storage)?;
    Ok(config.pool_contract)
}
pub fn get_config_fee_valut_contract(storage: &dyn Storage) -> StdResult<Addr> {
    let config = load_config(storage)?;
    Ok(config.fee_vault_contract)
}
pub fn update_past_price(config: &mut Config, past_price: Decimal) {
    config.past_price = past_price;
}
//@@Fee_Config
#[cw_serde]
pub struct FeeConfig {
    //@@ 얼마일지 정해야함.
    pub borrow_fee_rate: u8,
    //open close 시 각각 0.1% 총 0.2%공제
    pub open_close_fee_rate: u8,
    //0.2 % 추가
    pub limit_profit_loss_open_fee_rate: u8,
}
pub fn load_fee_config(storage: &dyn Storage) -> StdResult<FeeConfig> {
    FEE_CONFIG.load(storage)
}
pub fn load_open_and_close_fee(storage: &dyn Storage) -> StdResult<u8> {
    let fee_config = FEE_CONFIG.load(storage)?;
    Ok(fee_config.open_close_fee_rate)
}

pub const CONFIG: Item<Config> = Item::new("config");
pub const FEE_CONFIG: Item<FeeConfig> = Item::new("fee_config");
