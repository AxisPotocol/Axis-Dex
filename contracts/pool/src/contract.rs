#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    coin, to_binary, Addr, BankMsg, Binary, CosmosMsg, Decimal, Deps, DepsMut, Env, MessageInfo,
    Reply, Response, StdResult, SubMsg, SubMsgResult, Uint128, WasmMsg,
};
use cw2::set_contract_version;

use axis::pool::{ExecuteMsg, InstantiateMsg, QueryMsg};
use sei_cosmwasm::{SeiMsg, SeiQueryWrapper};

use crate::error::ContractError;

use crate::helpers::check::check_funds_and_get_funds;

use crate::helpers::calculate_lp_mint_amount;
use crate::state::{register_market_contract, save_config, save_pool, Config, Pool};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:pool";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");
const LP_DECIMAL: u8 = 6;
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut<SeiQueryWrapper>,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response<SeiMsg>, ContractError> {
    let InstantiateMsg {
        asset_denom,
        asset_decimal,
        stable_denom,
        stable_decimal,
        maximum_borrow_rate,
        market_code_id,
        market_instantiate_msg,
    } = msg;

    //inital deposit
    let (asset, stable) = check_funds_and_get_funds(info.funds, &asset_denom, &stable_denom)?;

    let (lp_amount, asset_amount, stable_amount) = calculate_lp_mint_amount(
        asset.amount,
        stable.amount,
        Uint128::zero(),
        Uint128::zero(),
        asset_decimal,
        stable_decimal,
        Uint128::zero(),
        LP_DECIMAL,
    )?;

    let lp_denom = "factory/".to_string() + env.contract.address.to_string().as_ref() + "/lp";
    let pool = Pool {
        asset_denom: asset_denom.clone(),
        asset_amount,
        asset_decimal,
        stable_denom: stable_denom.clone(),
        stable_amount,
        stable_decimal,
        asset_borrow_amount: Uint128::default(),
        stable_borrow_amount: Uint128::default(),
        lp_denom: lp_denom.clone(),
        lp_total_supply: lp_amount,
        lp_decimal: LP_DECIMAL,
    };

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    save_pool(deps.storage, &pool)?;
    let config = Config {
        owner: info.sender.clone(),
        lock: false,
        market_contract: Addr::unchecked(""),
        maximum_borrow_rate,
    };
    save_config(deps.storage, &config)?;
    let market_instantiate_tx = SubMsg::reply_on_success(
        CosmosMsg::Wasm(WasmMsg::Instantiate {
            admin: Some(info.sender.to_string()),
            code_id: market_code_id,
            msg: to_binary(&market_instantiate_msg)?,
            funds: vec![],
            label: format!("{:?}:{:?} market", asset_denom, stable_denom),
        }),
        1,
    );

    let lp_create_msg = SeiMsg::CreateDenom {
        subdenom: "lp".to_string(),
    };

    let lp_token = coin(lp_amount.into(), lp_denom);
    let initial_lp_mint = SeiMsg::MintTokens {
        amount: lp_token.to_owned(),
    };

    let send_msg = SubMsg::new(BankMsg::Send {
        to_address: info.sender.to_string(),
        amount: vec![lp_token],
    });

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("contract_address", env.contract.address)
        .add_attribute("asset_denom", asset_denom)
        .add_attribute("stable_denom", stable_denom)
        .add_submessage(market_instantiate_tx)
        .add_message(lp_create_msg)
        .add_message(initial_lp_mint)
        .add_submessage(send_msg))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut<SeiQueryWrapper>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<SeiMsg>, ContractError> {
    use ExecuteMsg::*;
    match msg {
        LeverageBorrow { position, amount } => {
            execute::leverage_borrow(deps, info, position, amount)
        }
        RePay {
            denom,
            position,
            amount,
            borrowed_amount,
        } => execute::repay(deps, info, denom, position, amount, borrowed_amount),
        Deposit {} => execute::deposit(deps, info, env),
        Withdraw {} => execute::withdraw(deps, env, info),
        Lock {} => execute::lock(deps, env, info),
        UnLock {} => execute::un_lock(deps, env, info),
    }
}

pub mod execute {
    use crate::{
        helpers::{
            check::{
                check_funds_and_get_funds, check_lp_funds_and_get_lp_funds, check_market_contract,
                check_maximum_leverage_amount, check_repay_denom,
            },
            create_bank_msg,
        },
        state::{
            check_lock, check_owner, load_config, load_maximum_borrow_rate, load_pool,
            save_remove_amount_pool,
        },
    };
    use cosmwasm_std::{coin, BankMsg, CosmosMsg, Decimal, Uint128};

    use sei_cosmwasm::SeiMsg;

    use super::*;

    pub fn leverage_borrow(
        deps: DepsMut<SeiQueryWrapper>,
        info: MessageInfo,
        position: bool,
        amount: Uint128,
    ) -> Result<Response<SeiMsg>, ContractError> {
        check_lock(deps.storage)?;
        check_market_contract(deps.storage, &info.sender)?;
        let mut pool = load_pool(deps.storage)?;
        let denom = match position {
            true => pool.asset_denom.clone(),
            false => pool.stable_denom.clone(),
        };
        let maximum_borrow_rate = load_maximum_borrow_rate(deps.storage)?;
        match position {
            true => check_maximum_leverage_amount(amount, pool.asset_amount, maximum_borrow_rate)?,
            false => {
                check_maximum_leverage_amount(amount, pool.stable_amount, maximum_borrow_rate)?
            }
        };
        match position {
            true => {
                pool.asset_amount -= amount;
                pool.asset_borrow_amount += amount;
            }
            false => {
                pool.stable_amount -= amount;
                pool.stable_borrow_amount += amount;
            }
        }

        save_pool(deps.storage, &pool)?;
        let msg = CosmosMsg::Bank(BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: vec![coin(amount.into(), denom)],
        });
        Ok(Response::new().add_message(msg))
    }

    pub fn repay(
        deps: DepsMut<SeiQueryWrapper>,
        info: MessageInfo,
        denom: String,
        position: bool,
        amount: Uint128,
        borrowed_amount: Uint128,
    ) -> Result<Response<SeiMsg>, ContractError> {
        check_market_contract(deps.storage, &info.sender)?;
        check_repay_denom(info.funds, &denom, amount)?;
        let mut pool = load_pool(deps.storage)?;
        match position {
            true => {
                pool.asset_amount += amount;
                pool.asset_borrow_amount -= borrowed_amount;
            }
            false => {
                pool.stable_amount += amount;
                pool.stable_borrow_amount -= borrowed_amount;
            }
        }
        save_pool(deps.storage, &pool)?;
        Ok(Response::new())
    }

    pub fn deposit(
        deps: DepsMut<SeiQueryWrapper>,
        info: MessageInfo,
        _env: Env,
    ) -> Result<Response<SeiMsg>, ContractError> {
        let mut pool = load_pool(deps.storage)?;

        let reserve_stable_amount = pool.stable_amount + pool.stable_borrow_amount;
        let reserve_asset_amount = pool.asset_amount + pool.asset_borrow_amount;
        let (send_asset, send_stable) =
            check_funds_and_get_funds(info.funds, &pool.asset_denom, &pool.stable_denom)?;

        //@@accepted_asset = deposit 가능한 asset_amount
        //@@accpeted_stable = deposit 가능한 stable_amount
        let (lp_mint_amount, accept_asset, accept_stable) = calculate_lp_mint_amount(
            send_asset.amount,
            send_stable.amount,
            reserve_asset_amount,
            reserve_stable_amount,
            pool.asset_decimal.into(),
            pool.stable_decimal.into(),
            pool.lp_total_supply,
            pool.lp_decimal.into(),
        )?;
        pool.lp_total_supply += lp_mint_amount;
        pool.asset_amount += accept_asset;
        pool.stable_amount += accept_stable;
        save_pool(deps.storage, &pool)?;
        // save_add_total_supply(deps.storage, &mut pool, lp_mint_amount);
        // save_add_amount_pool(deps.storage, &mut pool, accepted_asset, accepted_stable)?;
        let mut bank_msgs = Vec::new();

        //남은 금액 정산 시켜주는 로직
        if let Some(msg) = create_bank_msg(
            accept_asset,
            send_asset.amount,
            &send_asset.denom,
            &info.sender.to_string(),
        ) {
            bank_msgs.push(msg);
        }

        if let Some(msg) = create_bank_msg(
            accept_stable,
            send_stable.amount,
            &send_stable.denom,
            &info.sender.to_string(),
        ) {
            bank_msgs.push(msg);
        }

        let lp_token = coin(lp_mint_amount.into(), pool.lp_denom);
        let lp_mint_msg = SeiMsg::MintTokens {
            amount: lp_token.to_owned(),
        };

        let lp_send_msg = SubMsg::new(BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: vec![lp_token],
        });
        match bank_msgs.is_empty() {
            true => Ok(Response::new()
                .add_message(lp_mint_msg)
                .add_submessage(lp_send_msg)),
            false => Ok(Response::new()
                .add_messages(bank_msgs)
                .add_message(lp_mint_msg)
                .add_submessage(lp_send_msg)),
        }
    }

    pub fn withdraw(
        deps: DepsMut<SeiQueryWrapper>,
        _env: Env,
        info: MessageInfo,
    ) -> Result<Response<SeiMsg>, ContractError> {
        let mut pool = load_pool(deps.storage)?;

        let lp_token = check_lp_funds_and_get_lp_funds(info.funds, &pool.lp_denom)?;
        let lp_total_supply = pool.lp_total_supply;

        let ratio = Decimal::from_ratio(lp_token.amount, lp_total_supply);

        let withdraw_asset_amount = pool.asset_amount * ratio;
        let withdraw_stable_amount = pool.stable_amount * ratio;

        save_remove_amount_pool(
            deps.storage,
            &mut pool,
            withdraw_asset_amount,
            withdraw_stable_amount,
        )?;

        let lp_burn_msg = SeiMsg::BurnTokens { amount: lp_token };

        let asset_bank_msg = CosmosMsg::Bank(BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: vec![coin(withdraw_asset_amount.into(), pool.asset_denom)],
        });
        let stable_bank_msg = CosmosMsg::Bank(BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: vec![coin(withdraw_stable_amount.into(), pool.stable_denom)],
        });
        Ok(Response::new()
            .add_message(asset_bank_msg)
            .add_message(stable_bank_msg)
            .add_message(lp_burn_msg))
    }

    pub fn lock(
        deps: DepsMut<SeiQueryWrapper>,
        _env: Env,
        info: MessageInfo,
    ) -> Result<Response<SeiMsg>, ContractError> {
        check_owner(deps.storage, info.sender)?;
        let mut config = load_config(deps.storage)?;
        config.lock = true;
        save_config(deps.storage, &config)?;
        Ok(Response::default())
    }
    pub fn un_lock(
        deps: DepsMut<SeiQueryWrapper>,
        _env: Env,
        info: MessageInfo,
    ) -> Result<Response<SeiMsg>, ContractError> {
        check_owner(deps.storage, info.sender)?;
        let mut config = load_config(deps.storage)?;
        config.lock = false;
        save_config(deps.storage, &config)?;
        Ok(Response::default())
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn reply(
    deps: DepsMut<SeiQueryWrapper>,
    _env: Env,
    msg: Reply,
) -> Result<Response<SeiMsg>, ContractError> {
    match msg.id {
        1 => {
            // get new market contract address
            match msg.result {
                SubMsgResult::Ok(res) => match res.data {
                    Some(data) => {
                        let market_contract_addr = String::from_utf8(data.0).unwrap();

                        register_market_contract(
                            deps.storage,
                            Addr::unchecked(market_contract_addr),
                        )?;
                        Ok(Response::new())
                    }
                    None => Err(ContractError::MissingMarketContractAddr {}),
                },
                SubMsgResult::Err(_) => Err(ContractError::MarketContractInstantiationFailed {}),
            }
        }

        _ => Err(ContractError::InvalidReplyId {}),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps<SeiQueryWrapper>, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    use QueryMsg::*;
    match msg {
        GetPositionInformation { position } => {
            to_binary(&query::get_position_information(deps, position)?)
        }
        GetPositionBalance { position } => to_binary(&query::get_position_balance(deps, position)?),
        GetConfig {} => to_binary(&query::get_config(deps)?),
        GetPool {} => to_binary(&query::get_pool(deps)?),
    }
}

pub mod query {
    use crate::state::{load_config, load_pool};
    use axis::pool::{ConfigResponse, PoolResponse, PositionBalance, PositionInformationResponse};

    use super::*;
    pub fn get_position_balance(
        deps: Deps<SeiQueryWrapper>,
        position: bool,
    ) -> StdResult<PositionBalance> {
        let pool = load_pool(deps.storage)?;
        match position {
            true => Ok(PositionBalance {
                amount: pool.asset_amount,
            }),
            false => Ok(PositionBalance {
                amount: pool.stable_amount,
            }),
        }
    }
    pub fn get_position_information(
        deps: Deps<SeiQueryWrapper>,
        position: bool,
    ) -> StdResult<PositionInformationResponse> {
        let pool = load_pool(deps.storage)?;
        match position {
            true => Ok(PositionInformationResponse {
                denom: pool.asset_denom,
                amount: pool.asset_amount,
                decimal: pool.asset_decimal,
            }),
            false => Ok(PositionInformationResponse {
                denom: pool.stable_denom,
                amount: pool.stable_amount,
                decimal: pool.stable_decimal,
            }),
        }
    }
    pub fn get_config(deps: Deps<SeiQueryWrapper>) -> StdResult<ConfigResponse> {
        let config = load_config(deps.storage)?;

        Ok(ConfigResponse {
            market_contract: config.market_contract.to_string(),
            maximum_borrow_rate: config.maximum_borrow_rate,
        })
    }
    pub fn get_pool(deps: Deps<SeiQueryWrapper>) -> StdResult<PoolResponse> {
        let pool = load_pool(deps.storage)?;
        Ok(PoolResponse {
            asset_denom: pool.asset_denom,
            asset_amount: pool.asset_amount,
            asset_decimal: pool.asset_decimal,
            stable_denom: pool.stable_denom,
            stable_amount: pool.stable_amount,
            stable_decimal: pool.stable_decimal,
            asset_borrow_amount: pool.asset_borrow_amount,
            stable_borrow_amount: pool.stable_borrow_amount,
            lp_decimal: pool.lp_decimal,
            lp_denom: pool.lp_denom,
            lp_total_supply: pool.lp_total_supply,
        })
    }
}

#[cfg(test)]
mod tests {}
