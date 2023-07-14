use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128};

#[cw_serde]
pub struct InstantiateMsg {
    pub base_denom: String,
    pub price_denom: String,
    pub axis_contract: Addr,
    pub core_contract: Addr,
    pub lp_denom: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    Staking {},
    ClaimReward {},
    UnStaking {},
    Withdraw {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ConfigResponse)]
    GetConfig {},
}

#[cw_serde]
pub struct ConfigResponse {
    pub core_contract: Addr,
    pub axis_contract: Addr,
    pub lp_denom: String,
    pub base_denom: String,
    pub price_denom: String,
    pub staking_total: Uint128,
}
