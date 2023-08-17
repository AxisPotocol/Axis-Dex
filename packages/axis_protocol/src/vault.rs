use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Uint128;

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
    #[returns(DenomBalanceResponse)]
    GetDenomBalance { denom: String },
    #[returns(DenomPendingBalanceResponse)]
    GetDenomPendingBalance { denom: String },
    #[returns(AddressBalanceResponse)]
    GetAddressBalance { address: String },
}

#[cw_serde]
pub struct DenomBalanceResponse {
    pub denom: String,
    pub amount: Uint128,
}

#[cw_serde]
pub struct DenomPendingBalanceResponse {
    pub denom: String,
    pub amount: Uint128,
}

#[cw_serde]
pub struct AddressBalanceResponse {
    pub balances: Vec<DenomBalanceResponse>,
}
