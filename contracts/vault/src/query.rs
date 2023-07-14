use crate::ContractError;
use axis_protocol::es_axis::QueryMsg as ESAxisQueryMsg;
use cosmwasm_std::{Addr, QuerierWrapper, StdResult, Uint128};
use sei_cosmwasm::SeiQueryWrapper;

// @@query supply 되면 삭제
pub fn query_ex_axis_total_supply(
    querier: QuerierWrapper<SeiQueryWrapper>,
    es_axis_contract: &Addr,
) -> Result<Uint128, ContractError> {
    let total_supply: Uint128 = querier.query_wasm_smart(
        es_axis_contract.to_string(),
        &ESAxisQueryMsg::GetTotalSupply {},
    )?;
    Ok(total_supply)
}

// pub fn query_total_supply(querier: QuerierWrapper<SeiQueryWrapper>) -> StdResult<Uint128> {}
