use axis_protocol::axis::ExecuteMsg as AxisExecuteMsg;
use axis_protocol::market::{ExecuteMsg, InstantiateMsg, QueryMsg};
use axis_protocol::pool::ExecuteMsg as PoolExecuteMsg;
use axis_protocol::vault::ExecuteMsg as VaultExecuteMsg;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Addr, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint128,
};
use cw2::set_contract_version;
use sei_cosmwasm::{SeiMsg, SeiQueryWrapper};

use crate::error::ContractError;

use crate::query::query_base_coin_price_and_price_coin_price;
use crate::state::{save_state, Config, State, CONFIG};

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
        base_denom,
        base_decimal,
        price_denom,
        price_decimal,
        max_leverage,
        borrow_fee_rate,
        open_close_fee_rate,
        limit_profit_loss_open_fee_rate,
        axis_contract,
        vault_contract,
    } = msg;

    let (past_price, _) =
        query_base_coin_price_and_price_coin_price(&deps.querier, &base_denom, &price_denom)?;
    let config = Config {
        owner: info.sender.to_owned(),
        base_denom: base_denom.to_owned(),
        base_decimal,
        price_denom: price_denom.to_owned(),
        price_decimal,
        max_leverage,
        pool_contract: info.sender.to_owned(),
        vault_contract,
        axis_contract,
        borrow_fee_rate,
        open_close_fee_rate,
        limit_profit_loss_open_fee_rate,
    };
    let state = State {
        base_coin_total_fee: Uint128::zero(),
        price_coin_total_fee: Uint128::zero(),
        past_price,
    };

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    CONFIG.save(deps.storage, &config)?;
    save_state(deps.storage, &state)?;

    Ok(Response::new()
        .add_attribute("method", "instantiate")
        .add_attribute("owner", info.sender)
        .add_attribute("base_denom", base_denom)
        .add_attribute("price_denom", price_denom)
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
        query::{query_base_coin_price_and_price_coin_price, query_pool_balance},
        state::{load_config, load_state, update_past_price},
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
        let config = load_config(deps.storage)?;
        let mut state = load_state(deps.storage)?;
        let position = Position::new(position);
        check_leverage_rate(leverage, config.max_leverage)?;
        //fund 확인
        let (collateral_denom, collateral_amount) =
            check_funds_for_positions_get_funds(info.funds, &config, &position)?;

        //@@ entry price 가격 받아오기 난중에 Hook 로 빼자. config 로 뺄것.
        let (entry_price, price_price) = query_base_coin_price_and_price_coin_price(
            &deps.querier,
            &config.base_denom,
            &config.price_denom,
        )?;

        //funds 최소 금액 확인

        let open_fee_amount = {
            match (limit_loss_price, limit_profit_price) {
                (None, None) => calculate_open_fee_amount(
                    collateral_amount,
                    leverage,
                    config.open_close_fee_rate,
                ),
                (None, Some(_)) | (Some(_), None) | (Some(_), Some(_)) => {
                    calculate_open_fee_amount(
                        collateral_amount,
                        leverage,
                        config.limit_profit_loss_open_fee_rate,
                    )
                }
            }
        };
        match position {
            Position::Long => state.base_coin_total_fee += open_fee_amount,
            Position::Short => state.price_coin_total_fee += open_fee_amount,
        }
        save_state(deps.storage, &state)?;
        let collateral_amount = collateral_amount - open_fee_amount;

        //@@ 이 로직 확인!

        let (position_size, leverage_amount, liquidation_price) = {
            match position {
                Position::Long => get_trade_information(
                    entry_price,
                    entry_price,
                    collateral_amount,
                    config.base_decimal,
                    open_fee_amount,
                    leverage,
                    &position,
                )?,
                Position::Short => get_trade_information(
                    entry_price,
                    price_price,
                    collateral_amount,
                    config.price_decimal,
                    open_fee_amount,
                    leverage,
                    &position,
                )?,
            }
        };

        let pool_balance = query_pool_balance(deps.querier, &config.pool_contract, &position)?;

        //@@ 풀의 몇% 까지? 정해지면 로직넣어야함.
        //@@ pool 에서도 로직 있음.
        check_leverage_amount(pool_balance, leverage_amount)?;

        //info 만들기
        let trade = Trade::new(
            info.sender.to_owned(),
            entry_price.atomics(),
            liquidation_price,
            limit_profit_price,
            limit_loss_price,
            collateral_denom.to_owned(),
            collateral_amount,
            position.to_owned(),
            position_size,
            leverage,
            leverage_amount,
        );

        //Trade 저장하는 로직.
        trade_update(deps.storage, info.sender.to_owned(), trade)?;

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
            Position::Long => get_usd_amount(open_fee_amount, config.base_decimal, entry_price)?,
            Position::Short => get_usd_amount(open_fee_amount, config.price_decimal, price_price)?,
        };

        //@@ attribute 만들어야함.
        Ok(Response::new()
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: config.pool_contract.to_string(),
                msg: to_binary(&execute_msg)?,
                funds: vec![],
            }))
            .add_message(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: config.axis_contract.to_string(),
                msg: to_binary(&AxisExecuteMsg::AddFeeAmount {
                    base_denom: config.base_denom,
                    price_denom: config.price_denom,
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
        let config = load_config(deps.storage)?;
        let mut state = load_state(deps.storage)?;
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

        let (now_price_dec, price_price_dec) = query_base_coin_price_and_price_coin_price(
            &deps.querier,
            &config.base_denom,
            &config.price_denom,
        )?;

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
                config.base_decimal,
                now_price_dec,
                leverage,
            ),
            Position::Short => get_trader_amount(
                &user_position,
                &winning_position,
                entry_price,
                now_price,
                collateral_amount,
                config.price_decimal,
                price_price_dec,
                leverage,
            ),
        }?;
        //@@ pnl 로 뜯어야함.
        let close_fee_amount =
            calculate_close_fee_amount(trader_amount, config.open_close_fee_rate);
        let fee_usd = match user_position {
            Position::Long => get_usd_amount(close_fee_amount, config.base_decimal, now_price_dec)?,
            Position::Short => {
                get_usd_amount(close_fee_amount, config.price_decimal, price_price_dec)?
            }
        };
        trader_amount -= close_fee_amount;

        match user_position {
            Position::Long => state.base_coin_total_fee += close_fee_amount,
            Position::Short => state.price_coin_total_fee += close_fee_amount,
        }
        save_state(deps.storage, &state)?;
        let send_amount_to_pool =
            collateral_amount + leverage_amount - trader_amount - close_fee_amount;

        //포지션 맵에서 삭제
        trade_remove(deps.storage, trader.to_owned())?;

        //@@treasury msg 만들기
        let treasury_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: config.axis_contract.to_string(),
            msg: to_binary(&AxisExecuteMsg::AddFeeAmount {
                base_denom: config.base_denom,
                price_denom: config.price_denom,
                trader: trader.to_string(),
                fee_usd_amount: fee_usd.to_uint_ceil(),
            })?,
            funds: vec![],
        });

        //@@bank msg 만들기
        let user_bank_msg = CosmosMsg::Bank(BankMsg::Send {
            to_address: trader.to_string(),
            amount: vec![coin(trader_amount.into(), denom.to_owned())],
        });
        //@@Execute Msg 만들기
        let execute_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: config.pool_contract.to_string(),
            msg: to_binary(&PoolExecuteMsg::RePay {
                denom: denom.to_owned(),
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
        let mut state = load_state(deps.storage)?;
        let past_price = state.past_price;

        let (current_price, price_price) = query_base_coin_price_and_price_coin_price(
            &deps.querier,
            &config.base_denom,
            &config.price_denom,
        )?;
        update_past_price(&mut state, past_price);

        let price_destinated_trader = get_desitinated_price_traders(
            deps.storage,
            past_price.atomics(),
            current_price.atomics(),
        )?;

        let mut base_coin_to_pool = coin(0, config.base_denom.to_owned());
        let mut price_coin_to_pool = coin(0, config.price_denom.to_owned());
        let mut base_borrowed_amount = Uint128::zero();
        let mut price_borrowed_amount = Uint128::zero();
        let loss_bank_msgs = control_desitinated_traders(
            deps.storage,
            &mut config,
            &mut state,
            current_price,
            price_price,
            price_destinated_trader.limit_loss,
            &mut base_coin_to_pool,
            &mut price_coin_to_pool,
            &mut base_borrowed_amount,
            &mut price_borrowed_amount,
        )?;
        let profit_bank_msgs = control_desitinated_traders(
            deps.storage,
            &mut config,
            &mut state,
            current_price,
            price_price,
            price_destinated_trader.limit_profit,
            &mut base_coin_to_pool,
            &mut price_coin_to_pool,
            &mut base_borrowed_amount,
            &mut price_borrowed_amount,
        )?;
        let liquidated_msgs = control_desitinated_traders(
            deps.storage,
            &mut config,
            &mut state,
            current_price,
            price_price,
            price_destinated_trader.liquidated,
            &mut base_coin_to_pool,
            &mut price_coin_to_pool,
            &mut base_borrowed_amount,
            &mut price_borrowed_amount,
        )?;
        let mut bank_msgs = vec![];
        bank_msgs.extend(loss_bank_msgs);
        bank_msgs.extend(profit_bank_msgs);
        bank_msgs.extend(liquidated_msgs);
        //@@ fee_zero_reset is used fee division
        let (
            send_base_fee_to_pool,
            send_price_fee_to_pool,
            send_base_fee_to_valut,
            send_price_fee_to_valut,
        ) = fee_division(&mut state);
        save_state(deps.storage, &state)?;
        let repay_price_msg: CosmosMsg<SeiMsg> = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: config.pool_contract.to_string(),
            msg: to_binary(&PoolExecuteMsg::RePay {
                denom: config.price_denom.to_owned(),
                position: false,
                amount: price_coin_to_pool.amount + send_price_fee_to_pool,
                borrowed_amount: price_borrowed_amount,
            })?,
            funds: vec![price_coin_to_pool],
        });
        let repay_base_msg: CosmosMsg<SeiMsg> = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: config.pool_contract.to_string(),
            msg: to_binary(&PoolExecuteMsg::RePay {
                denom: config.base_denom.to_owned(),
                position: true,
                amount: base_coin_to_pool.amount + send_base_fee_to_pool,
                borrowed_amount: base_borrowed_amount,
            })?,
            funds: vec![base_coin_to_pool],
        });
        let send_fee_to_valut_msg: CosmosMsg<SeiMsg> = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: config.vault_contract.to_string(),
            msg: to_binary(&VaultExecuteMsg::RecievedFee {
                denom: config.base_denom.to_owned(),
                amount: send_base_fee_to_valut,
            })?,
            funds: vec![
                coin(send_base_fee_to_valut.into(), config.base_denom),
                coin(send_price_fee_to_valut.into(), config.price_denom),
            ],
        });

        Ok(Response::new().add_messages(bank_msgs).add_messages(vec![
            repay_base_msg,
            repay_price_msg,
            send_fee_to_valut_msg,
        ]))
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps<SeiQueryWrapper>, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetConfig {} => to_binary(&query::get_config(deps)?),
        QueryMsg::GetState {} => to_binary(&query::get_state(deps)?),
        QueryMsg::GetTrade { trader } => to_binary(&query::get_trade(deps, trader)?),
    }
}

pub mod query {
    use axis_protocol::market::{GetConfigResponse, GetStateResponse, TradeResponse};

    use crate::{
        state::{load_config, load_state},
        trade::{trades, Trade},
    };

    use super::*;

    pub fn get_config(deps: Deps<SeiQueryWrapper>) -> StdResult<GetConfigResponse> {
        let config = load_config(deps.storage)?;
        let Config {
            owner,
            base_denom,
            price_denom,
            base_decimal,
            price_decimal,
            max_leverage,
            pool_contract,
            vault_contract,
            axis_contract,
            borrow_fee_rate,
            open_close_fee_rate,
            limit_profit_loss_open_fee_rate,
        } = config;

        Ok(GetConfigResponse {
            base_denom,
            price_denom,
            base_decimal,
            price_decimal,
            max_leverage,
            pool_contract,
            vault_contract,
            axis_contract,
            borrow_fee_rate,
            open_close_fee_rate,
            limit_profit_loss_open_fee_rate,
        })
    }
    pub fn get_state(deps: Deps<SeiQueryWrapper>) -> StdResult<GetStateResponse> {
        let state = load_state(deps.storage)?;
        let State {
            base_coin_total_fee,
            price_coin_total_fee,
            past_price,
        } = state;
        Ok(GetStateResponse {
            base_coin_total_fee,
            price_coin_total_fee,
            past_price,
        })
    }

    pub fn get_trade(deps: Deps<SeiQueryWrapper>, trader: String) -> StdResult<TradeResponse> {
        let trader = deps.api.addr_validate(&trader)?;
        let trade = trades().load(deps.storage, trader.to_owned())?;
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
