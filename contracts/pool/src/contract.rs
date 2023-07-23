use axis_protocol::lp_staking::InstantiateMsg as LpStakingInstantiateMsg;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    coin, to_binary, Addr, BankMsg, Binary, CosmosMsg, Decimal, Deps, DepsMut, Env, MessageInfo,
    Reply, Response, StdResult, SubMsg, SubMsgResult, Uint128, WasmMsg,
};
use cw2::set_contract_version;

use axis_protocol::pool::{ExecuteMsg, InstantiateMsg, QueryMsg};
use sei_cosmwasm::{SeiMsg, SeiQueryWrapper};

use crate::error::ContractError;

use crate::helpers::check::check_funds_and_get_funds;

use crate::helpers::calculate_lp_mint_amount;
use crate::state::{load_config, save_config, save_pool, Config, Pool};

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
        base_denom,
        base_decimal,
        price_denom,
        price_decimal,
        maximum_borrow_rate,
        market_code_id,
        market_instantiate_msg,
        lp_staking_code_id,
        axis_contract,
        maker,
    } = msg;
    let core_contract = info.sender;
    //inital deposit
    let (base, price) = check_funds_and_get_funds(info.funds, &base_denom, &price_denom)?;

    let (lp_amount, base_amount, price_amount) = calculate_lp_mint_amount(
        base.amount,
        price.amount,
        Uint128::zero(),
        Uint128::zero(),
        base_decimal,
        price_decimal,
        Uint128::zero(),
        LP_DECIMAL,
    )?;

    let lp_denom = "factory/".to_string() + env.contract.address.to_string().as_ref() + "/lp";
    let config = Config {
        core_contract: core_contract.to_owned(),
        lock: false,
        market_contract: Addr::unchecked(""),
        maximum_borrow_rate,
        lp_staking_contract: Addr::unchecked(""),
        base_denom: base_denom.clone(),
        base_decimal,
        price_denom: price_denom.clone(),
        price_decimal,
        lp_denom: lp_denom.clone(),
        lp_decimal: LP_DECIMAL,
        withdraw_fee_rate: Decimal::permille(1),
    };

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let pool = Pool {
        base_amount,
        price_amount,
        base_borrow_amount: Uint128::zero(),
        price_borrow_amount: Uint128::zero(),
        lp_total_supply: lp_amount,
    };
    save_config(deps.storage, &config)?;

    save_pool(deps.storage, &pool)?;
    let market_instantiate_tx = SubMsg::reply_on_success(
        CosmosMsg::Wasm(WasmMsg::Instantiate {
            admin: None,
            code_id: market_code_id,
            msg: to_binary(&market_instantiate_msg)?,
            funds: vec![],
            label: format!("{:?}:{:?} market", base_denom, price_denom),
        }),
        1,
    );

    let lp_staking_instantiate_tx = SubMsg::reply_on_success(
        CosmosMsg::Wasm(WasmMsg::Instantiate {
            admin: None,
            code_id: lp_staking_code_id,
            msg: to_binary(&LpStakingInstantiateMsg {
                axis_contract,
                core_contract,
                base_denom: base_denom.to_owned(),
                price_denom: price_denom.to_owned(),
                lp_denom: lp_denom.to_owned(),
            })?,
            funds: vec![],
            label: format!("{:?}:{:?} market", base_denom, price_denom),
        }),
        2,
    );

    let lp_create_msg = SeiMsg::CreateDenom {
        subdenom: "lp".to_string(),
    };

    let lp_token = coin(lp_amount.into(), lp_denom);

    let initial_lp_mint = SeiMsg::MintTokens {
        amount: lp_token.to_owned(),
    };

    let send_msg = SubMsg::new(BankMsg::Send {
        to_address: maker.to_string(),
        amount: vec![lp_token],
    });

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("contract_address", env.contract.address)
        .add_attribute("base_denom", base_denom)
        .add_attribute("price_denom", price_denom)
        .add_submessages(vec![market_instantiate_tx, lp_staking_instantiate_tx])
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
                check_core_contract, check_funds_and_get_funds, check_lock,
                check_lp_funds_and_get_lp_funds, check_market_contract,
                check_maximum_leverage_amount, check_repay_denom,
            },
            create_bank_msg,
        },
        state::{load_config, load_pool},
    };

    use cosmwasm_std::CosmosMsg;
    use cosmwasm_std::{coin, BankMsg, Uint128};

    use sei_cosmwasm::SeiMsg;

    use super::*;

    pub fn leverage_borrow(
        deps: DepsMut<SeiQueryWrapper>,
        info: MessageInfo,
        position: bool,
        amount: Uint128,
    ) -> Result<Response<SeiMsg>, ContractError> {
        check_lock(deps.storage)?;
        let config = load_config(deps.storage)?;
        check_market_contract(&config.market_contract, &info.sender)?;
        let mut pool = load_pool(deps.storage)?;

        let denom = match position {
            true => config.base_denom.to_owned(),
            false => config.price_denom.to_owned(),
        };

        match position {
            true => {
                check_maximum_leverage_amount(amount, pool.base_amount, config.maximum_borrow_rate)?
            }
            false => check_maximum_leverage_amount(
                amount,
                pool.price_amount,
                config.maximum_borrow_rate,
            )?,
        };
        match position {
            true => {
                pool.base_amount -= amount;
                pool.base_borrow_amount += amount;
            }
            false => {
                pool.price_amount -= amount;
                pool.price_borrow_amount += amount;
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
        let mut pool = load_pool(deps.storage)?;
        let config = load_config(deps.storage)?;
        check_repay_denom(info.funds, &denom, amount)?;
        check_market_contract(&config.market_contract, &info.sender)?;
        match position {
            true => {
                pool.base_amount += amount;
                pool.base_borrow_amount -= borrowed_amount;
            }
            false => {
                pool.price_amount += amount;
                pool.price_borrow_amount -= borrowed_amount;
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
        let config = load_config(deps.storage)?;
        let reserve_price_amount = pool.price_amount + pool.price_borrow_amount;
        let reserve_base_amount = pool.base_amount + pool.base_borrow_amount;
        let (send_base, send_price) =
            check_funds_and_get_funds(info.funds, &config.base_denom, &config.price_denom)?;

        let lp_total_supply = pool.lp_total_supply;

        //@@accepted_base = deposit 가능한 base_amount
        //@@accpeted_price = deposit 가능한 price_amount
        let (lp_mint_amount, accept_base, accept_price) = calculate_lp_mint_amount(
            send_base.amount,
            send_price.amount,
            reserve_base_amount,
            reserve_price_amount,
            config.base_decimal.into(),
            config.price_decimal.into(),
            lp_total_supply,
            config.lp_decimal.into(),
        )?;
        pool.lp_total_supply += lp_mint_amount;
        pool.base_amount += accept_base;
        pool.price_amount += accept_price;
        save_pool(deps.storage, &pool)?;

        let mut bank_msgs = Vec::new();

        //남은 금액 정산 시켜주는 로직
        if let Some(msg) = create_bank_msg(
            accept_base,
            send_base.amount,
            &send_base.denom,
            &info.sender.to_string(),
        ) {
            bank_msgs.push(msg);
        }

        if let Some(msg) = create_bank_msg(
            accept_price,
            send_price.amount,
            &send_price.denom,
            &info.sender.to_string(),
        ) {
            bank_msgs.push(msg);
        }

        let lp_token = coin(lp_mint_amount.into(), config.lp_denom);
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
        let config = load_config(deps.storage)?;

        let lp_token = check_lp_funds_and_get_lp_funds(info.funds, &config.lp_denom)?;
        let lp_total_supply = pool.lp_total_supply;

        let withdraw_base_amount = pool
            .base_amount
            .checked_multiply_ratio(lp_token.amount, lp_total_supply)
            .and_then(|amount| Ok(amount - (amount * config.withdraw_fee_rate)))
            .unwrap();

        let withdraw_price_amount = pool
            .price_amount
            .checked_multiply_ratio(lp_token.amount, lp_total_supply)
            .and_then(|amount| Ok(amount - (amount * config.withdraw_fee_rate)))
            .unwrap();

        pool.base_amount -= withdraw_base_amount;
        pool.price_amount -= withdraw_price_amount;
        pool.lp_total_supply -= lp_token.amount;
        save_pool(deps.storage, &pool)?;

        let lp_burn_msg = SeiMsg::BurnTokens { amount: lp_token };

        let base_bank_msg = CosmosMsg::Bank(BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: vec![coin(withdraw_base_amount.into(), config.base_denom)],
        });
        let price_bank_msg = CosmosMsg::Bank(BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: vec![coin(withdraw_price_amount.into(), config.price_denom)],
        });
        Ok(Response::new()
            .add_message(base_bank_msg)
            .add_message(price_bank_msg)
            .add_message(lp_burn_msg))
    }

    pub fn lock(
        deps: DepsMut<SeiQueryWrapper>,
        _env: Env,
        info: MessageInfo,
    ) -> Result<Response<SeiMsg>, ContractError> {
        let config = load_config(deps.storage)?;
        check_core_contract(&config.core_contract, &info.sender)?;
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
        let config = load_config(deps.storage)?;
        check_core_contract(&config.core_contract, &info.sender)?;
        let mut config = load_config(deps.storage)?;
        config.lock = false;
        save_config(deps.storage, &config)?;
        Ok(Response::default())
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
    use axis_protocol::pool::{
        ConfigResponse, PoolResponse, PositionBalance, PositionInformationResponse,
    };

    use super::*;
    pub fn get_position_balance(
        deps: Deps<SeiQueryWrapper>,
        position: bool,
    ) -> StdResult<PositionBalance> {
        let pool = load_pool(deps.storage)?;
        match position {
            true => Ok(PositionBalance {
                amount: pool.base_amount,
            }),
            false => Ok(PositionBalance {
                amount: pool.price_amount,
            }),
        }
    }
    pub fn get_position_information(
        deps: Deps<SeiQueryWrapper>,
        position: bool,
    ) -> StdResult<PositionInformationResponse> {
        let pool = load_pool(deps.storage)?;
        let config = load_config(deps.storage)?;
        match position {
            true => Ok(PositionInformationResponse {
                denom: config.base_denom,
                amount: pool.base_amount,
                decimal: config.base_decimal,
            }),
            false => Ok(PositionInformationResponse {
                denom: config.price_denom,
                amount: pool.price_amount,
                decimal: config.price_decimal,
            }),
        }
    }
    pub fn get_config(deps: Deps<SeiQueryWrapper>) -> StdResult<ConfigResponse> {
        let config = load_config(deps.storage)?;

        Ok(ConfigResponse {
            core_contract: config.core_contract,
            lock: config.lock,
            market_contract: config.market_contract,
            lp_decimal: config.lp_decimal,
            lp_denom: config.lp_denom,
            maximum_borrow_rate: config.maximum_borrow_rate,
            lp_staking_contract: config.lp_staking_contract,
            base_decimal: config.base_decimal,
            base_denom: config.base_denom,
            price_decimal: config.price_decimal,
            price_denom: config.price_denom,
            withdraw_fee_rate: config.withdraw_fee_rate,
        })
    }
    pub fn get_pool(deps: Deps<SeiQueryWrapper>) -> StdResult<PoolResponse> {
        let pool = load_pool(deps.storage)?;
        Ok(PoolResponse {
            base_amount: pool.base_amount,
            price_amount: pool.price_amount,
            base_borrow_amount: pool.base_borrow_amount,
            price_borrow_amount: pool.price_borrow_amount,
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
        1 => {
            // get new market contract address
            match msg.result {
                SubMsgResult::Ok(res) => match res.data {
                    Some(data) => {
                        let mut config = load_config(deps.storage)?;
                        let addr = String::from_utf8(data.to_vec()).unwrap();
                        let market_contract_addr = deps.api.addr_validate(addr.trim())?;

                        config.market_contract = market_contract_addr.clone();
                        save_config(deps.storage, &config)?;

                        Ok(Response::new())
                    }
                    None => Err(ContractError::MissingMarketContractAddr {}),
                },
                SubMsgResult::Err(_) => Err(ContractError::MarketContractInstantiationFailed {}),
            }
        }
        2 => match msg.result {
            SubMsgResult::Ok(res) => match res.data {
                Some(data) => {
                    let mut config = load_config(deps.storage)?;
                    let addr = String::from_utf8(data.to_vec()).unwrap();
                    let lp_staking_contract_addr = deps.api.addr_validate(addr.trim())?;
                    config.lp_staking_contract = lp_staking_contract_addr;
                    save_config(deps.storage, &config)?;

                    Ok(Response::new())
                }
                None => Err(ContractError::MissingMarketContractAddr {}),
            },
            SubMsgResult::Err(_) => Err(ContractError::LpStakingContractInstantiationFailed {}),
        },
        _ => Err(ContractError::InvalidReplyId {}),
    }
}
