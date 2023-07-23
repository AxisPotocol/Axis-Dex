use axis_protocol::axis::{PoolAllowedMintAmountResponse, QueryMsg as AxisQueryMsg};

use cosmwasm_std::{Addr, QuerierWrapper, StdResult};
use sei_cosmwasm::SeiQueryWrapper;

pub fn query_pool_mint_amounts(
    querier: QuerierWrapper<SeiQueryWrapper>,
    axis_contract: &Addr,
    base_denom: &String,
    price_denom: &String,
    start_epoch: u64,
) -> StdResult<Vec<PoolAllowedMintAmountResponse>> {
    let axis_res: Vec<PoolAllowedMintAmountResponse> = querier.query_wasm_smart(
        axis_contract,
        &AxisQueryMsg::GetPoolAllowedMintAmount {
            base_denom: base_denom.to_owned(),
            price_denom: price_denom.to_owned(),
            start_epoch,
        },
    )?;

    Ok(axis_res)
}
