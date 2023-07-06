use cosmwasm_std::{Addr, Attribute, Coin};

use crate::{state::Config, ContractError};

pub fn check_owner(sender: &Addr, owner: &Addr) -> Result<(), ContractError> {
    match sender == owner {
        true => Ok(()),
        false => Err(ContractError::Unauthorized {}),
    }
}
pub fn check_valid_stable(config: &Config, stable_denom: &String) -> Result<(), ContractError> {
    match config.accept_stable_denoms.contains(stable_denom) {
        true => Ok(()),
        false => Err(ContractError::InvalidStable {}),
    }?;
    Ok(())
}
pub fn check_denom_and_get_validate_denom(
    funds: Vec<Coin>,
    asset_denom: &String,
    stable_denom: &String,
) -> Result<(Coin, Coin), ContractError> {
    let asset_coin = funds
        .iter()
        .find(|c| c.denom == *asset_denom)
        .ok_or_else(|| ContractError::InvalidDenom {})?;
    let stable_coin = funds
        .iter()
        .find(|c| c.denom == *stable_denom)
        .ok_or_else(|| ContractError::InvalidDenom {})?;
    Ok((asset_coin.clone(), stable_coin.clone()))
}

pub fn find_attribute_value(
    attributes: &Vec<Attribute>,
    key: &str,
) -> Result<String, ContractError> {
    for attribute in attributes {
        if attribute.key == key {
            return Ok(attribute.value.to_string());
        }
    }
    Err(ContractError::PoolContractInstantiationFailed {})
}
