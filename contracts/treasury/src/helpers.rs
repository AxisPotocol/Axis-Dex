use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{
    to_binary, Addr, CosmosMsg, CustomQuery, Querier, QuerierWrapper, StdResult, WasmMsg, WasmQuery,
};

use crate::error::ContractError;

pub fn check_market_contract(
    market_contract: &String,
    sender: &String,
) -> Result<(), ContractError> {
    match *market_contract == *sender {
        true => Ok(()),
        false => Err(ContractError::Unauthorized {}),
    }
}

pub fn check_core_contract(core_contract: &Addr, sender: &Addr) -> Result<(), ContractError> {
    match *core_contract == *sender {
        true => Ok(()),
        false => Err(ContractError::Unauthorized {}),
    }
}

pub fn check_last_updated(now: u64, latest_update: u64) -> Result<(), ContractError> {
    //86400 = 24hour
    match now - latest_update > 86400 {
        true => Ok(()),
        false => Err(ContractError::NotTime {}),
    }
}
