use axis::core::{PairContractResponse, QueryMsg as CoreQueryMsg};
use axis::pool::{ConfigResponse, QueryMsg as PoolQueryMsg};
use cosmwasm_std::{Addr, QuerierWrapper};
use sei_cosmwasm::SeiQueryWrapper;

use crate::error::ContractError;

pub fn query_pair_market(
    querier: QuerierWrapper<SeiQueryWrapper>,
    core_contract: &Addr,
    asset_denom: &String,
    stable_denom: &String,
) -> Result<String, ContractError> {
    let core_res: PairContractResponse = querier.query_wasm_smart(
        core_contract.to_string(),
        &CoreQueryMsg::GetPairContract {
            asset_denom: asset_denom.to_string(),
            stable_denom: stable_denom.to_string(),
        },
    )?;

    let pool_res: ConfigResponse =
        querier.query_wasm_smart(core_res.pool_contract, &PoolQueryMsg::GetConfig {})?;

    Ok(pool_res.market_contract)
}
