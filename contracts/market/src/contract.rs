#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128,
};
use cw2::set_contract_version;
use rune::market::{ExecuteMsg, InstantiateMsg, QueryMsg};
use rune::pool::ExecuteMsg as PoolExecuteMsg;
use rune::treasury::ExecuteMsg as TreasuryExecuteMsg;
use rune::vault::ExecuteMsg as VaultExecuteMsg;
use sei_cosmwasm::{SeiMsg, SeiQueryWrapper};

use crate::error::ContractError;

use crate::query::query_entry_and_stable_price;
use crate::state::{Config, FeeConfig, CONFIG, FEE_CONFIG};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:market";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    // deps: DepsMut,
    deps: DepsMut<SeiQueryWrapper>,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response<SeiMsg>, ContractError> {
    let InstantiateMsg {
        asset_denom,
        asset_decimal,
        stable_denom,
        stable_decimal,
        max_leverage,
        borrow_fee_rate,
        open_close_fee_rate,
        limit_profit_loss_open_fee_rate,
        treasury_contract,
    } = msg;

    let (past_price, _) = query_entry_and_stable_price(&deps.querier, &asset_denom, &stable_denom)?;
    let config = Config {
        owner: info.sender.clone(),
        asset_denom: asset_denom.clone(),
        asset_decimal,
        stable_denom: stable_denom.clone(),
        stable_decimal,
        max_leverage,
        pool_contract: info.sender.clone(),
        asset_total_fee: Uint128::default(),
        stable_total_fee: Uint128::default(),
        fee_vault_contract: Addr::unchecked("valut contract"),
        treasury_contract: Addr::unchecked(treasury_contract),
        past_price,
    };
    let fee_config = FeeConfig {
        borrow_fee_rate,
        open_close_fee_rate,
        limit_profit_loss_open_fee_rate,
    };
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    CONFIG.save(deps.storage, &config)?;
    FEE_CONFIG.save(deps.storage, &fee_config)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender)
        .add_attribute("asset_denom", asset_denom)
        .add_attribute("stable_denom", stable_denom)
        .add_attribute("max_leverage", format!("{}", max_leverage))
        .add_attribute("borrow_fee_late", format!("{}", borrow_fee_rate))
        .add_attribute("open_close_fee_late", format!("{}", open_close_fee_rate)))
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
        Open {
            position,
            leverage,
            limit_profit_price,
            limit_loss_price,
        } => execute::open(
            deps,
            env,
            info,
            position,
            leverage,
            limit_profit_price,
            limit_loss_price,
        ),
        Close {} => execute::close(deps, env, info),
        Liquidated {} => execute::hook_liquidated(deps, env, info),
    }
}

pub mod execute {

    use crate::{
        helpers::{
            calculate_close_fee_amount, calculate_open_fee_amount,
            check::{
                check_funds_for_positions_get_funds, check_leverage_amount, check_leverage_rate,
            },
            control_desitinated_traders, fee_division, get_trade_information, get_trader_amount,
            get_usd_amount,
        },
        position::Position,
        query::{query_entry_and_stable_price, query_pool_balance},
        state::{
            get_config_pool_contract, load_config, load_fee_config, load_open_and_close_fee,
            save_config_fee, update_past_price,
        },
        trade::{get_desitinated_price_traders, trade_load, trade_remove, trade_update, Trade},
    };
    use cosmwasm_std::{coin, BankMsg, CosmosMsg, Uint128, WasmMsg};

    use sei_cosmwasm::SeiQueryWrapper;

    use super::*;

    pub fn open(
        deps: DepsMut<SeiQueryWrapper>,
        _env: Env,
        info: MessageInfo,
        //position is true = long , false = short
        position: bool,
        leverage: u8,
        limit_profit_price: Option<Uint128>,
        limit_loss_price: Option<Uint128>,
    ) -> Result<Response<SeiMsg>, ContractError> {
        //@@ fee 는 다끝나고 가져가는걸로? 현재 로직 open 시 바로 config 에 저장하고 매 블록마다 보냄.
        //@@ 어떻게 버로잉 fee 기록할까?
        //TimeStamp?1시간은 3600초
        //trade 에 기록해놨다가 fee 가져갈까?
        let mut config = load_config(deps.storage)?;
        let position = Position::new(position);
        check_leverage_rate(leverage, config.max_leverage)?;
        //fund 확인
        let (collateral_denom, collateral_amount) =
            check_funds_for_positions_get_funds(info.funds, &config, &position)?;

        //@@ entry price 가격 받아오기 난중에 Hook 로 빼자. config 로 뺄것.
        let (entry_price, stable_price) =
            query_entry_and_stable_price(&deps.querier, &config.asset_denom, &config.stable_denom)?;

        //funds 최소 금액 확인

        let open_fee_amount = {
            let fee_config = load_fee_config(deps.storage)?;

            match (limit_loss_price, limit_profit_price) {
                (None, None) => calculate_open_fee_amount(
                    collateral_amount,
                    leverage,
                    fee_config.open_close_fee_rate,
                ),
                (None, Some(_)) | (Some(_), None) | (Some(_), Some(_)) => {
                    calculate_open_fee_amount(
                        collateral_amount,
                        leverage,
                        fee_config.limit_profit_loss_open_fee_rate,
                    )
                }
            }
        };
        save_config_fee(deps.storage, &mut config, &position, open_fee_amount)?;

        let collateral_amount = collateral_amount - open_fee_amount;

        //@@ 이 로직 확인!
        let (position_size, leverage_amount, liquidation_price) = {
            if position == Position::Long {
                get_trade_information(
                    entry_price,
                    entry_price,
                    collateral_amount,
                    config.asset_decimal,
                    open_fee_amount,
                    leverage,
                    &position,
                )?
            } else {
                get_trade_information(
                    entry_price,
                    stable_price,
                    collateral_amount,
                    config.stable_decimal,
                    open_fee_amount,
                    leverage,
                    &position,
                )?
            }
        };

        let pool_balance = query_pool_balance(deps.querier, &config.pool_contract, &position)?;

        //@@ 풀의 몇% 까지? 정해지면 로직넣어야함.
        //@@ pool 에서도 로직 있음.
        check_leverage_amount(pool_balance, leverage_amount)?;

        //info 만들기
        let trade = Trade::new(
            info.sender.clone(),
            entry_price.atomics(),
            liquidation_price,
            limit_profit_price,
            limit_loss_price,
            collateral_denom.clone(),
            collateral_amount,
            position.clone(),
            position_size,
            leverage,
            leverage_amount,
        );

        //Trade 저장하는 로직.
        trade_update(deps.storage, info.sender.clone(), trade)?;

        //Pool에서 자금 빌려오는 메시지

        let execute_msg = match position {
            Position::Long => PoolExecuteMsg::LeverageBorrow {
                position: true,
                amount: leverage_amount,
            },
            Position::Short => PoolExecuteMsg::LeverageBorrow {
                position: false,
                amount: leverage_amount,
            },
        };
        let fee_usd = match position {
            Position::Long => get_usd_amount(open_fee_amount, config.asset_decimal, entry_price)?,
            Position::Short => {
                get_usd_amount(open_fee_amount, config.stable_decimal, stable_price)?
            }
        };
        //@@ attribute 만들어야함.
        Ok(Response::new()
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: config.pool_contract.to_string(),
                msg: to_binary(&execute_msg)?,
                funds: vec![],
            }))
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: config.treasury_contract.to_string(),
                msg: to_binary(&TreasuryExecuteMsg::AddFeeAmount {
                    asset_denom: config.asset_denom,
                    stable_denom: config.stable_denom,
                    trader: info.sender.to_string(),
                    fee_usd_amount: fee_usd.to_uint_ceil(),
                })?,
                funds: vec![],
            })))
    }

    pub fn close(
        deps: DepsMut<SeiQueryWrapper>,
        _env: Env,
        info: MessageInfo,
    ) -> Result<Response<SeiMsg>, ContractError> {
        let mut config = load_config(deps.storage)?;
        //@@Trade 에서 timestamp 저장해야함.
        let trade = trade_load(deps.storage, info.sender)?;

        let Trade {
            trader,
            entry_price,
            collateral_denom: denom,
            collateral_amount,
            position: user_position,
            leverage,
            leverage_amount,
            ..
        } = trade;

        let (now_price_dec, stable_price_dec) =
            query_entry_and_stable_price(&deps.querier, &config.asset_denom, &config.stable_denom)?;

        let now_price = now_price_dec.atomics();

        let winning_position = {
            if now_price >= entry_price {
                Position::Long
            } else {
                Position::Short
            }
        };

        let mut trader_amount = match user_position {
            Position::Long => get_trader_amount(
                &user_position,
                &winning_position,
                entry_price,
                now_price,
                collateral_amount,
                config.asset_decimal,
                now_price_dec,
                leverage,
            ),
            Position::Short => get_trader_amount(
                &user_position,
                &winning_position,
                entry_price,
                now_price,
                collateral_amount,
                config.stable_decimal,
                stable_price_dec,
                leverage,
            ),
        }?;
        //@@ pnl 로 뜯어야함.
        let close_fee_amount =
            calculate_close_fee_amount(trader_amount, load_open_and_close_fee(deps.storage)?);
        let fee_usd = match user_position {
            Position::Long => {
                get_usd_amount(close_fee_amount, config.asset_decimal, now_price_dec)?
            }
            Position::Short => {
                get_usd_amount(close_fee_amount, config.stable_decimal, stable_price_dec)?
            }
        };
        trader_amount -= close_fee_amount;
        save_config_fee(deps.storage, &mut config, &user_position, close_fee_amount)?;

        let send_amount_to_pool =
            collateral_amount + leverage_amount - trader_amount - close_fee_amount;

        //포지션 맵에서 삭제
        trade_remove(deps.storage, trader.clone())?;

        //@@treasury msg 만들기
        let treasury_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: config.treasury_contract.to_string(),
            msg: to_binary(&TreasuryExecuteMsg::AddFeeAmount {
                asset_denom: config.asset_denom,
                stable_denom: config.stable_denom,
                trader: trader.to_string(),
                fee_usd_amount: fee_usd.to_uint_ceil(),
            })?,
            funds: vec![],
        });

        //@@bank msg 만들기
        let user_bank_msg = CosmosMsg::Bank(BankMsg::Send {
            to_address: trader.to_string(),
            amount: vec![coin(trader_amount.into(), denom.clone())],
        });
        //@@Execute Msg 만들기
        let execute_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: get_config_pool_contract(deps.storage)?.to_string(),
            msg: to_binary(&PoolExecuteMsg::RePay {
                denom: denom.clone(),
                position: user_position.convert_boolean(),
                amount: send_amount_to_pool,
                borrowed_amount: leverage_amount,
            })?,
            funds: vec![coin(send_amount_to_pool.into(), denom)],
        });

        Ok(Response::new()
            .add_message(user_bank_msg)
            .add_message(execute_msg)
            .add_message(treasury_msg))
    }

    pub fn hook_liquidated(
        deps: DepsMut<SeiQueryWrapper>,
        _env: Env,
        _info: MessageInfo,
    ) -> Result<Response<SeiMsg>, ContractError> {
        let mut config = load_config(deps.storage)?;
        let fee_config = load_fee_config(deps.storage)?;
        let FeeConfig {
            open_close_fee_rate,
            ..
        } = fee_config;
        let past_price = config.past_price;

        let (current_price, stable_price) =
            query_entry_and_stable_price(&deps.querier, &config.asset_denom, &config.stable_denom)?;
        update_past_price(&mut config, past_price);

        let price_destinated_trader = get_desitinated_price_traders(
            deps.storage,
            past_price.atomics(),
            current_price.atomics(),
        )?;

        let mut asset_coin_to_pool = coin(0, config.asset_denom.clone());
        let mut stable_coin_to_pool = coin(0, config.stable_denom.clone());
        let mut asset_borrowed_amount = Uint128::zero();
        let mut stable_borrowed_amount = Uint128::zero();
        let loss_bank_msgs = control_desitinated_traders(
            deps.storage,
            &mut config,
            current_price,
            stable_price,
            price_destinated_trader.limit_loss,
            &mut asset_coin_to_pool,
            &mut stable_coin_to_pool,
            open_close_fee_rate,
            &mut asset_borrowed_amount,
            &mut stable_borrowed_amount,
        )?;
        let profit_bank_msgs = control_desitinated_traders(
            deps.storage,
            &mut config,
            current_price,
            stable_price,
            price_destinated_trader.limit_profit,
            &mut asset_coin_to_pool,
            &mut stable_coin_to_pool,
            open_close_fee_rate,
            &mut asset_borrowed_amount,
            &mut stable_borrowed_amount,
        )?;
        let liquidated_msgs = control_desitinated_traders(
            deps.storage,
            &mut config,
            current_price,
            stable_price,
            price_destinated_trader.liquidated,
            &mut asset_coin_to_pool,
            &mut stable_coin_to_pool,
            open_close_fee_rate,
            &mut asset_borrowed_amount,
            &mut stable_borrowed_amount,
        )?;
        let mut bank_msgs = vec![];
        bank_msgs.extend(loss_bank_msgs);
        bank_msgs.extend(profit_bank_msgs);
        bank_msgs.extend(liquidated_msgs);
        //@@ fee_zero_reset is used fee division
        let (
            send_asset_fee_to_pool,
            send_stable_fee_to_pool,
            send_asset_fee_to_valut,
            send_stable_fee_to_valut,
        ) = fee_division(&mut config);

        let repay_stable_msg: CosmosMsg<SeiMsg> = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: get_config_pool_contract(deps.storage)?.to_string(),
            msg: to_binary(&PoolExecuteMsg::RePay {
                denom: config.stable_denom.clone(),
                position: false,
                amount: stable_coin_to_pool.amount + send_stable_fee_to_pool,
                borrowed_amount: stable_borrowed_amount,
            })?,
            funds: vec![stable_coin_to_pool],
        });
        let repay_asset_msg: CosmosMsg<SeiMsg> = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: get_config_pool_contract(deps.storage)?.to_string(),
            msg: to_binary(&PoolExecuteMsg::RePay {
                denom: config.asset_denom.clone(),
                position: true,
                amount: asset_coin_to_pool.amount + send_asset_fee_to_pool,
                borrowed_amount: asset_borrowed_amount,
            })?,
            funds: vec![asset_coin_to_pool],
        });
        let send_fee_to_valut_msg: CosmosMsg<SeiMsg> = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: config.fee_vault_contract.to_string(),
            msg: to_binary(&VaultExecuteMsg::RecievedFee {
                x_denom: config.asset_denom.clone(),
                x_amount: send_asset_fee_to_valut,
                y_denom: config.stable_denom.clone(),
                y_amount: send_stable_fee_to_valut,
            })?,
            funds: vec![
                coin(send_asset_fee_to_valut.into(), config.asset_denom),
                coin(send_stable_fee_to_valut.into(), config.stable_denom),
            ],
        });

        Ok(Response::new().add_messages(bank_msgs).add_messages(vec![
            repay_asset_msg,
            repay_stable_msg,
            send_fee_to_valut_msg,
        ]))
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps<SeiQueryWrapper>, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetConfig {} => to_binary(&query::get_config(deps)?),
        QueryMsg::GetFeeConfig {} => to_binary(&query::get_fee_config(deps)?),
        QueryMsg::GetTrade { trader } => to_binary(&query::get_trade(deps, trader)?),
    }
}

pub mod query {
    use rune::market::{GetConfigResponse, GetFeeConfigResponse, TradeResponse};

    use crate::{
        state::{load_config, load_fee_config},
        trade::{trades, Trade},
    };

    use super::*;

    pub fn get_config(deps: Deps<SeiQueryWrapper>) -> StdResult<GetConfigResponse> {
        let config = load_config(deps.storage)?;
        let Config {
            owner,
            asset_denom,
            stable_denom,
            asset_decimal,
            stable_decimal,
            max_leverage,
            pool_contract,
            fee_vault_contract,
            treasury_contract,
            asset_total_fee,
            stable_total_fee,
            past_price,
        } = config;
        Ok(GetConfigResponse {
            owner,
            asset_denom,
            stable_denom,
            asset_decimal,
            stable_decimal,
            max_leverage,
            pool_contract,
            fee_vault_contract,
            treasury_contract,
            asset_total_fee,
            stable_total_fee,
            past_price,
        })
    }
    pub fn get_fee_config(deps: Deps<SeiQueryWrapper>) -> StdResult<GetFeeConfigResponse> {
        let fee_config = load_fee_config(deps.storage)?;
        let FeeConfig {
            borrow_fee_rate,
            open_close_fee_rate,
            limit_profit_loss_open_fee_rate,
        } = fee_config;
        Ok(GetFeeConfigResponse {
            borrow_fee_rate,
            open_close_fee_rate,
            limit_profit_loss_open_fee_rate,
        })
    }

    pub fn get_trade(deps: Deps<SeiQueryWrapper>, trader: String) -> StdResult<TradeResponse> {
        let trader = deps.api.addr_validate(&trader)?;
        let trade = trades().load(deps.storage, trader.clone())?;
        let Trade {
            entry_price,
            trader,
            liquidation_price,
            limit_profit_price,
            limit_loss_price,
            collateral_denom,
            collateral_amount,
            position,
            position_size,
            leverage,
            leverage_amount,
        } = trade;
        Ok(TradeResponse {
            trader,
            entry_price,
            liquidation_price,
            limit_profit_price,
            limit_loss_price,
            collateral_denom,
            collateral_amount,
            position: position.convert_boolean(),
            position_size,
            leverage,
            leverage_amount,
        })
    }
}
