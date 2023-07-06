use cosmwasm_schema::cw_serde;
use cosmwasm_std::Uint128;
use cw_storage_plus::{Item, Map};

#[cw_serde]
pub struct Config {
    es_axis_denom: String,
    es_axis_total_supply: Uint128,
}

//denom balance
pub const BALANCE: Map<&str, Uint128> = Map::new("Vault Balance");

pub const CONFIG: Item<Config> = Item::new("Valut Config");
