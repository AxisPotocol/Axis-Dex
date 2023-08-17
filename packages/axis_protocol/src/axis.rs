use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128};

#[cw_serde]
pub struct InstantiateMsg {
    pub core_contract: Addr,
    pub owner: Addr,
    // pub airdrop_contract: Addr,
    // pub airdrop_instantiate_msg: AirDropInstantiateMsg,
}

#[cw_serde]
pub enum ExecuteMsg {
    AddFeeAmount {
        base_denom: String,
        price_denom: String,
        trader: Addr,
        fee_usd_amount: Uint128,
    },
    Setting {
        epoch: u64,
    },
    ClaimMintTrader {},
    ClaimMintMaker {
        base_denom: String,
        price_denom: String,
        sender: Addr,
        amount: Uint128,
    }, // RegisterAirDrop {
       //     air_drop_contract: String,
       // },
}
// query 뭐가 필요할까?
//1.config
//2. treasury
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ConfigResponse)]
    GetConfig {},
    #[returns(PendingFeeResponse)]
    GetPendingTotalFee {},
    #[returns(PoolAllowedMintAmountResponse)]
    GetPoolAllowedMintAmount {
        base_denom: String,
        price_denom: String,
        start_epoch: u64,
    },
    #[returns(TotalSupplyResponse)]
    GetTotalSupply {},
    #[returns(Vec<EpochTotalFeeAmountResponse>)]
    GetEpochTotalFeeAmount { start_epoch: u64, end_epoch: u64 },
}

#[cw_serde]
pub struct ConfigResponse {
    pub core_contract: String,
    pub axis_denom: String,
    pub mint_per_epoch_maker_amount: Uint128,
    pub mint_per_epoch_trader_amount: Uint128,
}
#[cw_serde]
pub struct PendingFeeResponse {
    pub pending_total_fee: Uint128,
}
#[cw_serde]
pub struct PoolAllowedMintAmountResponse {
    pub mint_amount: Uint128,
    pub epoch: u64,
}

#[cw_serde]
pub struct TotalSupplyResponse {
    pub denom: String,
    pub total_supply: Uint128,
}

#[cw_serde]
pub struct EpochTotalFeeAmountResponse {
    pub epoch: u64,
    pub amount: Uint128,
}
