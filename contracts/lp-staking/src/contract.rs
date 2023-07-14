#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128};
use cw2::set_contract_version;
use sei_cosmwasm::{SeiMsg, SeiQueryWrapper};
// use cw2::set_contract_version;

use crate::{
    error::ContractError,
    state::{save_config, Config},
};

use axis_protocol::lp_staking::{ExecuteMsg, InstantiateMsg, QueryMsg};

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
    let config = Config {
        lp_denom: msg.lp_denom,
        axis_contract: msg.axis_contract,
        core_contract: msg.core_contract,
        staking_total: Uint128::zero(),
        base_denom: msg.base_denom,
        price_denom: msg.price_denom,
    };
    save_config(deps.storage, &config)?;
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut<SeiQueryWrapper>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<SeiMsg>, ContractError> {
    use execute::*;
    match msg {
        ExecuteMsg::Staking {} => staking(deps, info),
        ExecuteMsg::UnStaking {} => un_staking(deps, info),
        ExecuteMsg::Withdraw {} => withdraw(deps, info),
        ExecuteMsg::ClaimReward {} => claim_reward(deps, info),
    }
}

mod execute {
    use crate::{
        helpers::{check_funds_and_get_lp, compute_mint_amount},
        state::{
            load_config, load_stakings, load_un_stakings, save_config, StakeInfo, UnStakeInfo,
            STAKING, UN_STAKING,
        },
        ContractError,
    };
    use axis_protocol::{axis::ExecuteMsg as AxisExecuteMsg, query::query_epoch};
    use cosmwasm_std::{
        coin, to_binary, BankMsg, CosmosMsg, DepsMut, MessageInfo, Response, StdResult, SubMsg,
        Uint128, WasmMsg,
    };
    use sei_cosmwasm::{SeiMsg, SeiQueryWrapper};

    pub fn staking(
        deps: DepsMut<SeiQueryWrapper>,
        info: MessageInfo,
    ) -> Result<Response<SeiMsg>, ContractError> {
        let mut config = load_config(deps.storage)?;
        let lp_coin = check_funds_and_get_lp(info.funds, &config.lp_denom)?;
        let epoch = query_epoch(deps.querier, &config.core_contract)?;
        STAKING.update(
            deps.storage,
            &info.sender,
            |exsits| -> StdResult<Vec<StakeInfo>> {
                match exsits {
                    Some(mut stakings) => {
                        if let Some(stake) = stakings.iter_mut().find(|s| s.start_epoch == epoch) {
                            stake.staking_amount += lp_coin.amount;
                        } else {
                            stakings.push(StakeInfo {
                                start_epoch: epoch + 1,
                                staking_amount: lp_coin.amount,
                            })
                        }
                        Ok(stakings)
                    }
                    None => {
                        let stakings = vec![StakeInfo {
                            start_epoch: epoch + 1,
                            staking_amount: lp_coin.amount,
                        }];
                        Ok(stakings)
                    }
                }
            },
        )?;
        config.staking_total += lp_coin.amount;
        save_config(deps.storage, &config)?;
        Ok(Response::default())
    }

    pub fn un_staking(
        deps: DepsMut<SeiQueryWrapper>,
        info: MessageInfo,
    ) -> Result<Response<SeiMsg>, ContractError> {
        let config = load_config(deps.storage)?;
        let epoch = query_epoch(deps.querier, &config.core_contract)?;
        let stakings = load_stakings(deps.storage, &info.sender)?;

        let unlock_epoch = epoch + 1;

        let mut axis_amount = Uint128::zero();
        let mut unstaking_amount = Uint128::zero();
        for stake in stakings.into_iter() {
            axis_amount += compute_mint_amount(
                &deps,
                &config,
                stake.staking_amount,
                stake.start_epoch,
                epoch,
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
    pub fn withdraw(
        deps: DepsMut<SeiQueryWrapper>,
        info: MessageInfo,
    ) -> Result<Response<SeiMsg>, ContractError> {
        let mut config = load_config(deps.storage)?;

        let un_stakings = load_un_stakings(deps.storage, &info.sender)?;

        let epoch = query_epoch(deps.querier, &config.core_contract)?;

        let lp_amount: Uint128 = un_stakings
            .iter()
            .filter(|stake| stake.unlock_epoch < epoch)
            .fold(Uint128::zero(), |lp_amount, stake| {
                lp_amount + stake.unstaking_amount
            });

        let remaining_un_stakings = un_stakings
            .into_iter()
            .filter(|un_stake| un_stake.unlock_epoch >= epoch)
            .collect::<Vec<UnStakeInfo>>();

        match remaining_un_stakings.len() {
            0 => UN_STAKING.remove(deps.storage, &info.sender),
            _ => UN_STAKING.save(deps.storage, &info.sender, &remaining_un_stakings)?,
        }

        config.staking_total -= lp_amount;
        save_config(deps.storage, &config)?;

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
        let epoch = query_epoch(deps.querier, &config.core_contract)?;
        for stake in stakings.iter_mut() {
            if stake.start_epoch == epoch {
                continue;
            }

            let reward = compute_mint_amount(
                &deps,
                &config,
                stake.staking_amount,
                stake.start_epoch,
                epoch,
            )?;
            stake.start_epoch = epoch;
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
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps<SeiQueryWrapper>, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    unimplemented!()
}

#[cfg(test)]
mod tests {}
