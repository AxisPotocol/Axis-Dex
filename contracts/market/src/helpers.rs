use cosmwasm_std::{coin, Attribute, BankMsg, Coin, CosmosMsg, Decimal, Storage, Uint128};
use sei_cosmwasm::SeiMsg;

use crate::{
    error::ContractError,
    helpers::check::check_collateral_value,
    position::Position,
    state::{Config, State},
    trade::{trade_remove, PriceDestinatedStatus, Trade},
};
const MINIMUM_USD_VALUE: u8 = 10;
const PRICE_DECIMAL: u32 = 18;
const LEVERAGE_DECIAML: u32 = 0;
pub fn calculate_open_fee_amount(
    collateral_amount: Uint128,
    leverage: u8,
    fee_rate: u8,
) -> Uint128 {
    collateral_amount * Uint128::new(leverage.into()) * Decimal::permille(fee_rate.into())
}
pub fn calculate_close_fee_amount(trader_amount: Uint128, fee_rate: u8) -> Uint128 {
    trader_amount * Decimal::permille(fee_rate.into())
}
pub fn calculate_position_size(collateral_amount: Uint128, leverage: u8) -> Uint128 {
    collateral_amount * Uint128::new(leverage.into())
}
pub fn calculate_percentage(amount: Uint128, percent: u64) -> Uint128 {
    amount * Decimal::percent(percent)
}
pub fn get_collateral_usd(
    collateral_amount: Uint128,
    collateral_decimal: u8,
    collateral_price: Decimal,
) -> Result<Decimal, ContractError> {
    let collateral_usd = Decimal::from_atomics(collateral_amount, collateral_decimal.into())
        .map_err(|_| ContractError::ConvertError {})?
        * collateral_price;
    Ok(collateral_usd)
}
pub fn get_usd_amount(
    base_amount: Uint128,
    base_decimal: u8,
    base_price: Decimal,
) -> Result<Decimal, ContractError> {
    let usd = Decimal::from_atomics(base_amount, base_decimal.into())
        .map_err(|_| ContractError::ConvertError {})?
        * base_price;
    Ok(usd)
}
//@@ 청산 금액 계산
//@@ decimal 로 하는게 맞나?
pub fn get_liquidation_price(
    entry_price: Decimal,
    collateral_price: Decimal,
    collateral_amount: Uint128,
    collateral_decimal: u8,
    open_fee_amount: Uint128,
    leverage: u8,
    position: &Position,
) -> Result<Decimal, ContractError> {
    let collateral_usd =
        get_collateral_usd(collateral_amount, collateral_decimal, collateral_price)?;

    //@@CollateralUSD is always greater than 10usd. Check if it is greater than 10 in the main function.

    check_collateral_value(collateral_usd, MINIMUM_USD_VALUE)?;

    let fee_usd = get_usd_amount(open_fee_amount, collateral_decimal, collateral_price)?;
    println!("open_fee_amount = {:?}", open_fee_amount);

    let leverage_dec =
        Decimal::from_atomics(leverage, 0).map_err(|_| ContractError::ConvertError {})?;

    //roll over_FEE
    //@@Open Price * (Collateral usd * 0.9 +fee) / Collateral usd / Leverage.

    let liquidation_destination = entry_price * (collateral_usd * Decimal::percent(90) + fee_usd)
        / collateral_usd
        / leverage_dec;

    let liquidation_price = match position {
        Position::Long => entry_price - liquidation_destination,
        Position::Short => entry_price + liquidation_destination,
    };

    Ok(liquidation_price)
}

//@@ 레버리지금액 계산
pub fn get_leverage_amount(
    collateral_amount: Uint128,
    leverage: u8,
) -> Result<Uint128, ContractError> {
    let leverage_amount = collateral_amount
        .checked_mul(Uint128::new(leverage.into()))
        .map_err(|_| ContractError::OverflowError {})?;
    Ok(leverage_amount)
}

pub fn get_trader_amount(
    trader_position: &Position,
    winning_position: &Position,
    entry_price: Uint128,
    current_price: Uint128,
    collateral_amount: Uint128,
    collateral_decimal: u8,
    collateral_price: Decimal,
    leverage: u8,
) -> Result<Uint128, ContractError> {
    let entry_price = Decimal::from_atomics(entry_price, PRICE_DECIMAL).unwrap();
    let leverage = Decimal::from_atomics(leverage, LEVERAGE_DECIAML).unwrap();
    let current_price = Decimal::from_atomics(current_price, PRICE_DECIMAL).unwrap();

    let past_leverage = entry_price * leverage;
    let current_leverage = current_price * leverage;

    let (profit_position, loss_position) = match winning_position == trader_position {
        true => (trader_position, winning_position),
        false => (winning_position, trader_position),
    };

    let profit_usd = match profit_position {
        Position::Long => current_leverage - past_leverage,
        Position::Short => past_leverage - current_leverage,
    };

    let loss_usd = match loss_position {
        Position::Long => past_leverage - current_leverage,
        Position::Short => current_leverage - past_leverage,
    };

    let one_coin_amount = Uint128::new(10u128.pow(collateral_decimal.into()));

    let profit_amount = (profit_usd / collateral_price) * one_coin_amount;
    let loss_amount = (loss_usd / collateral_price) * one_coin_amount;

    let trader_amount = match winning_position {
        Position::Long => collateral_amount + profit_amount,
        Position::Short => collateral_amount
            .checked_sub(loss_amount)
            .unwrap_or(Uint128::zero()),
    };

    Ok(trader_amount)
}
// pub fn get_trader_amount(
//     trader_position: &Position,
//     winning_position: &Position,
//     entry_price: Uint128,
//     current_price: Uint128,
//     collateral_amount: Uint128,
//     collateral_decimal: u8,
//     collateral_price: Decimal,
//     leverage: u8,
// ) -> Result<Uint128, ContractError> {
//     let entry_price = Decimal::from_atomics(entry_price, PRICE_DECIMAL).unwrap();
//     let leverage = Decimal::from_atomics(leverage, LEVERAGE_DECIAML).unwrap();
//     let current_price = Decimal::from_atomics(current_price, PRICE_DECIMAL).unwrap();

//     let past_leverage = entry_price * leverage;

//     let current_leverage = current_price * leverage;

//     let trader_amount = match winning_position == trader_position {
//         true => get_trader_profit_amount(
//             past_leverage,
//             current_leverage,
//             collateral_amount,
//             collateral_decimal,
//             collateral_price,
//             trader_position,
//         )?,
//         false => get_trader_loss_amount(
//             past_leverage,
//             current_leverage,
//             collateral_amount,
//             collateral_decimal,
//             collateral_price,
//             trader_position,
//         )?,
//     };
//     Ok(trader_amount)
// }

// pub fn get_trader_loss_amount(
//     past_leverage: Decimal,
//     current_leverage: Decimal,
//     collateral_amount: Uint128,
//     collateral_decimal: u8,
//     collateral_price: Decimal,
//     trader_position: &Position,
// ) -> Result<Uint128, ContractError> {
//     let loss_usd = match trader_position {
//         Position::Long => past_leverage - current_leverage,
//         Position::Short => current_leverage - past_leverage,
//     };

//     let one_coin_amount = Uint128::new(10u128.pow(collateral_decimal.into()));
//     let loss_amount = (loss_usd / collateral_price) * one_coin_amount;

//     let trader_amount = collateral_amount
//         .checked_sub(loss_amount)
//         .unwrap_or(Uint128::new(0));

//     Ok(trader_amount)
// }

// pub fn get_trader_profit_amount(
//     past_leverage: Decimal,
//     current_leverage: Decimal,
//     collateral_amount: Uint128,
//     collateral_decimal: u8,
//     collateral_price: Decimal,
//     trader_position: &Position,
// ) -> Result<Uint128, ContractError> {
//     let profit_usd = match trader_position {
//         Position::Long => current_leverage - past_leverage,
//         Position::Short => past_leverage - current_leverage,
//     };

//     let one_coin_amount = Uint128::new(10u128.pow(collateral_decimal.into()));

//     let profit_amount = (profit_usd / collateral_price) * one_coin_amount; //5 /

//     let trader_amount = collateral_amount + profit_amount;

//     Ok(trader_amount)
// }

//@@ This function is return bank_msgs and leveraged amount
pub fn control_desitinated_traders(
    storage: &mut dyn Storage,
    config: &Config,
    state: &mut State,
    bank_msgs: &mut Vec<CosmosMsg<SeiMsg>>,
    base_price: Decimal,
    stable_price: Decimal,
    trader_status: PriceDestinatedStatus,
    base_coin_to_pool: &mut Coin,
    stable_coin_to_pool: &mut Coin,
    base_leveraged_amount: &mut Uint128,
    stable_leveraged_amount: &mut Uint128,
) -> Result<(), ContractError> {
    //@@ This variables for recording open interest in a pool contract

    match trader_status {
        PriceDestinatedStatus::LimitLoss(trader) => {
            for trade in trader.into_iter() {
                let Trade {
                    collateral_amount,
                    trader,
                    entry_price,
                    leverage,
                    leverage_amount,
                    limit_loss_price,
                    position: trade_position,
                    collateral_denom: denom,
                    ..
                } = trade;
                match trade_position {
                    Position::Long => *base_leveraged_amount += leverage_amount,
                    Position::Short => *stable_leveraged_amount += leverage_amount,
                }

                let mut trader_amount = match trade_position {
                    Position::Long => get_trader_amount(
                        &trade_position,
                        &Position::Short,
                        entry_price,
                        limit_loss_price,
                        collateral_amount,
                        config.base_decimal,
                        base_price,
                        leverage,
                    )?,
                    Position::Short => get_trader_amount(
                        &trade_position,
                        &Position::Long,
                        entry_price,
                        limit_loss_price,
                        collateral_amount,
                        config.price_decimal,
                        stable_price,
                        leverage,
                    )?,
                };

                let close_fee_amount =
                    calculate_close_fee_amount(trader_amount, config.open_close_fee_rate);
                trader_amount -= close_fee_amount;

                let bank_msg: CosmosMsg<SeiMsg> = CosmosMsg::Bank(BankMsg::Send {
                    to_address: trader.to_string(),
                    amount: vec![coin(trader_amount.into(), denom)],
                });
                bank_msgs.push(bank_msg);
                let send_amount_to_pool =
                    collateral_amount + leverage_amount - trader_amount - close_fee_amount;

                match trade_position {
                    Position::Long => {
                        base_coin_to_pool.amount += send_amount_to_pool;
                        state.base_coin_total_fee += close_fee_amount;
                    }
                    Position::Short => {
                        stable_coin_to_pool.amount += send_amount_to_pool;
                        state.price_coin_total_fee += close_fee_amount
                    }
                }
                trade_remove(storage, trader)?;
            }
        }
        PriceDestinatedStatus::Liquidated(trader) => {
            for trade in trader.into_iter() {
                //청산이니까 전부 풀에 보내면됨.
                //청산시 보증금의 0.1% 를 공제

                let close_fee_amount =
                    calculate_close_fee_amount(trade.collateral_amount, config.open_close_fee_rate);

                let send_amount_to_pool =
                    trade.collateral_amount + trade.leverage_amount - close_fee_amount;

                match trade.position {
                    Position::Long => {
                        base_coin_to_pool.amount += send_amount_to_pool;
                        state.base_coin_total_fee += close_fee_amount;
                        *base_leveraged_amount += trade.leverage_amount;
                    }
                    Position::Short => {
                        stable_coin_to_pool.amount += send_amount_to_pool;
                        state.price_coin_total_fee += close_fee_amount;
                        *stable_leveraged_amount += trade.leverage_amount;
                    }
                }
                trade_remove(storage, trade.trader)?;
            }
        }

        PriceDestinatedStatus::LimitProfit(trader) => {
            for trade in trader.into_iter() {
                let Trade {
                    collateral_amount,
                    trader,
                    entry_price,
                    leverage,
                    leverage_amount,
                    limit_profit_price,
                    position: trade_position,
                    collateral_denom: denom,
                    ..
                } = trade;
                //pool

                let mut trader_amount = match trade_position {
                    Position::Long => get_trader_amount(
                        &trade_position,
                        &Position::Long,
                        entry_price,
                        limit_profit_price,
                        collateral_amount,
                        config.base_decimal,
                        base_price,
                        leverage,
                    )?,
                    Position::Short => get_trader_amount(
                        &trade_position,
                        &Position::Short,
                        entry_price,
                        limit_profit_price,
                        collateral_amount,
                        config.price_decimal,
                        stable_price,
                        leverage,
                    )?,
                };
                let close_fee_amount =
                    calculate_close_fee_amount(trader_amount, config.open_close_fee_rate);

                trader_amount -= close_fee_amount;

                let bank_msg: CosmosMsg<SeiMsg> = CosmosMsg::Bank(BankMsg::Send {
                    to_address: trader.to_string(),
                    amount: vec![coin(trader_amount.into(), denom)],
                });
                bank_msgs.push(bank_msg);
                let send_amount_to_pool =
                    collateral_amount + leverage_amount - trader_amount - close_fee_amount;
                match trade_position {
                    Position::Long => {
                        base_coin_to_pool.amount += send_amount_to_pool;
                        state.base_coin_total_fee += close_fee_amount;
                        *base_leveraged_amount += leverage_amount;
                    }
                    Position::Short => {
                        stable_coin_to_pool.amount += send_amount_to_pool;
                        state.price_coin_total_fee += close_fee_amount;
                        *stable_leveraged_amount += leverage_amount;
                    }
                }
                trade_remove(storage, trader)?;
            }
        }
    }
    Ok(())
}

pub fn fee_division(state: &mut State) -> (Uint128, Uint128, Uint128, Uint128) {
    let send_base_fee_to_pool = state.base_coin_total_fee * Decimal::percent(90);
    let send_price_fee_to_pool = state.price_coin_total_fee * Decimal::percent(90);
    let send_base_fee_to_valut = state.base_coin_total_fee * Decimal::percent(10);
    let send_price_fee_to_valut = state.price_coin_total_fee * Decimal::percent(10);

    state.base_coin_total_fee = Uint128::zero();
    state.price_coin_total_fee = Uint128::zero();
    (
        send_base_fee_to_pool,
        send_price_fee_to_pool,
        send_base_fee_to_valut,
        send_price_fee_to_valut,
    )
}

pub fn get_trade_information(
    entry_price: Decimal,
    collateral_price: Decimal,
    collateral_amount: Uint128,
    collateral_decimal: u8,
    open_fee_amount: Uint128,
    leverage: u8,
    position: &Position,
) -> Result<(Uint128, Uint128, Uint128), ContractError> {
    let position_size = calculate_position_size(collateral_amount, leverage);

    let leverage_amount = get_leverage_amount(collateral_amount, leverage)?;

    let liquidation_price = get_liquidation_price(
        entry_price,
        collateral_price,
        collateral_amount,
        collateral_decimal,
        open_fee_amount,
        leverage,
        position,
    )?
    .atomics();

    Ok((position_size, leverage_amount, liquidation_price))
}

pub mod check {
    use cosmwasm_std::{Coin, Decimal, Uint128};

    use crate::{error::ContractError, position::Position, state::Config};

    pub fn check_leverage_amount(
        pool_balance: Uint128,
        leverage_amount: Uint128,
    ) -> Result<(), ContractError> {
        //trade_amount 는 pool balance 보다 작아야함.
        match leverage_amount.le(&(pool_balance * Decimal::percent(10))) {
            true => Ok(()),
            false => Err(ContractError::LeverageAmountBigerThanPoolBalance {}),
        }
    }

    pub fn check_leverage_rate(leverage: u8, max_leverage: u8) -> Result<(), ContractError> {
        if leverage > 0 && leverage <= max_leverage {
            Ok(())
        } else {
            Err(ContractError::InvalidLeverage {})
        }
    }

    pub fn check_funds_for_positions_get_funds(
        funds: Vec<Coin>,
        config: &Config,
        position: &Position,
    ) -> Result<(String, Uint128), ContractError> {
        let collateral = match position {
            Position::Long => funds
                .into_iter()
                .find(|c| c.denom == config.base_denom)
                .ok_or_else(|| ContractError::InvalidDenom {})?,

            Position::Short => funds
                .into_iter()
                .find(|c| c.denom == config.price_denom)
                .ok_or_else(|| ContractError::InvalidDenom {})?,
        };

        Ok((collateral.denom, collateral.amount))
    }
    pub fn check_collateral_value(
        collateral_usd: Decimal,
        minimum_usd: u8,
    ) -> Result<(), ContractError> {
        if collateral_usd < Decimal::from_atomics(minimum_usd, 0).unwrap() {
            Err(ContractError::LowFunds {})
        } else {
            Ok(())
        }
    }
}
