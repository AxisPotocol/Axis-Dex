use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128};

#[cw_serde]
pub struct InstantiateMsg {
    pub core_contract: Addr,
    pub axis_denom: String,
    pub es_axis_code: u64,
}

#[cw_serde]
pub enum ExecuteMsg {
    Setting {},
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
    #[returns(StateResponse)]
    GetState {},
    #[returns(StakeInfoResponse)]
    GetStakeInfo { addr: String },
    #[returns(UnStakeInfoResponse)]
    GetUnStakeInfo { addr: String },
    #[returns(Uint128)]
    GetAvailableReward { addr: String },
}

#[cw_serde]
pub struct ConfigResponse {
    pub core_contract: Addr,
    pub axis_denom: String,
    pub es_axis_contract: Addr,
}

#[cw_serde]
pub struct StateResponse {
    pub pending_staking_total: Uint128,
    pub withdraw_pending_total: Uint128,
    pub staking_total: Uint128,
}
#[cw_serde]
pub struct UnStakeResponse {
    pub unlock_epoch: u64,
    pub withdraw_pending_amount: Uint128,
}
#[cw_serde]
pub struct StakeResponse {
    pub start_epoch: u64,
    pub staking_amount: Uint128,
}

#[cw_serde]
pub struct StakeInfoResponse {
    pub stake_infos: Vec<StakeResponse>,
}

#[cw_serde]
pub struct UnStakeInfoResponse {
    pub un_stake_infos: Vec<UnStakeResponse>,
}
