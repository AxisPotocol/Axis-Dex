use std::str::FromStr;

use cosmwasm_std::{coin, BankMsg, CosmosMsg, Decimal, Uint128, Uint64};
use rune::pool::{ConfigResponse, QueryMsg};

use sei_integration_tests::helper::mock_app;

use crate::{
    helpers::{
        calculate_lp_mint_amount,
        check::{
            check_funds_and_get_funds, check_lp_funds_and_get_lp_funds,
            check_maximum_leverage_amount, check_repay_denom,
        },
        create_bank_msg,
    },
    testing::app::init_exchange_rates,
};

use super::app::{init_default_balances, setup_test, ASSET_DENOM, STABLE_DENOM, TRADER1};

#[test]
fn test_calculate_lp_mint_amount() {
    //init test
    let asset_amount = Uint128::new(1000);
    let stable_amount = Uint128::new(1000);
    let reserve_asset_amount = Uint128::zero();
    let reserve_stable_amount = Uint128::zero();
    let lp_total_supply = Uint128::zero();
    let asset_decimal = 6;
    let stable_decimal = 6;
    let lp_decimal = 6;
    let (lp_mint_amount, accept_asset, accept_stable) = calculate_lp_mint_amount(
        asset_amount,
        stable_amount,
        reserve_asset_amount,
        reserve_stable_amount,
        asset_decimal,
        stable_decimal,
        lp_total_supply,
        lp_decimal,
    )
    .unwrap();
    assert_eq!(lp_mint_amount.u128(), 1000);
    assert_eq!(accept_asset.u128(), 1000);
    assert_eq!(accept_stable.u128(), 1000);

    //invalidAmount
    let asset_amount = Uint128::new(0);
    let stable_amount = Uint128::new(1000);
    let reserve_asset_amount = Uint128::zero();
    let reserve_stable_amount = Uint128::zero();
    let lp_total_supply = Uint128::zero();

    let result = calculate_lp_mint_amount(
        asset_amount,
        stable_amount,
        reserve_asset_amount,
        reserve_stable_amount,
        asset_decimal,
        stable_decimal,
        lp_total_supply,
        lp_decimal,
    );
    assert!(result.is_err());

    let asset_amount = Uint128::new(10);
    let stable_amount = Uint128::new(1000);
    let reserve_asset_amount = Uint128::new(10000);
    let reserve_stable_amount = Uint128::new(10000);
    let lp_total_supply = Uint128::new(10000);
    let (lp_mint_amount, accept_asset, accept_stable) = calculate_lp_mint_amount(
        asset_amount,
        stable_amount,
        reserve_asset_amount,
        reserve_stable_amount,
        asset_decimal,
        stable_decimal,
        lp_total_supply,
        lp_decimal,
    )
    .unwrap();
    assert_eq!(lp_mint_amount, Uint128::new(10));
    assert_eq!(accept_asset, Uint128::new(10));
    assert_eq!(accept_stable, Uint128::new(10));
}

#[test]
fn test_create_bank_msg() {
    let accepted = Uint128::new(10);
    let deposited = Uint128::new(100);
    let bank_msg = create_bank_msg(accepted, deposited, ASSET_DENOM, TRADER1);
    assert!(bank_msg.is_some());
    assert_eq!(
        bank_msg.unwrap(),
        CosmosMsg::Bank(BankMsg::Send {
            to_address: TRADER1.to_string(),
            amount: vec![coin(90, ASSET_DENOM)],
        })
    )
}

#[test]
fn test_register_market_contract() {
    let mut app = mock_app(init_default_balances, init_exchange_rates());
    let (pool_contract, maket_contract) =
        setup_test(&mut app, Uint128::new(1000), Uint128::new(1000));
    let msg = QueryMsg::GetConfig {};
    let result: ConfigResponse = app.wrap().query_wasm_smart(pool_contract, &msg).unwrap();

    assert_eq!(maket_contract, result.market_contract);
}

#[test]
fn test_check_maximum_leverage_amount() {
    //proper
    let maximum_borrow_rate: u8 = 10;
    let leverage_amount = Uint128::new(100);
    let pool_amount = Uint128::new(1000);
    let result = check_maximum_leverage_amount(leverage_amount, pool_amount, maximum_borrow_rate);
    assert!(result.is_ok());

    //fail test
    let leverage_amount = Uint128::new(100);
    let pool_amount = Uint128::new(999);
    let result = check_maximum_leverage_amount(leverage_amount, pool_amount, maximum_borrow_rate);
    assert!(result.is_err());
}

#[test]
fn test_check_funds_and_get_funds() {
    let funds = vec![coin(1000, "ubtc"), coin(2000, "uusdc")];
    let asset_denom = "ubtc".to_string();
    let stable_denom = "uusdc".to_string();

    let result = check_funds_and_get_funds(funds, &asset_denom, &stable_denom);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), (coin(1000, "ubtc"), coin(2000, "uusdc")));

    //fail test

    let funds = vec![coin(1000, "ubtc"), coin(2000, "uusdt")];
    let asset_denom = "ubtc".to_string();
    let stable_denom = "uusdc".to_string();

    let result = check_funds_and_get_funds(funds, &asset_denom, &stable_denom);
    assert!(result.is_err());
}
#[test]
fn test_check_lp_funds_and_get_lp_funds() {
    let mut app = mock_app(init_default_balances, init_exchange_rates());
    let (pool_contract, _) = setup_test(&mut app, Uint128::new(1_000), Uint128::new(1_000));

    let lp_token_denom = "factory/".to_string() + pool_contract.as_ref() + "/lp";
    let lp_token = coin(10000, lp_token_denom.clone());

    let result = check_lp_funds_and_get_lp_funds(vec![lp_token], &lp_token_denom);
    assert!(result.is_ok());
    assert_eq!(lp_token_denom, result.unwrap().denom);
}

#[test]
fn test_check_repay_denom() {
    let funds = vec![coin(2000, "ubtc")];
    let repay_denom = "ubtc".to_string();
    let repay_amount = Uint128::new(2000);

    let result = check_repay_denom(funds, &repay_denom, repay_amount);

    assert!(result.is_ok());
    //@@@Fail test

    //Incorrect denom fund
    let funds = vec![coin(2000, "ubtc")];
    let repay_denom = "uusdc".to_string();
    let repay_amount = Uint128::new(2000);
    let result = check_repay_denom(funds, &repay_denom, repay_amount);
    assert!(result.is_err());
    //Incorrect amount
    let funds = vec![coin(2000, "ubtc")];
    let repay_denom = "ubtc".to_string();
    let repay_amount = Uint128::new(1000);
    let result = check_repay_denom(funds, &repay_denom, repay_amount);
    assert!(result.is_err());
}
