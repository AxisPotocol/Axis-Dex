#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdResult, SubMsg,
    SubMsgResult, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use sei_cosmwasm::{SeiMsg, SeiQueryWrapper};

use crate::error::ContractError;
use crate::helpers::{find_attribute_value, register_es_axis};
use crate::query::query_epoch;
use crate::state::{save_config, save_state, Config, State, CONFIG};
use axis_protocol::es_axis::{
    ExecuteMsg as ESAxisExecuteMsg, InstantiateMsg as ESAxisInstantiateMsg,
};
use axis_protocol::staking::{ExecuteMsg, InstantiateMsg, QueryMsg};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:staking";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub const ONE_DAY_PER_MINT: u128 = 1_000_000_000_000_000;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut<SeiQueryWrapper>,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response<SeiMsg>, ContractError> {
    let config = Config {
        core_contract: msg.core_contract,
        axis_denom: msg.axis_denom,
        es_axis_contract: Addr::unchecked(""),
    };

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    save_config(deps.storage, &config)?;
    let state = State {
        pending_staking_total: Uint128::zero(),
        staking_total: Uint128::zero(),
        withdraw_pending_total: Uint128::zero(),
    };
    save_state(deps.storage, &state)?;
    // Instantiate
    let es_axis_instantiate_msg = SubMsg::reply_on_success(
        WasmMsg::Instantiate {
            admin: None,
            code_id: msg.es_axis_code,
            msg: to_binary(&ESAxisInstantiateMsg {})?,
            funds: vec![],
            label: "es-axis-contract".to_string(),
        },
        1,
    );
    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender)
        .add_submessage(es_axis_instantiate_msg))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut<SeiQueryWrapper>,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<SeiMsg>, ContractError> {
    use execute::*;
    match msg {
        ExecuteMsg::Setting {} => setting(deps, info),
        ExecuteMsg::Staking {} => staking(deps, info),
        ExecuteMsg::UnStaking {} => un_staking(deps, info),
        ExecuteMsg::Withdraw {} => withdraw(deps, info),
        ExecuteMsg::ClaimReward {} => claim_reward(deps, info),
    }
}

pub mod execute {

    use cosmwasm_std::{coin, BankMsg, CosmosMsg, SubMsg, WasmMsg};
    use sei_cosmwasm::{SeiMsg, SeiQueryWrapper};

    use crate::{
        helpers::{check_core_contract, check_funds_and_get_axis, compute_mint_amount},
        state::{
            load_config, load_stakings, load_state, load_un_stakings, save_config, save_state,
            StakeInfo, UnStakeInfo, EPOCH_STAKING_AMOUNT, STAKING, UN_STAKING,
        },
    };

    use super::*;
    pub fn staking(
        deps: DepsMut<SeiQueryWrapper>,
        info: MessageInfo,
    ) -> Result<Response<SeiMsg>, ContractError> {
        let config = load_config(deps.storage)?;
        let mut state = load_state(deps.storage)?;
        let axis_coin = check_funds_and_get_axis(info.funds, &config.axis_denom)?;
        let epoch = query_epoch(deps.querier, &config.core_contract)?;
        STAKING.update(
            deps.storage,
            info.sender,
            |exsits| -> StdResult<Vec<StakeInfo>> {
                match exsits {
                    Some(mut stakings) => {
                        if let Some(stake) = stakings.iter_mut().find(|s| s.start_epoch == epoch) {
                            stake.staking_amount += axis_coin.amount;
                        } else {
                            stakings.push(StakeInfo {
                                start_epoch: epoch + 1,
                                staking_amount: axis_coin.amount,
                            })
                        }
                        Ok(stakings)
                    }
                    None => {
                        let stakings = vec![StakeInfo {
                            start_epoch: epoch + 1,
                            staking_amount: axis_coin.amount,
                        }];
                        Ok(stakings)
                    }
                }
            },
        )?;
        state.pending_staking_total += axis_coin.amount;
        save_state(deps.storage, &state)?;
        Ok(Response::new())
    }

    pub fn un_staking(
        deps: DepsMut<SeiQueryWrapper>,
        info: MessageInfo,
    ) -> Result<Response<SeiMsg>, ContractError> {
        let config = load_config(deps.storage)?;
        let mut state = load_state(deps.storage)?;
        let epoch = query_epoch(deps.querier, &config.core_contract)?;
        let stakings = load_stakings(deps.storage, info.sender.clone())?;

        let unlock_epoch = epoch + 1;

        let mut withdraw_pending_amount = Uint128::zero();

        let mut ex_axis_amount = Uint128::zero();

        for stake in stakings.into_iter() {
            ex_axis_amount +=
                compute_mint_amount(deps.storage, stake.staking_amount, stake.start_epoch, epoch)?;
            withdraw_pending_amount += stake.staking_amount;
        }

        let un_stake = UnStakeInfo {
            unlock_epoch,
            withdraw_pending_amount,
        };

        STAKING.remove(deps.storage, info.sender.to_owned());

        UN_STAKING.update(
            deps.storage,
            info.sender.to_owned(),
            |exsists| -> StdResult<_> {
                match exsists {
                    Some(mut unstake) => {
                        unstake.push(un_stake);
                        Ok(unstake)
                    }
                    None => {
                        let new_un_stakes = vec![un_stake];
                        Ok(new_un_stakes)
                    }
                }
            },
        )?;

        state.staking_total -= withdraw_pending_amount;

        state.withdraw_pending_total += withdraw_pending_amount;

        save_state(deps.storage, &state)?;

        let claim_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: config.es_axis_contract.to_string(),
            msg: to_binary(&ESAxisExecuteMsg::Claim {
                sender: info.sender,
                amount: ex_axis_amount,
            })?,
            funds: vec![],
        });
        Ok(Response::new().add_message(claim_msg))
    }

    pub fn withdraw(
        deps: DepsMut<SeiQueryWrapper>,
        info: MessageInfo,
    ) -> Result<Response<SeiMsg>, ContractError> {
        let config = load_config(deps.storage)?;
        let mut state = load_state(deps.storage)?;
        let un_stakings = load_un_stakings(deps.storage, info.sender.clone())?;
        let epoch = query_epoch(deps.querier, &config.core_contract)?;
        let axis_amount: Uint128 = un_stakings
            .iter()
            .filter(|stake| stake.unlock_epoch < epoch)
            .fold(Uint128::zero(), |axis_amount, stake| {
                axis_amount + stake.withdraw_pending_amount
            });

        let remaining_un_stakings = un_stakings
            .into_iter()
            .filter(|un_stake| un_stake.unlock_epoch >= epoch)
            .collect::<Vec<UnStakeInfo>>();
        match remaining_un_stakings.len() {
            0 => UN_STAKING.remove(deps.storage, info.sender.clone()),
            _ => UN_STAKING.save(deps.storage, info.sender.clone(), &remaining_un_stakings)?,
        }

        state.withdraw_pending_total -= axis_amount;
        save_state(deps.storage, &state)?;

        let axis_token = coin(axis_amount.into(), config.axis_denom);

        let axis_send_msg = SubMsg::new(BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: vec![axis_token],
        });
        //axis_token 은 보내주고, es_axis 는 민트 wasm

        Ok(Response::new().add_submessage(axis_send_msg))
    }

    pub fn claim_reward(
        deps: DepsMut<SeiQueryWrapper>,
        info: MessageInfo,
    ) -> Result<Response<SeiMsg>, ContractError> {
        let config = load_config(deps.storage)?;
        let mut stakings = load_stakings(deps.storage, info.sender.clone())?;
        let mut claim_es_axis_amount = Uint128::zero();
        let epoch = query_epoch(deps.querier, &config.core_contract)?;
        for stake in stakings.iter_mut() {
            if stake.start_epoch == epoch {
                continue;
            }

            let reward =
                compute_mint_amount(deps.storage, stake.staking_amount, stake.start_epoch, epoch)?;
            stake.start_epoch = epoch;
            claim_es_axis_amount += reward;
        }

        let wasm_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: config.es_axis_contract.to_string(),
            msg: to_binary(&ESAxisExecuteMsg::Claim {
                sender: info.sender,
                amount: claim_es_axis_amount,
            })?,
            funds: vec![],
        });
        Ok(Response::new().add_message(wasm_msg))
    }

    pub fn setting(
        deps: DepsMut<SeiQueryWrapper>,
        info: MessageInfo,
    ) -> Result<Response<SeiMsg>, ContractError> {
        let config = load_config(deps.storage)?;
        let mut state = load_state(deps.storage)?;
        check_core_contract(&config.core_contract, &info.sender)?;
        let epoch = query_epoch(deps.querier, &config.core_contract)? - 1;
        EPOCH_STAKING_AMOUNT.save(deps.storage, epoch, &state.staking_total)?;

        state.staking_total += state.pending_staking_total;
        state.pending_staking_total = Uint128::zero();
        save_state(deps.storage, &state)?;
        //mint msg
        let wasm_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: config.es_axis_contract.to_string(),
            msg: to_binary(&ESAxisExecuteMsg::Mint {
                amount: ONE_DAY_PER_MINT.into(),
            })?,
            funds: vec![],
        });
        Ok(Response::new().add_message(wasm_msg))
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps<SeiQueryWrapper>, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetConfig {} => to_binary(&query::get_config(deps)?),
        QueryMsg::GetState {} => to_binary(&query::get_state(deps)?),
        QueryMsg::GetStakeInfo { addr } => to_binary(&query::get_stake_info(deps, addr)?),
        QueryMsg::GetUnStakeInfo { addr } => to_binary(&query::get_unstake_info(deps, addr)?),
        QueryMsg::GetAvailableReward { addr } => {
            to_binary(&query::get_available_claim_reward(deps, addr)?)
        }
    }
}

pub mod query {
    use axis_protocol::staking::{
        ConfigResponse, StakeInfoResponse, StakeResponse, StateResponse, UnStakeInfoResponse,
        UnStakeResponse,
    };

    use crate::{
        helpers::compute_mint_amount,
        state::{load_config, load_stakings, load_state, load_un_stakings, StakeInfo, UnStakeInfo},
    };

    use super::*;

    pub fn get_config(deps: Deps<SeiQueryWrapper>) -> StdResult<ConfigResponse> {
        let config = load_config(deps.storage)?;
        Ok(ConfigResponse {
            core_contract: config.core_contract,
            axis_denom: config.axis_denom,
            es_axis_contract: config.es_axis_contract,
        })
    }
    pub fn get_state(deps: Deps<SeiQueryWrapper>) -> StdResult<StateResponse> {
        let state = load_state(deps.storage)?;
        Ok(StateResponse {
            pending_staking_total: state.pending_staking_total,
            withdraw_pending_total: state.withdraw_pending_total,
            staking_total: state.staking_total,
        })
    }
    pub fn get_stake_info(
        deps: Deps<SeiQueryWrapper>,
        addr: String,
    ) -> StdResult<StakeInfoResponse> {
        let staker = deps.api.addr_validate(&addr)?;
        let stakings = load_stakings(deps.storage, staker)?;
        let stakings_response = stakings
            .into_iter()
            .map(|stake| StakeResponse {
                start_epoch: stake.start_epoch,
                staking_amount: stake.staking_amount,
            })
            .collect();
        Ok(StakeInfoResponse {
            stake_infos: stakings_response,
        })
    }
    pub fn get_unstake_info(
        deps: Deps<SeiQueryWrapper>,
        addr: String,
    ) -> StdResult<UnStakeInfoResponse> {
        let un_staker = deps.api.addr_validate(&addr)?;
        let un_stakings = load_un_stakings(deps.storage, un_staker)?;
        let un_stakings_response = un_stakings
            .into_iter()
            .map(|un_stake| UnStakeResponse {
                unlock_epoch: un_stake.unlock_epoch,
                withdraw_pending_amount: un_stake.withdraw_pending_amount,
            })
            .collect();
        Ok(UnStakeInfoResponse {
            un_stake_infos: un_stakings_response,
        })
    }
    pub fn get_available_claim_reward(
        deps: Deps<SeiQueryWrapper>,
        addr: String,
    ) -> StdResult<Uint128> {
        let config = load_config(deps.storage)?;
        let epoch = query_epoch(deps.querier, &config.core_contract)?;
        let staker = deps.api.addr_validate(&addr)?;
        let stakings = load_stakings(deps.storage, staker)?;
        let ex_axis_amount: Uint128 = stakings
            .iter()
            .map(|stake| {
                compute_mint_amount(deps.storage, stake.staking_amount, stake.start_epoch, epoch)
            })
            .sum::<StdResult<Uint128>>()?;

        Ok(ex_axis_amount)
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
                    let es_axis_contract_addr =
                        find_attribute_value(&res.events[1].attributes, "contract_address")?;
                    register_es_axis(
                        deps.storage,
                        deps.api.addr_validate(&es_axis_contract_addr)?,
                    )?;
                    Ok(Response::new())
                }
                None => Err(ContractError::MissingEsAxisContractAddr {}),
            },
            SubMsgResult::Err(_) => Err(ContractError::ESAxisContractInstantiateFailed {}),
        },

        _ => Err(ContractError::InvalidReplyId {}),
    }
}
