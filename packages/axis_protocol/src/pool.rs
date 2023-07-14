use crate::lp_staking::InstantiateMsg as LpStakingInstantiateMsg;
use crate::market::InstantiateMsg as MarketInstantiateMsg;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128};

#[cw_serde]
pub struct InstantiateMsg {
    pub maker: Addr,
    pub base_denom: String,
    pub base_decimal: u8,
    pub price_denom: String,
    pub price_decimal: u8,
    pub maximum_borrow_rate: u8,
    pub market_code_id: u64,
    pub market_instantiate_msg: MarketInstantiateMsg,
    pub lp_staking_code_id: u64,
    pub axis_contract: Addr,
    // pub lp_decimal: u8,
    //pub fee_Valut_contract:String
}

#[cw_serde]
pub enum ExecuteMsg {
    LeverageBorrow {
        position: bool,
        amount: Uint128,
    },
    RePay {
        denom: String,
        position: bool,
        amount: Uint128,
        borrowed_amount: Uint128,
    },
    Deposit {},
    Withdraw {},
    Lock {},
    UnLock {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    // GetCount returns the current count as a json-encoded number
    #[returns(PositionInformationResponse)]
    GetPositionInformation { position: bool },
    #[returns(PositionBalance)]
    GetPositionBalance { position: bool },
    #[returns(ConfigResponse)]
    GetConfig {},
    #[returns(PoolResponse)]
    GetPool {},
}

#[cw_serde]
pub struct PositionInformationResponse {
    pub denom: String,
    pub amount: Uint128,
    pub decimal: u8,
}
#[cw_serde]
pub struct PositionBalance {
    pub amount: Uint128,
}

#[cw_serde]
pub struct ConfigResponse {
    pub lock: bool,
    pub market_contract: Addr,
    pub maximum_borrow_rate: u8,
    pub lp_staking_contract: Addr,
}

#[cw_serde]
pub struct PoolResponse {
    pub base_denom: String,
    pub base_amount: Uint128,
    pub base_decimal: u8,
    pub price_denom: String,
    pub price_amount: Uint128,
    pub price_decimal: u8,
    pub base_borrow_amount: Uint128,
    pub price_borrow_amount: Uint128,
    pub lp_denom: String,
    pub lp_decimal: u8,
    pub lp_total_supply: Uint128,
}

// #[cw_serde]
// pub struct TreasuryResponse {
//     pub amount: Uint128,
//     pub epoch: u64,
// }
// #[cw_serde]
// pub struct PoolTreasuryResponse {
//     pub treasurys: Vec<TreasuryResponse>,
// }
