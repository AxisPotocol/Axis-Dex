use cosmwasm_schema::cw_serde;

use cosmwasm_std::{Addr, Decimal, StdResult, Storage, Uint128};
use cw_storage_plus::Item;

use crate::position::Position;

//@@Config
#[cw_serde]
pub struct Config {
    pub base_denom: String,
    pub price_denom: String,
    pub base_decimal: u8,
    pub price_decimal: u8,
    //최대 레버리지
    pub max_leverage: u8,
    //@@ 얼마일지 정해야함.
    pub borrow_fee_rate: u8,
    //open close 시 각각 0.1% 총 0.2%공제
    pub open_close_fee_rate: u8,
    //0.2 % 추가
    pub limit_profit_loss_open_fee_rate: u8,
    //base/total * 0.01
    //open 0.1 //close 0.1 open 시 0.2 공제
    //stop limit fee 0.1 + open/close fee
    //최소 금액
    pub pool_contract: Addr,
    pub vault_contract: Addr,
    pub axis_contract: Addr,
}

pub fn save_config(storage: &mut dyn Storage, config: &Config) -> StdResult<()> {
    CONFIG.save(storage, config)
}

pub fn load_config(storage: &dyn Storage) -> StdResult<Config> {
    CONFIG.load(storage)
}

#[cw_serde]
pub struct State {
    pub base_coin_total_fee: Uint128,
    pub price_coin_total_fee: Uint128,
    pub past_price: Decimal,
}

pub fn save_state(storage: &mut dyn Storage, state: &State) -> StdResult<()> {
    STATE.save(storage, state)
}

pub fn load_state(storage: &dyn Storage) -> StdResult<State> {
    STATE.load(storage)
}

pub fn state_send_fee(state: &mut State) -> (Uint128, Uint128) {
    let base_coin_total_fee = state.base_coin_total_fee;
    let price_coin_total_fee = state.price_coin_total_fee;
    state.base_coin_total_fee = Uint128::zero();
    state.price_coin_total_fee = Uint128::zero();

    (base_coin_total_fee, price_coin_total_fee)
}
pub fn state_fee_zero_reset(state: &mut State) {
    state.base_coin_total_fee = Uint128::zero();
    state.price_coin_total_fee = Uint128::zero();
}

pub fn update_past_price(state: &mut State, past_price: Decimal) {
    state.past_price = past_price;
}
//@@Fee_Config

pub const CONFIG: Item<Config> = Item::new("config");
pub const STATE: Item<State> = Item::new("state");
