use axis_protocol::core::{
    ConfigResponse as CoreConfigRepsonse, PairLpStakingContractResponse,
    PairMarketContractResponse, PairPoolContractResponse, QueryMsg as CoreQueryMsg,
};
use axis_protocol::pool::{ConfigResponse as PoolConfigResponse, QueryMsg as PoolQueryMsg};
use cosmwasm_std::{Addr, QuerierWrapper};
use sei_cosmwasm::SeiQueryWrapper;

use crate::error::ContractError;

pub fn query_pair_pool_market(
    querier: QuerierWrapper<SeiQueryWrapper>,
    core_contract: &Addr,
    base_denom: &String,
    price_denom: &String,
) -> Result<Addr, ContractError> {
    let core_res: PairMarketContractResponse = querier.query_wasm_smart(
        core_contract.to_string(),
        &CoreQueryMsg::GetPairPoolContract {
            base_denom: base_denom.to_string(),
            price_denom: price_denom.to_string(),
        },
    )?;

    Ok(core_res.market_contract)
}

pub fn query_pair_market_contract(
    querier: QuerierWrapper<SeiQueryWrapper>,
    core_contract: &Addr,
    base_denom: &String,
    price_denom: &String,
) -> Result<Addr, ContractError> {
    let core_res: PairMarketContractResponse = querier.query_wasm_smart(
        core_contract.to_string(),
        &CoreQueryMsg::GetPairMarketContract {
            base_denom: base_denom.to_string(),
            price_denom: price_denom.to_string(),
        },
    )?;
    Ok(core_res.market_contract)
}

pub fn query_pair_lp_staking_contract(
    querier: QuerierWrapper<SeiQueryWrapper>,
    core_contract: &Addr,
    base_denom: &String,
    price_denom: &String,
) -> Result<Addr, ContractError> {
    let core_res: PairLpStakingContractResponse = querier.query_wasm_smart(
        core_contract,
        &CoreQueryMsg::GetPairLpStakingContract {
            base_denom: base_denom.to_string(),
            price_denom: price_denom.to_string(),
        },
    )?;

    Ok(core_res.lp_staking_contract)
}
