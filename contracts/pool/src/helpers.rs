use sei_cosmwasm::SeiMsg;

use cosmwasm_std::{coin, BankMsg, CosmosMsg, Decimal, Uint128};

use crate::error::ContractError;

pub fn create_bank_msg(
    accepted: Uint128,

    deposited: Uint128,
    denom: &str,
    to_address: &str,
) -> Option<CosmosMsg<SeiMsg>> {
    if deposited - accepted != Uint128::zero() {
        Some(CosmosMsg::Bank(BankMsg::Send {
            to_address: to_address.to_string(),
            amount: vec![coin((deposited - accepted).into(), denom)],
        }))
    } else {
        None
    }
}
pub fn calculate_lp_mint_amount(
    send_asset_amount: Uint128,
    send_stable_amount: Uint128,
    reserve_asset_amount: Uint128,
    reserve_stable_amount: Uint128,
    asset_decimal: u8,
    stable_decimal: u8,
    lp_total_supply: Uint128,
    lp_decimal: u8,
) -> Result<(Uint128, Uint128, Uint128), ContractError> {
    if send_asset_amount.is_zero() || send_stable_amount.is_zero() {
        return Err(ContractError::InvalidAmount {});
    }
    match lp_total_supply.is_zero() {
        true => {
            // let cx = send_asset_amount.u128().to_string().len() - 1; // characteristic of x
            // let cy = send_stable_amount.u128().to_string().len() - 1; // characteristic of y
            // let c = ((cx + 1) + (cy + 1) + 1) / 2; // ceil(((cx + 1) + (cy + 1)) / 2)
            // let res = 10u128.pow(c as u32); // 10^c
            // let lp_amount = Uint128::from(res);
            // println!("lp_init_amount = {:?}", lp_amount);
            // Ok((lp_amount, send_asset_amount, send_stable_amount))

            let asset_amount_dec =
                Decimal::from_atomics(send_asset_amount, asset_decimal.into()).unwrap();
            let stable_amount_dec =
                Decimal::from_atomics(send_stable_amount, stable_decimal.into()).unwrap();
            let sqrt_dec = (asset_amount_dec * stable_amount_dec).sqrt();
            let lp_amount = sqrt_dec * Uint128::new(10u128.pow(lp_decimal.into()));

            Ok((lp_amount, send_asset_amount, send_stable_amount))
        }
        false => {
            let expected_asset_amount =
                Decimal::from_ratio(send_stable_amount, reserve_stable_amount)
                    * reserve_asset_amount;
            let expected_stable_amount =
                Decimal::from_ratio(send_asset_amount, reserve_asset_amount)
                    * reserve_stable_amount;

            let accpeted_asset_amount = Uint128::min(send_asset_amount, expected_asset_amount);
            let accepted_stable_amount = Uint128::min(send_stable_amount, expected_stable_amount);
            let ratio = {
                let asset_ratio = Decimal::from_ratio(accpeted_asset_amount, reserve_asset_amount);
                let stable_ratio =
                    Decimal::from_ratio(accepted_stable_amount, reserve_stable_amount);

                Decimal::min(asset_ratio, stable_ratio)
            };
            let lp_mint_amount = lp_total_supply * ratio;

            Ok((
                lp_mint_amount,
                accpeted_asset_amount,
                accepted_stable_amount,
            ))
        }
    }
}

pub mod check {
    use cosmwasm_std::{Addr, Coin, Decimal, Storage, Uint128};

    use crate::{error::ContractError, state::load_market_contract};

    pub fn check_market_contract(
        storage: &mut dyn Storage,
        sender: &Addr,
    ) -> Result<(), ContractError> {
        let market_contract = load_market_contract(storage)?;

        match market_contract == sender {
            true => Ok(()),
            false => Err(ContractError::Unauthorized {}),
        }
    }

    pub fn check_maximum_leverage_amount(
        leverage_amount: Uint128,
        pool_amount: Uint128,
        maximum_borrow_rate: u8,
    ) -> Result<(), ContractError> {
        if leverage_amount > (pool_amount * Decimal::percent(maximum_borrow_rate.into())) {
            Err(ContractError::OverflowLeverage {})
        } else {
            Ok(())
        }
    }
    pub fn check_funds_and_get_funds(
        funds: Vec<Coin>,
        asset_denom: &String,
        stable_denom: &String,
    ) -> Result<(Coin, Coin), ContractError> {
        let asset = funds
            .iter()
            .find(|c| c.denom == *asset_denom)
            .ok_or_else(|| ContractError::InvalidDenom {})?;
        let stable = funds
            .iter()
            .find(|c| c.denom == *stable_denom)
            .ok_or_else(|| ContractError::InvalidDenom {})?;
        Ok((asset.clone(), stable.clone()))
    }
    pub fn check_lp_funds_and_get_lp_funds(
        funds: Vec<Coin>,
        lp_denom: &String,
    ) -> Result<Coin, ContractError> {
        let lp = funds
            .into_iter()
            .find(|c| c.denom == *lp_denom)
            .ok_or_else(|| ContractError::InvalidDenom {})?;
        Ok(lp)
    }
    pub fn check_repay_denom(
        funds: Vec<Coin>,
        denom: &String,
        amount: Uint128,
    ) -> Result<(), ContractError> {
        let coin = funds
            .into_iter()
            .find(|c| c.denom == *denom)
            .ok_or_else(|| ContractError::InvalidDenom {})?;
        match coin.amount == amount {
            true => Ok(()),
            false => Err(ContractError::InvalidAmount {}),
        }
    }
}
