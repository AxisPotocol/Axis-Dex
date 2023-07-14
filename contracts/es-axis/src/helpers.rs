use cosmwasm_std::{Addr, Coin};

use crate::{state::Config, ContractError};

pub fn check_staking_contract(sender: &Addr, state: &Config) -> Result<(), ContractError> {
    match *sender == state.staking_contarct {
        true => Ok(()),
        false => Err(ContractError::Unauthorized {}),
    }
}

pub fn check_funds_and_get_es_axis(
    funds: Vec<Coin>,
    axis_denom: &String,
) -> Result<Coin, ContractError> {
    let es_axis = funds
        .iter()
        .find(|c| c.denom == *axis_denom)
        .ok_or_else(|| ContractError::InvalidDenom {})?;

    Ok(es_axis.clone())
}
