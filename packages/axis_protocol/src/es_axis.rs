use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128};

#[cw_serde]
pub struct InstantiateMsg {}

#[cw_serde]
pub enum ExecuteMsg {
    Claim { sender: Addr, amount: Uint128 },
    Mint { amount: Uint128 },
    Burn {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(Uint128)]
    GetTotalSupply {},
    #[returns(ConfigResponse)]
    GetConfig {},
}

#[cw_serde]
pub struct ConfigResponse {
    pub staking_contarct: String,
    pub es_axis_total_supply: Uint128,
    pub es_axis_denom: String,
}
