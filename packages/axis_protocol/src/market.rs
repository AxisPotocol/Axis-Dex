use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Uint128};

#[cw_serde]
pub struct InstantiateMsg {
    pub base_denom: String,
    pub base_decimal: u8,
    pub price_denom: String,
    pub price_decimal: u8,
    pub max_leverage: u8,
    pub borrow_fee_rate: u8,
    pub open_close_fee_rate: u8,
    pub limit_profit_loss_open_fee_rate: u8,
    pub axis_contract: Addr,
    pub vault_contract: Addr,
}

#[cw_serde]
pub enum ExecuteMsg {
    Open {
        position: bool,
        leverage: u8,
        limit_profit_price: Option<Uint128>,
        limit_loss_price: Option<Uint128>,
    },
    Close {},
    Liquidated {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(GetConfigResponse)]
    GetConfig {},
    #[returns(GetStateResponse)]
    GetState {},
    #[returns(TradeResponse)]
    GetTrade { trader: String },
}

#[cw_serde]
pub struct GetConfigResponse {
    pub base_denom: String,
    pub price_denom: String,
    pub base_decimal: u8,
    pub price_decimal: u8,
    pub max_leverage: u8,
    pub borrow_fee_rate: u8,
    pub open_close_fee_rate: u8,
    pub limit_profit_loss_open_fee_rate: u8,
    pub pool_contract: Addr,
    pub vault_contract: Addr,
    pub axis_contract: Addr,
}
#[cw_serde]
pub struct GetStateResponse {
    pub base_coin_total_fee: Uint128,
    pub price_coin_total_fee: Uint128,
    pub past_price: Decimal,
}
#[cw_serde]
pub struct TradeResponse {
    //user
    pub trader: Addr,
    //거래 시점 가격
    pub entry_price: Uint128,
    //청산 가격
    pub liquidation_price: Uint128,
    //no limit is Uint128::MAX
    pub limit_profit_price: Uint128,
    pub limit_loss_price: Uint128,

    //증거금 종류
    pub collateral_denom: String,
    //증거금 양 fee 공제 금액
    pub collateral_amount: Uint128,
    //포지션 Long=true or Short=short
    pub position: bool,
    //포지션 사이즈 = 증거금 * 레버리지 비율
    pub position_size: Uint128,
    //레버리지 비율
    pub leverage: u8,
    //레버리지한 금액
    pub leverage_amount: Uint128,
}
