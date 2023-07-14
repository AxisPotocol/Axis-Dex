use cosmwasm_std::{Addr, Coin};

use crate::ContractError;

pub fn check_funds_and_get_token(funds: Vec<Coin>, denom: &String) -> Result<Coin, ContractError> {
    let valid_token = funds
        .iter()
        .find(|c| c.denom == *denom)
        .ok_or_else(|| ContractError::InvalidDenom {})?;

    Ok(valid_token.clone())
}

pub fn check_valid_denom(denom_list: &Vec<String>, denom: &String) -> Result<(), ContractError> {
    match denom_list.contains(denom) {
        true => Ok(()),
        false => Err(ContractError::InvalidDenom {}),
    }
}

pub fn check_core_contract(core: &Addr, sender: &Addr) -> Result<(), ContractError> {
    match *core == *sender {
        true => Ok(()),
        false => Err(ContractError::Unauthorized {}),
    }
}

pub fn check_owner(sender: &Addr, owner: &Addr) -> Result<(), ContractError> {
    match *sender == *owner {
        true => Ok(()),
        false => Err(ContractError::Unauthorized {}),
    }
}
