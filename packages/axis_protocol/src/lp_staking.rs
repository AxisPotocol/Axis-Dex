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
    Setting { epoch: u64 },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ConfigResponse)]
    GetConfig {},
    #[returns(StateResponse)]
    GetState {},
    #[returns(Vec<StakeInfoResponse>)]
    GetStakeInfo { address: Addr },
    #[returns(Vec<UnStakeInfoResponse>)]
    GetUnstakeInfo { address: Addr },
    #[returns(EpochTotalStakingResponse)]
    GetEpochTotalStaking { start_epoch: u64, end_epoch: u64 },
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

#[cw_serde]
pub struct StateResponse {
    pub staking_total: Uint128,
    pub withdraw_pending_total: Uint128,
}

#[cw_serde]
pub struct EpochTotalStakingResponse {
    pub epoch: u64,
    pub amount: Uint128,
}

#[cw_serde]
pub struct StakeInfoResponse {
    pub start_epoch: u64,
    pub staking_amount: Uint128,
}

#[cw_serde]
pub struct UnStakeInfoResponse {
    pub unlock_epoch: u64,
    pub unstaking_amount: Uint128,
}
