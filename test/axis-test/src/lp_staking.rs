use cosmwasm_std::{Addr, Coin, Decimal, Uint128};
use cw_multi_test::Executor;
use sei_integration_tests::helper::mock_app;

use crate::app::{
    init_default_balances, init_exchange_rates, setup_init, Contracts, ADMIN, BTC_DENOM, TRADER1,
    USDC_DENOM,
};

use axis_protocol::{
    lp_staking::{ConfigResponse, ExecuteMsg, QueryMsg, StakeInfoResponse, StateResponse},
    pool::{
        ConfigResponse as PoolConfigResponse, ExecuteMsg as PoolExecuteMsg,
        QueryMsg as PoolQueryMsg,
    },
};
#[test]
fn valid_staking_same_epoch() {
    let mut app = mock_app(init_default_balances, init_exchange_rates());

    let contracts = setup_init(&mut app, BTC_DENOM, USDC_DENOM);
    let Contracts {
        lp_staking_contract,
        pool_contract,
        ..
    } = contracts;
    let admin = Addr::unchecked(ADMIN);
    let pool_config_res: PoolConfigResponse = app
        .wrap()
        .query_wasm_smart(pool_contract.to_owned(), &PoolQueryMsg::GetConfig {})
        .unwrap();
    let lp_denom = pool_config_res.lp_denom;

    assert_eq!(pool_config_res.lp_staking_contract, lp_staking_contract);
    let admin_lp_coin = app
        .wrap()
        .query_balance(admin.to_owned(), lp_denom.to_owned())
        .unwrap();
    //@@Staking twice in the same epoch

    let admin_lp_one = Coin {
        denom: lp_denom.to_owned(),
        amount: admin_lp_coin.amount * Decimal::percent(50),
    };
    let admin_lp_two = Coin {
        denom: lp_denom.to_owned(),
        amount: admin_lp_coin.amount * Decimal::percent(50),
    };

    let _ = app.execute_contract(
        admin.to_owned(),
        lp_staking_contract.to_owned(),
        &ExecuteMsg::Staking {},
        &vec![admin_lp_one.to_owned()],
    );

    let _ = app
        .execute_contract(
            admin.to_owned(),
            lp_staking_contract.to_owned(),
            &ExecuteMsg::Staking {},
            &vec![admin_lp_two.to_owned()],
        )
        .unwrap();

    let stake_info_res: Vec<StakeInfoResponse> = app
        .wrap()
        .query_wasm_smart(
            lp_staking_contract.to_owned(),
            &QueryMsg::GetStakeInfo {
                address: admin.to_owned(),
            },
        )
        .unwrap();
    //same epoch is one Stake Info
    assert_eq!(
        stake_info_res,
        vec![StakeInfoResponse {
            start_epoch: 1,
            staking_amount: admin_lp_coin.amount
        }]
    );

    let lp_staking_state_res: StateResponse = app
        .wrap()
        .query_wasm_smart(lp_staking_contract.to_owned(), &QueryMsg::GetState {})
        .unwrap();

    assert_eq!(
        lp_staking_state_res.stake_pending_total,
        admin_lp_coin.amount
    );
    assert_eq!(lp_staking_state_res.staking_total, Uint128::zero());
    assert_eq!(lp_staking_state_res.withdraw_pending_total, Uint128::zero());
}

#[test]
fn valid_staking_other_epoch() {
    let mut app = mock_app(init_default_balances, init_exchange_rates());

    let contracts = setup_init(&mut app, BTC_DENOM, USDC_DENOM);
    let Contracts {
        lp_staking_contract,
        pool_contract,
        core_contract,
        ..
    } = contracts;

    let pool_config_res: PoolConfigResponse = app
        .wrap()
        .query_wasm_smart(pool_contract.to_owned(), &PoolQueryMsg::GetConfig {})
        .unwrap();
    let lp_denom = pool_config_res.lp_denom;

    assert_eq!(pool_config_res.lp_staking_contract, lp_staking_contract);
    //trader deferent epoch deposit
    let trader = Addr::unchecked(TRADER1);

    let trader_btc = app
        .wrap()
        .query_balance(trader.to_owned(), BTC_DENOM)
        .unwrap();

    let trader_usdc = app
        .wrap()
        .query_balance(trader.to_owned(), USDC_DENOM)
        .unwrap();

    let _ = app
        .execute_contract(
            trader.to_owned(),
            pool_contract.to_owned(),
            &PoolExecuteMsg::Deposit {},
            &vec![trader_btc, trader_usdc],
        )
        .unwrap();

    let trader_lp_coin = app
        .wrap()
        .query_balance(trader.to_owned(), lp_denom.to_owned())
        .unwrap();

    let trader_lp_one = Coin {
        denom: lp_denom.to_owned(),
        amount: trader_lp_coin.amount * Decimal::percent(50),
    };
    // println!("@@@@@@@trader_lp_one = {:?}", trader_lp_one);
    let trader_lp_two = Coin {
        denom: lp_denom.to_owned(),
        amount: trader_lp_coin.amount * Decimal::percent(50),
    };

    let _ = app
        .execute_contract(
            trader.to_owned(),
            lp_staking_contract.to_owned(),
            &ExecuteMsg::Staking {},
            &vec![trader_lp_one.to_owned()],
        )
        .unwrap();

    //@@Setting
    let _ = app
        .execute_contract(
            core_contract.to_owned(),
            lp_staking_contract.to_owned(),
            &ExecuteMsg::Setting { epoch: 1 },
            &vec![],
        )
        .unwrap();

    let _ = app
        .execute_contract(
            trader.to_owned(),
            lp_staking_contract.to_owned(),
            &ExecuteMsg::Staking {},
            &vec![trader_lp_two.to_owned()],
        )
        .unwrap();

    let stake_info_res: Vec<StakeInfoResponse> = app
        .wrap()
        .query_wasm_smart(
            lp_staking_contract.to_owned(),
            &QueryMsg::GetStakeInfo {
                address: trader.to_owned(),
            },
        )
        .unwrap();

    assert_eq!(
        stake_info_res,
        vec![
            StakeInfoResponse {
                start_epoch: 1,
                staking_amount: trader_lp_one.amount
            },
            StakeInfoResponse {
                start_epoch: 2,
                staking_amount: trader_lp_two.amount
            }
        ]
    );

    let lp_staking_state_res: StateResponse = app
        .wrap()
        .query_wasm_smart(lp_staking_contract.to_owned(), &QueryMsg::GetState {})
        .unwrap();

    assert_eq!(lp_staking_state_res.epoch, 1);
    assert_eq!(lp_staking_state_res.staking_total, trader_lp_one.amount);
    assert_eq!(
        lp_staking_state_res.stake_pending_total,
        trader_lp_two.amount
    );
    assert_eq!(lp_staking_state_res.withdraw_pending_total, Uint128::zero());
}

#[test]
fn fail_staking() {
    let mut app = mock_app(init_default_balances, init_exchange_rates());

    let contracts = setup_init(&mut app, BTC_DENOM, USDC_DENOM);
    let Contracts {
        lp_staking_contract,
        pool_contract,
        ..
    } = contracts;

    //trader deferent epoch deposit
    let trader = Addr::unchecked(TRADER1);

    let trader_btc = app
        .wrap()
        .query_balance(trader.to_owned(), BTC_DENOM)
        .unwrap();

    let trader_usdc = app
        .wrap()
        .query_balance(trader.to_owned(), USDC_DENOM)
        .unwrap();

    let _ = app
        .execute_contract(
            trader.to_owned(),
            pool_contract.to_owned(),
            &PoolExecuteMsg::Deposit {},
            &vec![trader_btc, trader_usdc],
        )
        .unwrap();

    let trader_btc = app
        .wrap()
        .query_balance(trader.to_owned(), BTC_DENOM.to_owned())
        .unwrap();

    let staking_result = app.execute_contract(
        trader.to_owned(),
        lp_staking_contract.to_owned(),
        &ExecuteMsg::Staking {},
        &vec![trader_btc.to_owned()],
    );

    assert!(staking_result.is_err());
}

#[test]
fn valid_unstaking_test() {
    let mut app = mock_app(init_default_balances, init_exchange_rates());

    let contracts = setup_init(&mut app, BTC_DENOM, USDC_DENOM);
    let Contracts {
        lp_staking_contract,
        pool_contract,
        core_contract,
        ..
    } = contracts;

    let pool_config_res: PoolConfigResponse = app
        .wrap()
        .query_wasm_smart(pool_contract.to_owned(), &PoolQueryMsg::GetConfig {})
        .unwrap();
    let lp_denom = pool_config_res.lp_denom;

    assert_eq!(pool_config_res.lp_staking_contract, lp_staking_contract);
    //trader deferent epoch deposit
    let trader = Addr::unchecked(TRADER1);

    let trader_btc = app
        .wrap()
        .query_balance(trader.to_owned(), BTC_DENOM)
        .unwrap();

    let trader_usdc = app
        .wrap()
        .query_balance(trader.to_owned(), USDC_DENOM)
        .unwrap();

    let _ = app
        .execute_contract(
            trader.to_owned(),
            pool_contract.to_owned(),
            &PoolExecuteMsg::Deposit {},
            &vec![trader_btc, trader_usdc],
        )
        .unwrap();

    let trader_lp_coin = app
        .wrap()
        .query_balance(trader.to_owned(), lp_denom.to_owned())
        .unwrap();

    let trader_lp_one = Coin {
        denom: lp_denom.to_owned(),
        amount: trader_lp_coin.amount * Decimal::percent(50),
    };
    // println!("@@@@@@@trader_lp_one = {:?}", trader_lp_one);
    let trader_lp_two = Coin {
        denom: lp_denom.to_owned(),
        amount: trader_lp_coin.amount * Decimal::percent(50),
    };

    let _ = app
        .execute_contract(
            trader.to_owned(),
            lp_staking_contract.to_owned(),
            &ExecuteMsg::Staking {},
            &vec![trader_lp_one.to_owned()],
        )
        .unwrap();

    //@@Setting
    let _ = app
        .execute_contract(
            core_contract.to_owned(),
            lp_staking_contract.to_owned(),
            &ExecuteMsg::Setting { epoch: 1 },
            &vec![],
        )
        .unwrap();

    let _ = app
        .execute_contract(
            trader.to_owned(),
            lp_staking_contract.to_owned(),
            &ExecuteMsg::Staking {},
            &vec![trader_lp_two.to_owned()],
        )
        .unwrap();

    let unstaking_result = app.execute_contract(
        trader.to_owned(),
        lp_staking_contract.to_owned(),
        &ExecuteMsg::UnStaking {},
        &vec![],
    );

    assert!(unstaking_result.is_ok());
}
