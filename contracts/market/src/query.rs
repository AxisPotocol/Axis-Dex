use cosmwasm_std::{Addr, Decimal, QuerierWrapper, Uint128};

use rune::pool::{PositionBalance, QueryMsg as PoolQueryMsg};
use sei_cosmwasm::{SeiQuerier, SeiQueryWrapper};

use crate::{error::ContractError, position::Position};

pub fn query_entry_and_stable_price<'a>(
    querier: &'a QuerierWrapper<SeiQueryWrapper>,
    asset_denom: &String,
    stable_denom: &String,
) -> Result<(Decimal, Decimal), ContractError> {
    let querier = SeiQuerier::new(querier);
    let exchange_rate_res = querier.query_exchange_rates()?;
    let asset_price = exchange_rate_res
        .denom_oracle_exchange_rate_pairs
        .iter()
        .find(|d| d.denom == *asset_denom)
        .map(|d| d.oracle_exchange_rate.exchange_rate)
        .ok_or_else(|| ContractError::NotFoundExchangeRate {})?;

    let stable_price = exchange_rate_res
        .denom_oracle_exchange_rate_pairs
        .iter()
        .find(|d| d.denom == *stable_denom)
        .map(|d| d.oracle_exchange_rate.exchange_rate)
        .ok_or_else(|| ContractError::NotFoundExchangeRate {})?;
    match asset_price.decimal_places() == 18 && stable_price.decimal_places() == 18 {
        true => Ok((asset_price, stable_price)),
        false => Err(ContractError::DecimalError {}),
    }
}

pub fn query_pool_balance(
    querier: QuerierWrapper<SeiQueryWrapper>,
    pool_contract: &Addr,
    position: &Position,
) -> Result<Uint128, ContractError> {
    let pool_balance: PositionBalance = querier.query_wasm_smart(
        pool_contract.to_string(),
        &PoolQueryMsg::GetPositionBalance {
            position: position.convert_boolean(),
        },
    )?;
    Ok(pool_balance.amount)
}
