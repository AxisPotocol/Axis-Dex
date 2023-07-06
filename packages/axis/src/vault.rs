use cosmwasm_schema::cw_serde;
use cosmwasm_std::Uint128;

#[cw_serde]
pub enum ExecuteMsg {
    RecievedFee { denom: String, amount: Uint128 },
}
