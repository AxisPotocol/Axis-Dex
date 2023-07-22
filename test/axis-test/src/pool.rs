use cosmwasm_std::{coin, Addr, Uint128};

use pool::state::Pool;
use sei_cosmwasm::{SeiMsg, SeiQueryWrapper};
use sei_integration_tests::{helper::mock_app, module::SeiModule};

use crate::{
    app::{
        create_pair, init_default_balances, init_exchange_rates, setup_init, ADMIN, BTC_DENOM,
        ETH_DENOM, TRADER1, TRADER2, USDC_DENOM,
    },
    utils::{deposit, position_close, position_open, withdraw},
};
use axis_protocol::{
    core::{PairPoolContractResponse, QueryMsg as CoreQueryMsg},
    market::ExecuteMsg as MarketExecuteMsg,
    pool::{ConfigResponse, ExecuteMsg, PoolResponse, QueryMsg},
};
use cosmwasm_std::{
    testing::{MockApi, MockStorage},
    Api, Decimal, Empty, GovMsg, IbcMsg, IbcQuery, Response, Storage, SubMsgResult, Uint64,
};
use cw_multi_test::{
    App, AppResponse, BankKeeper, ContractWrapper, DistributionKeeper, Executor, FailingModule,
    Router, StakeKeeper, WasmKeeper,
};

pub struct TestDeposit {
    pub reserve_base_amount: u128,
    pub reserve_price_amount: u128,
    pub init_total_supply: u128,
    pub deposit_base_amount: u128,
    pub deposit_price_amount: u128,
    pub expect_lp_amount: u128,
    pub expect_base_amount: u128,
    pub expect_price_amount: u128,
}
#[test]
fn test_deposit() {
    let test_deposits = vec![
        //Balanced Deposit
        TestDeposit {
            reserve_base_amount: 10_000,
            reserve_price_amount: 10_000,
            init_total_supply: 10_000,
            deposit_base_amount: 2_000,
            deposit_price_amount: 2_000,
            expect_lp_amount: 2_000,
            expect_base_amount: 98_000,
            expect_price_amount: 98_000,
        },
        //Unequal Reserve Ratio, Equal Deposit Ratio
        TestDeposit {
            reserve_base_amount: 10_000,
            reserve_price_amount: 5_000,
            init_total_supply: 7_071,
            deposit_base_amount: 2_000,
            deposit_price_amount: 2_000,
            expect_lp_amount: 1414,
            expect_base_amount: 98_000,
            expect_price_amount: 99_000,
        },
        //Equal Reserve Ratio, Unequal Deposit Ratio
        TestDeposit {
            reserve_base_amount: 10_000,
            reserve_price_amount: 10_000,
            init_total_supply: 10_000,
            deposit_base_amount: 2_000,
            deposit_price_amount: 1_000,
            expect_lp_amount: 1_000,
            expect_base_amount: 99_000,
            expect_price_amount: 99_000,
        },
        //minum_minting
        TestDeposit {
            reserve_base_amount: 10_000,
            reserve_price_amount: 10_000,
            init_total_supply: 10_000,
            deposit_base_amount: 1,
            deposit_price_amount: 10,
            expect_lp_amount: 1,
            expect_base_amount: 99_999,
            expect_price_amount: 99_999,
        },
    ];
    for test in test_deposits.into_iter() {
        let mut app = mock_app(init_default_balances, init_exchange_rates());

        let contracts = setup_init(&mut app, BTC_DENOM, USDC_DENOM);
        
        let trader1 = Addr::unchecked(TRADER1);
        let base_denom = ETH_DENOM;
        let price_denom = USDC_DENOM;
        let pool_contract = create_pair(
            &mut app,
            &Addr::unchecked(ADMIN),
            &contracts,
            base_denom,
            price_denom,
            test.reserve_base_amount,
            test.reserve_price_amount,
        )
        .unwrap();

        let config_res: ConfigResponse = app
            .wrap()
            .query_wasm_smart(pool_contract.to_owned(), &QueryMsg::GetConfig {})
            .unwrap();

        let lp_denom = config_res.lp_denom;
        let admin_lp = app
            .wrap()
            .query_balance(ADMIN, lp_denom.to_owned())
            .unwrap();

        assert_eq!(admin_lp.amount, Uint128::new(test.init_total_supply));
        let _ = deposit(
            &mut app,
            &pool_contract,
            &trader1,
            base_denom,
            price_denom,
            test.deposit_base_amount,
            test.deposit_price_amount,
        );

        let trader_lp = app.wrap().query_balance(TRADER1, lp_denom).unwrap();
        assert_eq!(trader_lp.amount, Uint128::new(test.expect_lp_amount));
    }
}

pub struct TestWithdraw<'a> {
    pub trader: &'a str,
    pub reserve_base_amount: u128,
    pub reserve_price_amount: u128,
    pub deposit_base_amount: u128,
    pub deposit_price_amount: u128,
    pub withdraw_lp_amount: u128,
    pub expect_reserve_base_amount: u128,
    pub expect_reserve_price_amount: u128,
    pub expect_trader_base_amount: u128,
    pub expect_trader_price_amount: u128,
}

#[test]
fn test_withdraw() {
    let test_withdraws = vec![
        TestWithdraw {
            //initial_balance = base 100_000 / price 100_000
            trader: TRADER1,
            reserve_base_amount: 1_000_000,
            reserve_price_amount: 1_000_000,
            deposit_base_amount: 10_000,
            deposit_price_amount: 10_000,
            withdraw_lp_amount: 5_000,
            expect_reserve_base_amount: 1_005_005,
            expect_reserve_price_amount: 1_005_005,
            expect_trader_base_amount: 9_994_995,
            expect_trader_price_amount: 9_994_995,
        },
        TestWithdraw {
            //initial_balance = base 10_000_000 / price 10_000_000
            trader: TRADER2,
            reserve_base_amount: 1_000_000,
            reserve_price_amount: 1_000_000,
            deposit_base_amount: 100_000,
            deposit_price_amount: 100_000,
            withdraw_lp_amount: 100_000,
            expect_reserve_base_amount: 1_000_100,
            expect_reserve_price_amount: 1_000_100,
            expect_trader_base_amount: 9_999_900,
            expect_trader_price_amount: 9_999_900,
        },
    ];

    for test in test_withdraws.into_iter() {
        let mut app = mock_app(init_default_balances, init_exchange_rates());

        let contracts = setup_init(&mut app, BTC_DENOM, USDC_DENOM);

        let trader1 = Addr::unchecked(TRADER1);
        let base_denom = ETH_DENOM;
        let price_denom = USDC_DENOM;
        let pool_contract = create_pair(
            &mut app,
            &Addr::unchecked(ADMIN),
            &contracts,
            base_denom,
            price_denom,
            test.reserve_base_amount,
            test.reserve_price_amount,
        )
        .unwrap();

        let config_res: ConfigResponse = app
            .wrap()
            .query_wasm_smart(pool_contract.to_owned(), &QueryMsg::GetConfig {})
            .unwrap();

        let lp_denom = config_res.lp_denom;
        let _ = deposit(
            &mut app,
            &pool_contract,
            &trader1,
            base_denom,
            price_denom,
            test.deposit_base_amount,
            test.deposit_price_amount,
        );

        let _ = withdraw(
            &mut app,
            &pool_contract,
            &trader1,
            test.withdraw_lp_amount,
            lp_denom.as_str(),
        );
        let pool: PoolResponse = app
            .wrap()
            .query_wasm_smart(pool_contract, &QueryMsg::GetPool {})
            .unwrap();

        assert_eq!(test.expect_reserve_base_amount, pool.base_amount.into());
        assert_eq!(test.expect_reserve_price_amount, pool.price_amount.into());

        assert_eq!(
            test.expect_trader_base_amount,
            app.wrap()
                .query_balance(trader1.to_owned(), base_denom)
                .unwrap()
                .amount
                .into()
        );

        assert_eq!(
            test.expect_trader_price_amount,
            app.wrap()
                .query_balance(trader1, price_denom)
                .unwrap()
                .amount
                .into()
        )
    }
}

#[test]
pub fn test_borrow() {
    let mut app = mock_app(init_default_balances, init_exchange_rates());

    let contracts = setup_init(&mut app, BTC_DENOM, USDC_DENOM);

    let base_denom = ETH_DENOM;
    let price_denom = USDC_DENOM;
    let pool_contract = create_pair(
        &mut app,
        &Addr::unchecked(ADMIN),
        &contracts,
        base_denom,
        price_denom,
        1_000_000_000,
        1_000_000_000,
    )
    .unwrap();

    let config_res: ConfigResponse = app
        .wrap()
        .query_wasm_smart(pool_contract.to_owned(), &QueryMsg::GetConfig {})
        .unwrap();
    let trader1 = Addr::unchecked(TRADER1);
    let market_contract = config_res.market_contract;
    let _ = position_open(
        &mut app,
        &market_contract,
        &trader1,
        true,
        10,
        10_000_000,
        base_denom,
    );

    let pool: PoolResponse = app
        .wrap()
        .query_wasm_smart(pool_contract, &QueryMsg::GetPool {})
        .unwrap();
    //@@ leverage_amount = 99_000_000;
    //@@ collateral = 10_000_000;
    //@@ fee = collateral * leverage * 0.001(0.1%) = 100_000
    //@@ collateral = 10_000_000 - 100_000 = 9_900_000
    //@@ position size = 9_900_000 * leverage(10) = 99_000_000
    //@@ pool_borrow = position_size
    //@@ 1_000_000_000 - 99_000_000
    assert_eq!(901_000_000, pool.base_amount.u128());
}

#[test]
pub fn test_repay() {
    let mut app = mock_app(init_default_balances, init_exchange_rates());

    let contracts = setup_init(&mut app, BTC_DENOM, USDC_DENOM);

    let base_denom = ETH_DENOM;
    let price_denom = USDC_DENOM;
    let pool_contract = create_pair(
        &mut app,
        &Addr::unchecked(ADMIN),
        &contracts,
        base_denom,
        price_denom,
        1_000_000_000,
        1_000_000_000,
    )
    .unwrap();

    let config_res: ConfigResponse = app
        .wrap()
        .query_wasm_smart(pool_contract.to_owned(), &QueryMsg::GetConfig {})
        .unwrap();
    let trader1 = Addr::unchecked(TRADER1);
    let market_contract = config_res.market_contract;
    let _ = position_open(
        &mut app,
        &market_contract,
        &trader1,
        true,
        10,
        10_000_000,
        base_denom,
    );

    let _ = position_close(&mut app, &market_contract, &trader1);

    let pool: PoolResponse = app
        .wrap()
        .query_wasm_smart(pool_contract.to_owned(), &QueryMsg::GetPool {})
        .unwrap();

    assert_eq!(1_000_000_000, pool.base_amount.u128());
    let pool_base_amount = app
        .wrap()
        .query_balance(pool_contract, base_denom)
        .unwrap()
        .amount;
    assert_eq!(pool_base_amount, pool.base_amount);
}
