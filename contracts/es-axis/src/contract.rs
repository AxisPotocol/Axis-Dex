#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128,
};
use cw2::set_contract_version;
use sei_cosmwasm::{SeiMsg, SeiQueryWrapper};
// use cw2::set_contract_version;

use crate::{
    error::ContractError,
    state::{save_config, Config},
};

use axis_protocol::es_axis::{ExecuteMsg, InstantiateMsg, QueryMsg};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:es-axis";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const ES_AXIS_DENOM: &str = "esAxis";
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut<SeiQueryWrapper>,
    env: Env,
    info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response<SeiMsg>, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let es_axis_denom =
        "factory/".to_string() + env.contract.address.to_string().as_ref() + "/" + ES_AXIS_DENOM;

    let config = Config {
        staking_contarct: info.sender,
        es_axis_total_supply: Uint128::zero(),
        es_axis_denom,
    };
    save_config(deps.storage, &config)?;
    let es_axis_create_msg = SeiMsg::CreateDenom {
        subdenom: ES_AXIS_DENOM.to_owned(),
    };

    Ok(Response::new().add_message(es_axis_create_msg))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut<SeiQueryWrapper>,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<SeiMsg>, ContractError> {
    match msg {
        ExecuteMsg::Claim { sender, amount } => execute::claim(deps, info, sender, amount),
        ExecuteMsg::Mint { amount } => execute::mint(deps, info, amount),
        ExecuteMsg::Burn {} => execute::burn(deps, info),
    }
}

pub mod execute {
    use cosmwasm_std::{coin, Addr, BankMsg, SubMsg, Uint128};
    use sei_cosmwasm::SeiQueryWrapper;

    use crate::{
        helpers::{check_funds_and_get_es_axis, check_staking_contract},
        state::load_config,
    };

    use super::*;

    pub fn claim(
        deps: DepsMut<SeiQueryWrapper>,
        info: MessageInfo,
        sender: Addr,
        amount: Uint128,
    ) -> Result<Response<SeiMsg>, ContractError> {
        let config = load_config(deps.storage)?;
        check_staking_contract(&info.sender, &config)?;

        let es_axis_token = coin(amount.into(), "es_AXIS");

        let send_msg = SubMsg::new(BankMsg::Send {
            to_address: sender.to_string(),
            amount: vec![es_axis_token],
        });
        Ok(Response::new().add_submessage(send_msg))
    }
    pub fn mint(
        deps: DepsMut<SeiQueryWrapper>,
        info: MessageInfo,
        amount: Uint128,
    ) -> Result<Response<SeiMsg>, ContractError> {
        let mut config = load_config(deps.storage)?;

        check_staking_contract(&info.sender, &config)?;

        let new_tokens = coin(amount.into(), config.es_axis_denom.to_owned());

        let es_axis_mint_msg = SeiMsg::MintTokens { amount: new_tokens };

        config.es_axis_total_supply += amount;

        save_config(deps.storage, &config)?;

        Ok(Response::new().add_message(es_axis_mint_msg))
    }
    pub fn burn(
        deps: DepsMut<SeiQueryWrapper>,
        info: MessageInfo,
    ) -> Result<Response<SeiMsg>, ContractError> {
        let mut config = load_config(deps.storage)?;

        let burn_ex_axis = check_funds_and_get_es_axis(info.funds, &config.es_axis_denom)?;
        config.es_axis_total_supply -= burn_ex_axis.amount;
        save_config(deps.storage, &config)?;

        let burn_msg = SeiMsg::BurnTokens {
            amount: burn_ex_axis,
        };

        Ok(Response::new().add_message(burn_msg))
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps<SeiQueryWrapper>, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetTotalSupply {} => to_binary(&query::total_supply(deps)?),
        QueryMsg::GetConfig {} => to_binary(&query::config(deps)?),
    }
}

pub mod query {
    use crate::state::load_config;
    use axis_protocol::es_axis::ConfigResponse;
    use cosmwasm_std::{Deps, StdResult, Uint128};
    use sei_cosmwasm::SeiQueryWrapper;

    pub fn total_supply(deps: Deps<SeiQueryWrapper>) -> StdResult<Uint128> {
        let config = load_config(deps.storage)?;
        Ok(config.es_axis_total_supply)
    }
    pub fn config(deps: Deps<SeiQueryWrapper>) -> StdResult<ConfigResponse> {
        let config = load_config(deps.storage)?;
        Ok(ConfigResponse {
            staking_contarct: config.staking_contarct.to_string(),
            es_axis_denom: config.es_axis_denom,
            es_axis_total_supply: config.es_axis_total_supply,
        })
    }
}

#[cfg(test)]
mod tests {}
