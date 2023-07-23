use cosmwasm_std::Addr;

use crate::error::ContractError;

pub fn check_market_contract(market_contract: &Addr, sender: &Addr) -> Result<(), ContractError> {
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
pub fn check_pool_contract(pool_contract: &Addr, sender: &Addr) -> Result<(), ContractError> {
    match *pool_contract == *sender {
        true => Ok(()),
        false => Err(ContractError::Unauthorized {}),
    }
}

pub fn check_lp_staking_contract(
    lp_staking_contract: &Addr,
    sender: &Addr,
) -> Result<(), ContractError> {
    match *lp_staking_contract == *sender {
        true => Ok(()),
        false => Err(ContractError::Unauthorized {}),
    }
}
