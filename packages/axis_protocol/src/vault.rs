use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128};

#[cw_serde]
pub struct InstantiateMsg {
    pub core_contract: String,
    pub es_axis_contract: String,
    pub es_axis_denom: String,
    pub denom_list: Vec<String>,
}

#[cw_serde]
pub enum ExecuteMsg {
    RecievedFee {
        base_denom: String,
        base_amount: Uint128,
        price_denom: String,
        price_amount: Uint128,
    },
    Swap {},
    Setting {
        epoch: u64,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(GetDenomBalanceResponse)]
    GetDenomBalance { denom: String },
    #[returns(GetDenomPendingBalanceResponse)]
    GetDenomPendingBalance { denom: String },
    #[returns(GetAddressBalanceResponse)]
    GetAddressBalance { address: String },
}

#[cw_serde]
pub struct GetDenomBalanceResponse {
    pub denom: String,
    pub amount: Uint128,
}

#[cw_serde]
pub struct GetDenomPendingBalanceResponse {
    pub denom: String,
    pub amount: Uint128,
}

#[cw_serde]
pub struct GetAddressBalanceResponse {
    pub balances: Vec<GetDenomBalanceResponse>,
}
