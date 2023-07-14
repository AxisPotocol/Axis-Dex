use cosmwasm_std::{Addr, Coin, Uint128};

use crate::ContractError;

pub fn check_denom_and_amount(
    funds: Vec<Coin>,
    base_denom: &String,
    base_amount: Uint128,
    price_denom: &String,
    price_amount: Uint128,
) -> Result<(), ContractError> {
    funds
        .iter()
        .find(|c| c.denom == *base_denom && c.amount == base_amount)
        .ok_or_else(|| ContractError::InvalidDenom {})?;
    funds
        .iter()
        .find(|c| c.denom == *price_denom && c.amount == price_amount)
        .ok_or_else(|| ContractError::InvalidDenom {})?;
    Ok(())
}
pub fn check_funds_and_get_token(funds: Vec<Coin>, denom: &String) -> Result<Coin, ContractError> {
    let valid_token = funds
        .into_iter()
        .find(|c| c.denom == *denom)
        .ok_or_else(|| ContractError::InvalidDenom {})?;
    Ok(valid_token)
}

pub fn check_valid_denom(
    denom_list: &Vec<String>,
    base_denom: &String,
    price_denom: &String,
) -> Result<(), ContractError> {
    match denom_list.contains(base_denom) | denom_list.contains(price_denom) {
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
