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
use crate::state::{
    load_config, register_axis_contract, register_pair_pool_contract, Config, CONFIG,
    PAIR_MARKET_CONTRACT, PAIR_POOL, PAIR_POOL_LP_STAKING_CONTRACT,
};
use axis_protocol::axis::InstantiateMsg as AxisInstantiateMsg;
use axis_protocol::core::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg};
use axis_protocol::pool::{
    ConfigResponse as PoolConfigReponse, ExecuteMsg as PoolExecuteMsg,
    InstantiateMsg as PoolInstantiateMsg, QueryMsg as PoolQueryMsg,
};

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
        epoch: 0,
        accept_price_denoms: msg.accept_price_denoms,
        axis_contract: Addr::unchecked(""),
        staking_contract: Addr::unchecked(""),
        vault_contract: Addr::unchecked(""),
    };
    CONFIG.save(deps.storage, &config)?;
    //@@axis instantiate
    let axis_instantiate_msg = WasmMsg::Instantiate {
        admin: Some(info.sender.to_string()),
        code_id: msg.axis_code_id,
        msg: to_binary(&AxisInstantiateMsg {
            core_contract: env.contract.address,
            owner: info.sender.to_owned(),
        })?,
        funds: vec![],
        label: format!("axis"),
    };

    let axis_instatiate_tx = SubMsg::reply_on_success(axis_instantiate_msg, 1);
    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender)
        .add_submessage(axis_instatiate_tx))
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
        ExecuteMsg::RegisterPriceDenom { price_denom } => {
            execute::register_price_denom(deps, info, price_denom)
        }
        ExecuteMsg::AllPoolLock {} => execute::all_pool_lock(deps, info),
        ExecuteMsg::PairLock {
            base_denom,
            price_denom,
        } => execute::pair_pool_lock(deps, info, base_denom, price_denom),
        ExecuteMsg::AllPoolUnLock {} => execute::all_pool_unlock(deps, info),
        ExecuteMsg::PairUnLock {
            base_denom,
            price_denom,
        } => execute::pair_pool_un_lock(deps, info, base_denom, price_denom),
        ExecuteMsg::Setting {} => execute::setting(deps, info),
        ExecuteMsg::UpdateConfig {
            vault_contract,
            staking_contract,
        } => execute::update_config(deps, info, vault_contract, staking_contract),
    }
}

pub mod execute {
    use axis_protocol::{
        axis::ExecuteMsg as AxisExecuteMsg, staking::ExecuteMsg as StakingExecuteMsg,
        vault::ExecuteMsg as VaultExecuteMsg,
    };
    use cosmwasm_std::{CosmosMsg, Order, SubMsg, WasmMsg};

    use sei_cosmwasm::{SeiMsg, SeiQueryWrapper};

    use crate::{
        helpers::{check_denom_and_get_validate_denom, check_owner, check_valid_price},
        state::{check_pair, load_config, load_pair, save_config, PAIR_POOL},
    };

    use super::*;

    pub fn create_pair(
        deps: DepsMut<SeiQueryWrapper>,
        info: MessageInfo,
        msg: PoolInstantiateMsg,
        pool_code_id: u64,
    ) -> Result<Response<SeiMsg>, ContractError> {
        //price denom 이 있는지 확인
        let config = load_config(deps.storage)?;
        // info.funds 로 확인해야함??
        check_owner(&info.sender, &config.owner)?;
        //허용된 price denom 인지 확인

        check_valid_price(&config, &msg.price_denom)?;
        //pair 이미 있는지 확인해야함.
        check_pair(deps.storage, &msg.base_denom, &msg.price_denom)?;
        //denom 에 맞는 base 구하는 로직
        let (base_coin, price_coin) =
            check_denom_and_get_validate_denom(info.funds, &msg.base_denom, &msg.price_denom)?;

        //pool contract 배포
        let init_pool_msg = CosmosMsg::Wasm(WasmMsg::Instantiate {
            admin: Some(info.sender.to_string()),
            code_id: pool_code_id,
            msg: to_binary(&msg)?,
            label: format!("{:?}:{:?}", base_coin.denom, price_coin.denom),
            funds: vec![base_coin, price_coin],
        });
        let init_pool_tx = SubMsg::reply_on_success(init_pool_msg, 2);

        Ok(Response::new()
            .add_attribute("method", "create_pair")
            .add_submessage(init_pool_tx))
    }
    pub fn register_price_denom(
        deps: DepsMut<SeiQueryWrapper>,
        info: MessageInfo,
        price_denom: String,
    ) -> Result<Response<SeiMsg>, ContractError> {
        let mut config = load_config(deps.storage)?;
        //only owner
        check_owner(&info.sender, &config.owner)?;
        //이미 있는지 확인
        match config.accept_price_denoms.contains(&price_denom) {
            true => Err(ContractError::InvalidPrice {}),
            false => Ok(()),
        }?;
        config.accept_price_denoms.push(price_denom);
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
        base_denom: String,
        price_denom: String,
    ) -> Result<Response<SeiMsg>, ContractError> {
        let config = load_config(deps.storage)?;
        check_owner(&info.sender, &config.owner)?;
        let pair_addr = load_pair(deps.storage, &base_denom, &price_denom)?;
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
        base_denom: String,
        price_denom: String,
    ) -> Result<Response<SeiMsg>, ContractError> {
        let config = load_config(deps.storage)?;
        check_owner(&info.sender, &config.owner)?;
        let pair_addr = load_pair(deps.storage, &base_denom, &price_denom)?;
        let pool_unlock_msg = PoolExecuteMsg::UnLock {};
        let pool_lock_tx = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: pair_addr.to_string(),
            msg: to_binary(&pool_unlock_msg)?,
            funds: vec![],
        });
        Ok(Response::new().add_message(pool_lock_tx))
    }
    pub fn setting(
        deps: DepsMut<SeiQueryWrapper>,
        info: MessageInfo,
    ) -> Result<Response<SeiMsg>, ContractError> {
        //Axis token setting
        //staking setting
        //vault setting
        //@@checking timestamp
        let mut config = load_config(deps.storage)?;

        let axis_setting_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: config.axis_contract.to_string(),
            msg: to_binary(&AxisExecuteMsg::Setting {})?,
            funds: vec![],
        });
        let vault_setting_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: config.vault_contract.to_string(),
            msg: to_binary(&VaultExecuteMsg::Setting {})?,
            funds: vec![],
        });
        let axis_staking_setting_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: config.staking_contract.to_string(),
            msg: to_binary(&StakingExecuteMsg::Setting {})?,
            funds: vec![],
        });
        config.epoch += 1;
        save_config(deps.storage, &config)?;
        Ok(Response::new().add_messages(vec![
            axis_setting_msg,
            vault_setting_msg,
            axis_staking_setting_msg,
        ]))
    }
    pub fn update_config(
        deps: DepsMut<SeiQueryWrapper>,
        info: MessageInfo,
        vault_contract: Option<String>,
        staking_contract: Option<String>,
    ) -> Result<Response<SeiMsg>, ContractError> {
        let config = load_config(deps.storage)?;
        check_owner(&info.sender, &config.owner)?;
        let mut config = load_config(deps.storage)?;
        if let Some(staking_contract) = staking_contract {
            if config.staking_contract == Addr::unchecked("") {
                config.staking_contract = deps.api.addr_validate(&staking_contract)?;
            }
        }
        if let Some(vault_contract) = vault_contract {
            if config.vault_contract == Addr::unchecked("") {
                config.vault_contract = deps.api.addr_validate(&vault_contract)?;
            }
        }
        Ok(Response::new())
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps<SeiQueryWrapper>, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetConfig {} => to_binary(&query::get_config(deps)?),
        QueryMsg::GetPairPoolContract {
            base_denom,
            price_denom,
        } => to_binary(&query::get_pair_pool_contract(
            deps,
            base_denom,
            price_denom,
        )?),
        QueryMsg::GetPairLpStakingContract {
            base_denom,
            price_denom,
        } => to_binary(&query::get_pair_lp_staking_contract(
            deps,
            base_denom,
            price_denom,
        )?),
        QueryMsg::GetPairMarketContract {
            base_denom,
            price_denom,
        } => to_binary(&query::get_pair_market_contract(
            deps,
            base_denom,
            price_denom,
        )?),
    }
}

pub mod query {
    use axis_protocol::core::{
        PairLpStakingContractResponse, PairMarketContractResponse, PairPoolContractResponse,
    };

    use crate::state::{
        load_config, load_pair, PAIR_MARKET_CONTRACT, PAIR_POOL_LP_STAKING_CONTRACT,
    };

    use super::*;

    pub fn get_pair_pool_contract(
        deps: Deps<SeiQueryWrapper>,
        base_denom: String,
        price_denom: String,
    ) -> StdResult<PairPoolContractResponse> {
        let pool_contract = load_pair(deps.storage, &base_denom, &price_denom)?;

        Ok(PairPoolContractResponse {
            base_denom,
            price_denom,
            pool_contract,
        })
    }
    pub fn get_pair_market_contract(
        deps: Deps<SeiQueryWrapper>,
        base_denom: String,
        price_denom: String,
    ) -> StdResult<PairMarketContractResponse> {
        let market_contract =
            PAIR_MARKET_CONTRACT.load(deps.storage, (&base_denom, &price_denom))?;
        Ok(PairMarketContractResponse {
            base_denom,
            price_denom,
            market_contract,
        })
    }
    pub fn get_pair_lp_staking_contract(
        deps: Deps<SeiQueryWrapper>,
        base_denom: String,
        price_denom: String,
    ) -> StdResult<PairLpStakingContractResponse> {
        let lp_staking_contract =
            PAIR_POOL_LP_STAKING_CONTRACT.load(deps.storage, (&base_denom, &price_denom))?;
        Ok(PairLpStakingContractResponse {
            base_denom,
            price_denom,
            lp_staking_contract,
        })
    }

    pub fn get_config(deps: Deps<SeiQueryWrapper>) -> StdResult<ConfigResponse> {
        let config = load_config(deps.storage)?;
        Ok(ConfigResponse {
            epoch: config.epoch,
            accept_price_denoms: config.accept_price_denoms,
            axis_contract: config.axis_contract,
            owner: config.owner,
            staking_contract: config.staking_contract,
            vault_contract: config.vault_contract,
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
                Some(data) => {
                    let addr = String::from_utf8(data.to_vec()).unwrap();
                    let axis_contract_addr = deps.api.addr_validate(addr.trim())?;
                    register_axis_contract(deps.storage, axis_contract_addr)?;

                    Ok(Response::new())
                }
                None => Err(ContractError::MissingAxisContractAddr {}),
            },
            SubMsgResult::Err(_) => Err(ContractError::AxisContractInstantiationFailed {}),
        },
        2 => match msg.result {
            SubMsgResult::Ok(res) => match res.data {
                Some(data) => {
                    let addr = String::from_utf8(data.to_vec()).unwrap();
                    let pool_contract_addr = deps.api.addr_validate(addr.trim())?;
                    let base_denom = find_attribute_value(&res.events[1].attributes, "base_denom")?;
                    let price_denom =
                        find_attribute_value(&res.events[1].attributes, "price_denom")?;

                    let key = (&base_denom, &price_denom);

                    PAIR_POOL.save(deps.storage, key, &pool_contract_addr)?;

                    let pool_config: PoolConfigReponse = deps
                        .querier
                        .query_wasm_smart(pool_contract_addr, &PoolQueryMsg::GetConfig {})?;

                    PAIR_MARKET_CONTRACT.save(deps.storage, key, &pool_config.market_contract)?;
                    PAIR_POOL_LP_STAKING_CONTRACT.save(
                        deps.storage,
                        key,
                        &pool_config.lp_staking_contract,
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
