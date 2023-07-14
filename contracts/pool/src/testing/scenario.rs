use axis_protocol::{
    market::{GetConfigResponse as MarketGetConfigResponse, QueryMsg as MarketQueryMsg},
    pool::{ConfigResponse, ExecuteMsg, PoolResponse, QueryMsg},
};
use cosmwasm_std::{coin, Addr, Uint128};
use cw_multi_test::Executor;

use sei_integration_tests::helper::mock_app;

use crate::testing::app::init_exchange_rates;

use super::app::{init_default_balances, setup_test, ASSET_DENOM, STABLE_DENOM, TRADER1, TRADER2};

#[test]
fn proper_instantiate_contract() {
    let mut app = mock_app(init_default_balances, init_exchange_rates());
    let (pool_contract, market_contract) =
        setup_test(&mut app, Uint128::new(1000), Uint128::new(1000));
    let msg = MarketQueryMsg::GetConfig {};
    let result: MarketGetConfigResponse = app
        .wrap()
        .query_wasm_smart(market_contract.clone(), &msg)
        .unwrap();

    assert_eq!(pool_contract, result.pool_contract);

    let msg = QueryMsg::GetConfig {};
    let result: ConfigResponse = app
        .wrap()
        .query_wasm_smart(pool_contract.clone(), &msg)
        .unwrap();

    assert_eq!(result.market_contract, market_contract);

    let msg = QueryMsg::GetPool {};
    let result: PoolResponse = app
        .wrap()
        .query_wasm_smart(pool_contract.clone(), &msg)
        .unwrap();

    let PoolResponse {
        asset_amount,
        asset_denom,
        stable_amount,
        stable_denom,
        ..
    } = result;

    let asset = app
        .wrap()
        .query_balance(pool_contract.clone(), asset_denom)
        .unwrap();
    let stable = app
        .wrap()
        .query_balance(pool_contract, stable_denom)
        .unwrap();
    assert_eq!(asset_amount, asset.amount);
    assert_eq!(stable_amount, stable.amount);
}

pub struct TestDeposit {
    pub reserve_asset_amount: u128,
    pub reserve_stable_amount: u128,
    pub init_total_supply: u128,
    pub deposit_asset_amount: u128,
    pub deposit_stable_amount: u128,
    pub expect_lp_amount: u128,
    pub expect_asset_amount: u128,
    pub expect_stable_amount: u128,
}
#[test]
fn test_deposit() {
    let test_deposits = vec![
        //"ideal deposit"
        TestDeposit {
            reserve_asset_amount: 10_000,
            reserve_stable_amount: 10_000,
            init_total_supply: 10_000,
            deposit_asset_amount: 2_000,
            deposit_stable_amount: 2_000,
            expect_lp_amount: 2_000,
            expect_asset_amount: 98_000,
            expect_stable_amount: 98_000,
        },
        //Unequal Reserve Ratio, Equal Deposit Ratio
        TestDeposit {
            reserve_asset_amount: 10_000,
            reserve_stable_amount: 5_000,
            init_total_supply: 7_071, // Set directly
            deposit_asset_amount: 2_000,
            deposit_stable_amount: 2_000,
            expect_lp_amount: 1414,
            expect_asset_amount: 98_000,
            expect_stable_amount: 99_000,
        },
        //Equal Reserve Ratio, Unequal Deposit Ratio
        TestDeposit {
            reserve_asset_amount: 10_000,
            reserve_stable_amount: 10_000,
            init_total_supply: 10_000, // Set directly
            deposit_asset_amount: 2_000,
            deposit_stable_amount: 1_000,
            expect_lp_amount: 1_000,
            expect_asset_amount: 99_000,
            expect_stable_amount: 99_000,
        },
        //minum_minting
        TestDeposit {
            reserve_asset_amount: 10_000,
            reserve_stable_amount: 10_000,
            init_total_supply: 10_000,
            deposit_asset_amount: 1,
            deposit_stable_amount: 10,
            expect_lp_amount: 1,
            expect_asset_amount: 99_999,
            expect_stable_amount: 99_999,
        },
    ];
    for test in test_deposits.into_iter() {
        let mut app = mock_app(init_default_balances, init_exchange_rates());
        let (pool_contract, _) = setup_test(
            &mut app,
            Uint128::new(test.reserve_asset_amount),
            Uint128::new(test.reserve_stable_amount),
        );
        let msg = QueryMsg::GetPool {};
        let result: PoolResponse = app
            .wrap()
            .query_wasm_smart(pool_contract.clone(), &msg)
            .unwrap();
        assert_eq!(result.lp_total_supply, Uint128::new(test.init_total_supply));
        let msg = ExecuteMsg::Deposit {};
        app.execute_contract(
            Addr::unchecked(TRADER1),
            pool_contract.clone(),
            &msg,
            &vec![
                coin(test.deposit_asset_amount, ASSET_DENOM),
                coin(test.deposit_stable_amount, STABLE_DENOM),
            ],
        )
        .unwrap();

        let lp_denom = result.lp_denom;
        let result = app.wrap().query_balance(TRADER1, lp_denom.clone()).unwrap();
        assert_eq!(result, coin(test.expect_lp_amount, lp_denom));

        let trader1_asset_token = app.wrap().query_balance(TRADER1, ASSET_DENOM).unwrap();
        let trader1_stable_token = app.wrap().query_balance(TRADER1, STABLE_DENOM).unwrap();
        assert_eq!(
            trader1_asset_token.amount,
            Uint128::new(test.expect_asset_amount)
        );
        assert_eq!(
            trader1_stable_token.amount,
            Uint128::new(test.expect_stable_amount)
        );
    }
}

pub struct TestWithdraw<'a> {
    pub trader: &'a str,
    pub reserve_asset_amount: u128,
    pub reserve_stable_amount: u128,
    pub deposit_asset_amount: u128,
    pub deposit_stable_amount: u128,
    pub withdraw_lp_amount: u128,
    pub expect_asset_amount: u128,
    pub expect_stable_amount: u128,
}
#[test]
fn test_withdraw() {
    let test_vec = vec![
        TestWithdraw {
            //initial_balance = asset 100_000 / stable 100_000
            trader: TRADER1,
            reserve_asset_amount: 1_000_000,
            reserve_stable_amount: 1_000_000,
            deposit_asset_amount: 10_000,
            deposit_stable_amount: 10_000,
            withdraw_lp_amount: 5_000,
            expect_asset_amount: 94_999,
            expect_stable_amount: 94_999,
        },
        TestWithdraw {
            //initial_balance = asset 10_000_000 / stable 10_000_000
            trader: TRADER2,
            reserve_asset_amount: 1_000_000,
            reserve_stable_amount: 1_000_000,
            deposit_asset_amount: 100_000,
            deposit_stable_amount: 100_000,
            withdraw_lp_amount: 100_000,
            expect_asset_amount: 9_999_999,
            expect_stable_amount: 9_999_999,
        },
    ];
    for test in test_vec.into_iter() {
        let mut app = mock_app(init_default_balances, init_exchange_rates());
        let (pool_contract, _) = setup_test(
            &mut app,
            Uint128::new(test.reserve_asset_amount),
            Uint128::new(test.reserve_stable_amount),
        );
        let deposit_msg = ExecuteMsg::Deposit {};
        let _ = app
            .execute_contract(
                Addr::unchecked(test.trader),
                pool_contract.clone(),
                &deposit_msg,
                &vec![
                    coin(test.deposit_asset_amount, ASSET_DENOM),
                    coin(test.deposit_stable_amount, STABLE_DENOM),
                ],
            )
            .unwrap();

        let get_pool_msg = QueryMsg::GetPool {};
        let get_pool_result: PoolResponse = app
            .wrap()
            .query_wasm_smart(pool_contract.clone(), &get_pool_msg)
            .unwrap();

        let lp_denom = get_pool_result.lp_denom;
        //@@Withdraw Test
        let withdraw_msg = ExecuteMsg::Withdraw {};
        let _ = app.execute_contract(
            Addr::unchecked(test.trader),
            pool_contract,
            &withdraw_msg,
            &vec![coin(test.withdraw_lp_amount, lp_denom)],
        );
        let trader1_asset_token = app.wrap().query_balance(test.trader, ASSET_DENOM).unwrap();
        let trader1_stable_token = app.wrap().query_balance(test.trader, STABLE_DENOM).unwrap();
        assert_eq!(
            trader1_asset_token.amount,
            Uint128::new(test.expect_asset_amount)
        );
        assert_eq!(
            trader1_stable_token.amount,
            Uint128::new(test.expect_stable_amount)
        );
    }
}
