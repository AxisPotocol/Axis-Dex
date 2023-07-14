use axis_protocol::core::{ConfigResponse, QueryMsg as CoreQueryMsg};
use cosmwasm_std::{Addr, QuerierWrapper, StdResult};
use sei_cosmwasm::SeiQueryWrapper;

pub fn query_epoch(
    querier: QuerierWrapper<SeiQueryWrapper>,
    core_contract: &Addr,
) -> StdResult<u64> {
    let core_config: ConfigResponse =
        querier.query_wasm_smart(core_contract.to_string(), &CoreQueryMsg::GetConfig {})?;

    Ok(core_config.epoch)
}
