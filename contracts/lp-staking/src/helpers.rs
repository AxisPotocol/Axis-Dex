use cosmwasm_std::{Addr, Coin, Decimal, DepsMut, Order, StdResult, Uint128};
use cw_storage_plus::Bound;
use sei_cosmwasm::SeiQueryWrapper;

use crate::{
    query::query_pool_mint_amounts,
    state::{Config, EPOCH_STAKING_TOTAL_AMOUNT},
    ContractError,
};

pub fn check_funds_and_get_lp(funds: Vec<Coin>, lp_denom: &String) -> Result<Coin, ContractError> {
    let asset = funds
        .iter()
        .find(|c| c.denom == *lp_denom)
        .ok_or_else(|| ContractError::InvalidDenom {})?;

    Ok(asset.clone())
}

pub fn check_core_contract(core_contract: &Addr, sender: &Addr) -> Result<(), ContractError> {
    match *core_contract == *sender {
        true => Ok(()),
        false => Err(ContractError::Unauthorized {}),
    }
}

pub fn compute_mint_amount(
    deps: &DepsMut<SeiQueryWrapper>,
    config: &Config,
    staking_amount: Uint128,
    start_epoch: u64,
    now_epoch: u64,
) -> StdResult<Uint128> {
    if start_epoch == now_epoch {
        return Ok(Uint128::zero());
    }
    let mint_amounts = query_pool_mint_amounts(
        deps.querier,
        &config.axis_contract,
        &config.base_denom,
        &config.price_denom,
        start_epoch,
    )?;
    let stake_ratio = EPOCH_STAKING_TOTAL_AMOUNT
        .range(
            deps.storage,
            Some(Bound::inclusive(start_epoch)),
            None,
            Order::Ascending,
        )
        .into_iter()
        .map(|data| {
            let (key, total) = data?;
            let ratio = Decimal::from_ratio(staking_amount, total);
            Ok((key, ratio))
        })
        .collect::<StdResult<Vec<(u64, Decimal)>>>()?;
    // @@ mint_epoch 이랑 epoch 이랑 다를 경우가 있을까?

    let staker_mint_amount = mint_amounts
        .into_iter()
        .zip(stake_ratio.into_iter())
        .map(|(item, (_, ratio))| Ok(item.mint_amount * ratio))
        .sum::<StdResult<Uint128>>()?;

    Ok(staker_mint_amount)
}
