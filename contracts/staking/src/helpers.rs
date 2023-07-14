use cosmwasm_std::{Addr, Attribute, Coin, Decimal, Order, StdResult, Storage, Uint128};
use cw_storage_plus::Bound;

use crate::{
    contract::ONE_DAY_PER_MINT,
    error::ContractError,
    state::{load_config, save_config, EPOCH_STAKING_AMOUNT},
};

pub fn check_funds_and_get_axis(
    funds: Vec<Coin>,
    axis_denom: &String,
) -> Result<Coin, ContractError> {
    let asset = funds
        .iter()
        .find(|c| c.denom == *axis_denom)
        .ok_or_else(|| ContractError::InvalidDenom {})?;

    Ok(asset.clone())
}

pub fn compute_mint_amount(
    storage: &dyn Storage,
    staking_amount: Uint128,
    start: u64,
    now: u64,
) -> StdResult<Uint128> {
    if start == now {
        return Ok(Uint128::zero());
    }
    let total = EPOCH_STAKING_AMOUNT
        .range(
            storage,
            Some(Bound::inclusive(start)),
            Some(Bound::exclusive(now)),
            Order::Ascending,
        )
        .into_iter()
        .map(|epoch_staking| {
            let (_, epoch_total_amount) = epoch_staking?;
            let ratio = Decimal::from_ratio(staking_amount, epoch_total_amount);
            let mint_amount = Uint128::new(ONE_DAY_PER_MINT) * ratio;
            Ok(mint_amount)
        })
        .sum::<StdResult<Uint128>>()?;

    Ok(total)
}

pub fn check_core_contract(core: &Addr, sender: &Addr) -> Result<(), ContractError> {
    match *core == *sender {
        true => Ok(()),
        false => Err(ContractError::Unauthorized {}),
    }
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
    Err(ContractError::ESAxisContractInstantiateFailed {})
}

pub fn register_es_axis(storage: &mut dyn Storage, es_axis_contract: Addr) -> StdResult<()> {
    let mut config = load_config(storage)?;
    config.es_axis_contract = es_axis_contract;
    save_config(storage, &config)?;
    Ok(())
}
