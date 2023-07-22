use crate::pool::InstantiateMsg as PoolInstantiateMsg;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;
#[cw_serde]
pub struct InstantiateMsg {
    pub accept_price_denoms: Vec<String>,
    pub axis_code_id: u64,
    pub next_update_timestamp: u64,
}

#[cw_serde]
pub enum ExecuteMsg {
    CreatePair {
        pool_init_msg: PoolInstantiateMsg,
        pool_code_id: u64,
    },
    RegisterPriceDenom {
        price_denom: String,
    },
    AllPoolLock {},
    PairLock {
        base_denom: String,
        price_denom: String,
    },
    AllPoolUnLock {},
    PairUnLock {
        base_denom: String,
        price_denom: String,
    },
    Setting {},
    UpdateConfig {
        vault_contract: Option<String>,
        staking_contract: Option<String>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ConfigResponse)]
    GetConfig {},
    #[returns(PairPoolContractResponse)]
    GetPairPoolContract {
        base_denom: String,
        price_denom: String,
    },
    #[returns(PairMarketContractResponse)]
    GetPairMarketContract {
        base_denom: String,
        price_denom: String,
    },
    #[returns(PairLpStakingContractResponse)]
    GetPairLpStakingContract {
        base_denom: String,
        price_denom: String,
    },
}
#[cw_serde]
pub enum SudoMsg {
    NewEpoch { epoch: u64 },
}
#[cw_serde]
pub struct ConfigResponse {
    pub owner: Addr,
    pub epoch: u64,
    pub accept_price_denoms: Vec<String>,
    pub axis_contract: Addr,
    pub staking_contract: Addr,
    pub vault_contract: Addr,
}
#[cw_serde]
pub struct PairPoolContractResponse {
    pub base_denom: String,
    pub price_denom: String,
    pub pool_contract: Addr,
}

#[cw_serde]
pub struct PairMarketContractResponse {
    pub base_denom: String,
    pub price_denom: String,
    pub market_contract: Addr,
}

#[cw_serde]
pub struct PairLpStakingContractResponse {
    pub base_denom: String,
    pub price_denom: String,
    pub lp_staking_contract: Addr,
}
