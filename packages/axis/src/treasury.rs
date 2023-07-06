use cosmwasm_schema::cw_serde;
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
        asset_denom: String,
        stable_denom: String,
        trader: String,
        fee_usd_amount: Uint128,
    },
    Setting {},
    ClaimMint {},
    // RegisterAirDrop {
    //     air_drop_contract: String,
    // },
}
// query 뭐가 필요할까?
//1.config
//2. treasury
#[cw_serde]
pub enum QueryMsg {}
