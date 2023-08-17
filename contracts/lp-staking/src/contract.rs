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
    state::{save_config, save_state, Config, State},
};

use axis_protocol::{
    lp_staking::{ExecuteMsg, InstantiateMsg, QueryMsg},
    query::query_epoch,
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:lp-staking";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut<SeiQueryWrapper>,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response<SeiMsg>, ContractError> {
    let epoch = query_epoch(deps.querier, &msg.core_contract)?;
    let config = Config {
        lp_denom: msg.lp_denom,
        axis_contract: msg.axis_contract,
        core_contract: msg.core_contract,
        base_denom: msg.base_denom,
        price_denom: msg.price_denom,
    };
    save_config(deps.storage, config)?;

    let state = State {
        epoch,
        staking_total: Uint128::zero(),
        stake_pending_total: Uint128::zero(),
        withdraw_pending_total: Uint128::zero(),
    };
    save_state(deps.storage, state)?;
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
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
        ExecuteMsg::Staking {} => staking(deps, info),
        ExecuteMsg::UnStaking {} => un_staking(deps, info),
        ExecuteMsg::Withdraw {} => withdraw(deps, info),
        ExecuteMsg::ClaimReward {} => claim_reward(deps, info),
        ExecuteMsg::Setting { epoch } => setting(deps, info, epoch),
    }
}

mod execute {
    use crate::{
        helpers::{check_core_contract, check_funds_and_get_lp, compute_mint_amount},
        state::{
            load_config, load_stakings, load_state, load_un_stakings, save_state, StakeInfo,
            UnStakeInfo, EPOCH_STAKING_TOTAL_AMOUNT, STAKING, UN_STAKING,
        },
        ContractError,
    };
    use axis_protocol::axis::ExecuteMsg as AxisExecuteMsg;
    use cosmwasm_std::{
        coin, to_binary, BankMsg, CosmosMsg, DepsMut, MessageInfo, Response, StdResult, SubMsg,
        Uint128, WasmMsg,
    };
    use sei_cosmwasm::{SeiMsg, SeiQueryWrapper};

    pub fn staking(
        deps: DepsMut<SeiQueryWrapper>,
        info: MessageInfo,
    ) -> Result<Response<SeiMsg>, ContractError> {
        let config = load_config(deps.storage)?;

        let lp_coin = check_funds_and_get_lp(info.funds, &config.lp_denom)?;

        let mut state = load_state(deps.storage)?;
        STAKING.update(
            deps.storage,
            &info.sender,
            |exsits| -> StdResult<Vec<StakeInfo>> {
                match exsits {
                    Some(mut stakings) => {
                        if let Some(stake) = stakings
                            .iter_mut()
                            .find(|s| s.start_epoch == state.epoch + 1)
                        {
                            stake.staking_amount += lp_coin.amount;
                        } else {
                            stakings.push(StakeInfo {
                                start_epoch: state.epoch + 1,
                                staking_amount: lp_coin.amount,
                            })
                        }
                        Ok(stakings)
                    }
                    None => {
                        let stakings = vec![StakeInfo {
                            start_epoch: state.epoch + 1,
                            staking_amount: lp_coin.amount,
                        }];
                        Ok(stakings)
                    }
                }
            },
        )?;

        state.stake_pending_total += lp_coin.amount;
        save_state(deps.storage, state)?;

        Ok(Response::default())
    }

    pub fn un_staking(
        deps: DepsMut<SeiQueryWrapper>,
        info: MessageInfo,
    ) -> Result<Response<SeiMsg>, ContractError> {
        let config = load_config(deps.storage)?;
        let mut state = load_state(deps.storage)?;
        let stakings = load_stakings(deps.storage, &info.sender)?
            .into_iter()
            .filter(|s| s.start_epoch <= state.epoch)
            .collect::<Vec<StakeInfo>>();

        let unlock_epoch = state.epoch + 1;

        let mut axis_amount = Uint128::zero();
        let mut unstaking_amount = Uint128::zero();
        for stake in stakings.into_iter() {
            axis_amount += compute_mint_amount(
                &deps,
                &config,
                stake.staking_amount,
                stake.start_epoch,
                state.epoch,
            )?;
            unstaking_amount += stake.staking_amount;
        }
        let un_stake = UnStakeInfo {
            unlock_epoch,
            unstaking_amount,
        };
        STAKING.remove(deps.storage, &info.sender);

        UN_STAKING.update(deps.storage, &info.sender, |exsists| -> StdResult<_> {
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
        })?;

        state.withdraw_pending_total += unstaking_amount;
        state.staking_total -= unstaking_amount;
        save_state(deps.storage, state)?;
        match axis_amount.is_zero() {
            true => Ok(Response::new()),
            false => {
                let claim_msg = CosmosMsg::Wasm(WasmMsg::Execute {
                    contract_addr: config.axis_contract.to_string(),
                    msg: to_binary(&AxisExecuteMsg::ClaimMintMaker {
                        base_denom: config.base_denom,
                        price_denom: config.price_denom,
                        sender: info.sender,
                        amount: axis_amount,
                    })?,
                    funds: vec![],
                });
                Ok(Response::new().add_message(claim_msg))
            }
        }
    }

    pub fn withdraw(
        deps: DepsMut<SeiQueryWrapper>,
        info: MessageInfo,
    ) -> Result<Response<SeiMsg>, ContractError> {
        let config = load_config(deps.storage)?;

        let un_stakings = load_un_stakings(deps.storage, &info.sender)?;
        let mut state = load_state(deps.storage)?;

        let lp_amount: Uint128 = un_stakings
            .iter()
            .filter(|stake| stake.unlock_epoch < state.epoch)
            .fold(Uint128::zero(), |lp_amount, stake| {
                lp_amount + stake.unstaking_amount
            });

        let remaining_un_stakings = un_stakings
            .into_iter()
            .filter(|un_stake| un_stake.unlock_epoch >= state.epoch)
            .collect::<Vec<UnStakeInfo>>();

        match remaining_un_stakings.len() {
            0 => UN_STAKING.remove(deps.storage, &info.sender),
            _ => UN_STAKING.save(deps.storage, &info.sender, &remaining_un_stakings)?,
        }

        state.withdraw_pending_total -= lp_amount;
        save_state(deps.storage, state)?;
        let lp_token = coin(lp_amount.into(), config.lp_denom);

        let axis_send_msg = SubMsg::new(BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: vec![lp_token],
        });
        Ok(Response::new().add_submessage(axis_send_msg))
    }

    pub fn claim_reward(
        deps: DepsMut<SeiQueryWrapper>,
        info: MessageInfo,
    ) -> Result<Response<SeiMsg>, ContractError> {
        let config = load_config(deps.storage)?;
        let mut stakings = load_stakings(deps.storage, &info.sender)?;
        let mut claim_axis_amount = Uint128::zero();
        let state = load_state(deps.storage)?;

        for stake in stakings.iter_mut() {
            if stake.start_epoch == state.epoch {
                continue;
            }
            let reward = compute_mint_amount(
                &deps,
                &config,
                stake.staking_amount,
                stake.start_epoch,
                state.epoch,
            )?;
            stake.start_epoch = state.epoch;
            claim_axis_amount += reward;
        }

        let claim_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: config.axis_contract.to_string(),
            msg: to_binary(&AxisExecuteMsg::ClaimMintMaker {
                base_denom: config.base_denom,
                price_denom: config.price_denom,
                sender: info.sender,
                amount: claim_axis_amount,
            })?,
            funds: vec![],
        });
        Ok(Response::new().add_message(claim_msg))
    }

    pub fn setting(
        deps: DepsMut<SeiQueryWrapper>,
        info: MessageInfo,
        epoch: u64,
    ) -> Result<Response<SeiMsg>, ContractError> {
        let config = load_config(deps.storage)?;
        check_core_contract(&config.core_contract, &info.sender)?;

        let mut state = load_state(deps.storage)?;

        EPOCH_STAKING_TOTAL_AMOUNT.save(deps.storage, state.epoch, &state.staking_total)?;
        state.staking_total += state.stake_pending_total;
        state.stake_pending_total = Uint128::zero();
        state.epoch = epoch;
        save_state(deps.storage, state)?;

        Ok(Response::new())
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps<SeiQueryWrapper>, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetConfig {} => to_binary(&query::get_config(deps)?),
        QueryMsg::GetState {} => to_binary(&query::get_state(deps)?),
        QueryMsg::GetStakeInfo { address } => to_binary(&query::get_stake_info(deps, address)?),
        QueryMsg::GetUnstakeInfo { address } => {
            to_binary(&query::get_un_stake_info(deps, address)?)
        }
        QueryMsg::GetEpochTotalStaking {
            start_epoch,
            end_epoch,
        } => to_binary(&query::get_epoch_total_staking(
            deps,
            start_epoch,
            end_epoch,
        )?),
    }
}

pub mod query {
    use axis_protocol::lp_staking::{
        ConfigResponse, EpochTotalStakingResponse, StakeInfoResponse, StateResponse,
        UnStakeInfoResponse,
    };
    use cosmwasm_std::{Addr, Deps, Order, StdResult};
    use cw_storage_plus::Bound;
    use sei_cosmwasm::SeiQueryWrapper;

    use crate::state::{
        load_config, load_stakings, load_state, load_un_stakings, EPOCH_STAKING_TOTAL_AMOUNT,
    };

    pub fn get_config(deps: Deps<SeiQueryWrapper>) -> StdResult<ConfigResponse> {
        let config = load_config(deps.storage)?;

        Ok(ConfigResponse {
            core_contract: config.core_contract,
            axis_contract: config.axis_contract,
            lp_denom: config.lp_denom,
            base_denom: config.base_denom,
            price_denom: config.price_denom,
        })
    }
    pub fn get_state(deps: Deps<SeiQueryWrapper>) -> StdResult<StateResponse> {
        let state = load_state(deps.storage)?;

        Ok(StateResponse {
            staking_total: state.staking_total,

            withdraw_pending_total: state.withdraw_pending_total,
            epoch: state.epoch,
            stake_pending_total: state.stake_pending_total,
        })
    }

    pub fn get_stake_info(
        deps: Deps<SeiQueryWrapper>,
        address: Addr,
    ) -> StdResult<Vec<StakeInfoResponse>> {
        let stake_infos = load_stakings(deps.storage, &address)?;
        let stake_infos_res = stake_infos
            .into_iter()
            .map(|stake| StakeInfoResponse {
                start_epoch: stake.start_epoch,
                staking_amount: stake.staking_amount,
            })
            .collect::<Vec<StakeInfoResponse>>();
        Ok(stake_infos_res)
    }

    pub fn get_un_stake_info(
        deps: Deps<SeiQueryWrapper>,
        address: Addr,
    ) -> StdResult<Vec<UnStakeInfoResponse>> {
        let un_stake_infos = load_un_stakings(deps.storage, &address)?;
        let un_stake_infos_res = un_stake_infos
            .into_iter()
            .map(|un_stake| UnStakeInfoResponse {
                unlock_epoch: un_stake.unlock_epoch,
                unstaking_amount: un_stake.unstaking_amount,
            })
            .collect::<Vec<UnStakeInfoResponse>>();
        Ok(un_stake_infos_res)
    }

    pub fn get_epoch_total_staking(
        deps: Deps<SeiQueryWrapper>,
        start_epoch: u64,
        end_epoch: u64,
    ) -> StdResult<Vec<EpochTotalStakingResponse>> {
        let total_staking_amounts = EPOCH_STAKING_TOTAL_AMOUNT
            .range(
                deps.storage,
                Some(Bound::inclusive(start_epoch)),
                Some(Bound::inclusive(end_epoch)),
                Order::Ascending,
            )
            .into_iter()
            .map(|item| {
                let (epoch, amount) = item?;
                Ok(EpochTotalStakingResponse { epoch, amount })
            })
            .collect::<StdResult<Vec<EpochTotalStakingResponse>>>()?;
        Ok(total_staking_amounts)
    }
}

#[cfg(test)]
mod tests {}
