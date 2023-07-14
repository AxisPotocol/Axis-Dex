use axis_protocol::axis::{PoolAllowedMintAmountResponse, QueryMsg as AxisQueryMsg};

use cosmwasm_std::{Addr, QuerierWrapper, StdResult, Uint128};
use sei_cosmwasm::SeiQueryWrapper;

pub fn query_pool_mint_amounts(
    querier: QuerierWrapper<SeiQueryWrapper>,
    axis_contract: &Addr,
    base_denom: &String,
    price_denom: &String,
    start_epoch: u64,
    end_epoch: u64,
) -> StdResult<Vec<(u64, Uint128)>> {
    let axis_res: PoolAllowedMintAmountResponse = querier.query_wasm_smart(
        axis_contract,
        &AxisQueryMsg::GetPoolAllowedMintAmount {
            base_denom: base_denom.to_owned(),
            price_denom: price_denom.to_owned(),
            start_epoch,
            end_epoch,
        },
    )?;
    Ok(axis_res.mint_amount)
}