use crate::pool::InstantiateMsg as PoolInstantiateMsg;
use cosmwasm_schema::{cw_serde, QueryResponses};
#[cw_serde]
pub struct InstantiateMsg {
    pub accept_stable_denoms: Vec<String>,
    pub rune_treasury_code_id: u64,
}

#[cw_serde]
pub enum ExecuteMsg {
    CreatePair {
        pool_init_msg: PoolInstantiateMsg,
        pool_code_id: u64,
    },
    RegisterStableAsset {
        stable_denom: String,
    },
    AllPoolLock {},
    PairLock {
        asset_denom: String,
        stable_denom: String,
    },
    AllPoolUnLock {},
    PairUnLock {
        asset_denom: String,
        stable_denom: String,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(PairContractResponse)]
    GetPairContract {
        asset_denom: String,
        stable_denom: String,
    },
    #[returns(ConfigResponse)]
    GetConfig {},
}

#[cw_serde]
pub struct PairContractResponse {
    pub asset_denom: String,
    pub stable_denom: String,
    pub pool_contract: String,
}

#[cw_serde]
pub struct ConfigResponse {
    pub owner: String,
    pub accept_stable_denoms: Vec<String>,
    pub treausry_contract_address: String,
}
