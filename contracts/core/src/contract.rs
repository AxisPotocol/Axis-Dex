#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdResult, SubMsg,
    SubMsgResult, WasmMsg,
};
use cw2::set_contract_version;
use sei_cosmwasm::{SeiMsg, SeiQueryWrapper};

use crate::error::ContractError;

use crate::helpers::find_attribute_value;
use crate::state::{register_pair, register_treasury, Config, CONFIG};
use axis::core::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg};
use axis::pool::{ExecuteMsg as PoolExecuteMsg, InstantiateMsg as PoolInstantiateMsg};
use axis::treasury::InstantiateMsg as TreasuryInstantiateMsg;
// version info for migration info
const CONTRACT_NAME: &str = "crates.io:core";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut<SeiQueryWrapper>,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response<SeiMsg>, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let config = Config {
        owner: info.sender.clone(),
        accept_stable_denoms: msg.accept_stable_denoms,
        treausry_contract_address: Addr::unchecked(""),
    };
    CONFIG.save(deps.storage, &config)?;
    //@@treasury instantiate
    let treasury_instantiate_msg = WasmMsg::Instantiate {
        admin: Some(env.contract.address.to_string()),
        code_id: msg.rune_treasury_code_id,
        msg: to_binary(&TreasuryInstantiateMsg {
            core_contract: env.contract.address,
            owner: info.sender.clone(),
        })?,
        funds: vec![],
        label: format!("RUNE TREASURY"),
    };
    let treasury_instatiate_tx = SubMsg::reply_on_success(treasury_instantiate_msg, 1);
    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender)
        .add_submessage(treasury_instatiate_tx))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut<SeiQueryWrapper>,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<SeiMsg>, ContractError> {
    match msg {
        ExecuteMsg::CreatePair {
            pool_init_msg,
            pool_code_id,
        } => execute::create_pair(deps, info, pool_init_msg, pool_code_id),
        ExecuteMsg::RegisterStableAsset { stable_denom } => {
            execute::register_stable_denom(deps, info, stable_denom)
        }
        ExecuteMsg::AllPoolLock {} => execute::all_pool_lock(deps, info),
        ExecuteMsg::PairLock {
            asset_denom,
            stable_denom,
        } => execute::pair_pool_lock(deps, info, asset_denom, stable_denom),
        ExecuteMsg::AllPoolUnLock {} => execute::all_pool_unlock(deps, info),
        ExecuteMsg::PairUnLock {
            asset_denom,
            stable_denom,
        } => execute::pair_pool_un_lock(deps, info, asset_denom, stable_denom),
    }
}

pub mod execute {
    use cosmwasm_std::{CosmosMsg, Order, SubMsg, WasmMsg};

    use sei_cosmwasm::{SeiMsg, SeiQueryWrapper};

    use crate::{
        helpers::{check_denom_and_get_validate_denom, check_owner, check_valid_stable},
        state::{check_pair, load_config, load_pair, save_config, PAIR_POOL},
    };

    use super::*;

    pub fn create_pair(
        deps: DepsMut<SeiQueryWrapper>,
        info: MessageInfo,
        msg: PoolInstantiateMsg,
        pool_code_id: u64,
    ) -> Result<Response<SeiMsg>, ContractError> {
        //stable denom 이 있는지 확인
        let config = load_config(deps.storage)?;
        // info.funds 로 확인해야함??
        check_owner(&info.sender, &config.owner)?;
        //허용된 stable denom 인지 확인

        check_valid_stable(&config, &msg.stable_denom)?;
        //pair 이미 있는지 확인해야함.
        check_pair(deps.storage, &msg.asset_denom, &msg.stable_denom)?;
        //denom 에 맞는 asset 구하는 로직
        let (asset_coin, stable_coin) =
            check_denom_and_get_validate_denom(info.funds, &msg.asset_denom, &msg.stable_denom)?;

        //pool contract 배포
        let init_pool_msg = CosmosMsg::Wasm(WasmMsg::Instantiate {
            admin: Some(info.sender.to_string()),
            code_id: pool_code_id,
            msg: to_binary(&msg)?,
            label: format!("{:?}:{:?}", asset_coin.denom, stable_coin.denom),
            funds: vec![asset_coin, stable_coin],
        });
        let init_pool_tx = SubMsg::reply_on_success(init_pool_msg, 2);

        Ok(Response::new()
            .add_attribute("method", "create_pair")
            .add_submessage(init_pool_tx))
    }
    pub fn register_stable_denom(
        deps: DepsMut<SeiQueryWrapper>,
        info: MessageInfo,
        stable_denom: String,
    ) -> Result<Response<SeiMsg>, ContractError> {
        let mut config = load_config(deps.storage)?;
        //only owner
        check_owner(&info.sender, &config.owner)?;
        //이미 있는지 확인
        match config.accept_stable_denoms.contains(&stable_denom) {
            true => Err(ContractError::InvalidStable {}),
            false => Ok(()),
        }?;
        config.accept_stable_denoms.push(stable_denom);
        save_config(deps.storage, &config)?;
        Ok(Response::new())
    }

    pub fn all_pool_lock(
        deps: DepsMut<SeiQueryWrapper>,
        info: MessageInfo,
    ) -> Result<Response<SeiMsg>, ContractError> {
        let config = load_config(deps.storage)?;
        check_owner(&info.sender, &config.owner)?;
        let pair_lock_msgs = PAIR_POOL
            .range(deps.storage, None, None, Order::Ascending)
            .map(|item| {
                let (_, pair_addr) = item?;
                let pool_lock_msg = PoolExecuteMsg::Lock {};

                Ok(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: pair_addr.to_string(),
                    msg: to_binary(&pool_lock_msg)?,
                    funds: vec![],
                }))
            })
            .collect::<StdResult<Vec<CosmosMsg<SeiMsg>>>>()?;

        Ok(Response::new().add_messages(pair_lock_msgs))
    }
    pub fn all_pool_unlock(
        deps: DepsMut<SeiQueryWrapper>,
        info: MessageInfo,
    ) -> Result<Response<SeiMsg>, ContractError> {
        let config = load_config(deps.storage)?;
        check_owner(&info.sender, &config.owner)?;
        let pair_lock_msgs = PAIR_POOL
            .range(deps.storage, None, None, Order::Ascending)
            .map(|item| {
                let (_, pair_addr) = item?;
                let pool_lock_msg = PoolExecuteMsg::UnLock {};

                Ok(CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: pair_addr.to_string(),
                    msg: to_binary(&pool_lock_msg)?,
                    funds: vec![],
                }))
            })
            .collect::<StdResult<Vec<CosmosMsg<SeiMsg>>>>()?;

        Ok(Response::new().add_messages(pair_lock_msgs))
    }

    pub fn pair_pool_lock(
        deps: DepsMut<SeiQueryWrapper>,
        info: MessageInfo,
        asset_denom: String,
        stable_denom: String,
    ) -> Result<Response<SeiMsg>, ContractError> {
        let config = load_config(deps.storage)?;
        check_owner(&info.sender, &config.owner)?;
        let pair_addr = load_pair(deps.storage, &asset_denom, &stable_denom)?;
        let pool_lock_msg = PoolExecuteMsg::Lock {};
        let pool_lock_tx = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: pair_addr.to_string(),
            msg: to_binary(&pool_lock_msg)?,
            funds: vec![],
        });
        Ok(Response::new().add_message(pool_lock_tx))
    }
    pub fn pair_pool_un_lock(
        deps: DepsMut<SeiQueryWrapper>,
        info: MessageInfo,
        asset_denom: String,
        stable_denom: String,
    ) -> Result<Response<SeiMsg>, ContractError> {
        let config = load_config(deps.storage)?;
        check_owner(&info.sender, &config.owner)?;
        let pair_addr = load_pair(deps.storage, &asset_denom, &stable_denom)?;
        let pool_unlock_msg = PoolExecuteMsg::UnLock {};
        let pool_lock_tx = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: pair_addr.to_string(),
            msg: to_binary(&pool_unlock_msg)?,
            funds: vec![],
        });
        Ok(Response::new().add_message(pool_lock_tx))
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps<SeiQueryWrapper>, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetPairContract {
            asset_denom,
            stable_denom,
        } => to_binary(&query::get_pair_contract(deps, asset_denom, stable_denom)?),
        QueryMsg::GetConfig {} => to_binary(&query::get_config(deps)?),
    }
}

pub mod query {
    use axis::core::PairContractResponse;

    use crate::state::{load_config, load_pair};

    use super::*;

    pub fn get_pair_contract(
        deps: Deps<SeiQueryWrapper>,
        asset_denom: String,
        stable_denom: String,
    ) -> StdResult<PairContractResponse> {
        let pool_contract = load_pair(deps.storage, &asset_denom, &stable_denom)?.to_string();
        Ok(PairContractResponse {
            asset_denom,
            stable_denom,
            pool_contract,
        })
    }

    pub fn get_config(deps: Deps<SeiQueryWrapper>) -> StdResult<ConfigResponse> {
        let config = load_config(deps.storage)?;
        Ok(ConfigResponse {
            accept_stable_denoms: config.accept_stable_denoms,
            owner: config.owner.to_string(),
            treausry_contract_address: config.treausry_contract_address.to_string(),
        })
    }
}
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(
    deps: DepsMut<SeiQueryWrapper>,
    _env: Env,
    msg: Reply,
) -> Result<Response<SeiMsg>, ContractError> {
    match msg.id {
        1 => match msg.result {
            SubMsgResult::Ok(res) => match res.data {
                Some(_) => {
                    let traesury_contract_addr =
                        find_attribute_value(&res.events[1].attributes, "contract_address")?;
                    register_treasury(deps.storage, Addr::unchecked(traesury_contract_addr))?;
                    Ok(Response::new())
                }
                None => Err(ContractError::MissingPoolContractAddr {}),
            },
            SubMsgResult::Err(_) => Err(ContractError::TreasuryContractInstantiationFailed {}),
        },
        2 => match msg.result {
            SubMsgResult::Ok(res) => match res.data {
                Some(_) => {
                    let pool_contract_addr =
                        find_attribute_value(&res.events[1].attributes, "contract_address")?;
                    let asset_denom =
                        find_attribute_value(&res.events[1].attributes, "asset_denom")?;
                    let stable_denom =
                        find_attribute_value(&res.events[1].attributes, "stable_denom")?;

                    register_pair(
                        deps.storage,
                        asset_denom,
                        stable_denom,
                        Addr::unchecked(pool_contract_addr),
                    )?;
                    Ok(Response::new())
                }
                None => Err(ContractError::MissingPoolContractAddr {}),
            },
            SubMsgResult::Err(_) => Err(ContractError::PoolContractInstantiationFailed {}),
        },
        _ => Err(ContractError::InvalidReplyId {}),
    }
}
