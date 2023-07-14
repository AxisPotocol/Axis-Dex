use cosmwasm_std::{Addr, Decimal, QuerierWrapper, Uint128};

use axis_protocol::pool::{PositionBalance, QueryMsg as PoolQueryMsg};
use sei_cosmwasm::{SeiQuerier, SeiQueryWrapper};

use crate::{error::ContractError, position::Position};

pub fn query_base_coin_price_and_price_coin_price<'a>(
    querier: &'a QuerierWrapper<SeiQueryWrapper>,
    base_denom: &String,
    stable_denom: &String,
) -> Result<(Decimal, Decimal), ContractError> {
    let querier = SeiQuerier::new(querier);
    let exchange_rate_res = querier.query_exchange_rates()?;
    let base_coin_price = exchange_rate_res
        .denom_oracle_exchange_rate_pairs
        .iter()
        .find(|d| d.denom == *base_denom)
        .map(|d| d.oracle_exchange_rate.exchange_rate)
        .ok_or_else(|| ContractError::NotFoundExchangeRate {})?;

    let price_coin_price = exchange_rate_res
        .denom_oracle_exchange_rate_pairs
        .iter()
        .find(|d| d.denom == *stable_denom)
        .map(|d| d.oracle_exchange_rate.exchange_rate)
        .ok_or_else(|| ContractError::NotFoundExchangeRate {})?;
    match base_coin_price.decimal_places() == 18 && price_coin_price.decimal_places() == 18 {
        true => Ok((base_coin_price, price_coin_price)),
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
