use std::str::FromStr;

use axis_protocol::market::{ExecuteMsg, QueryMsg, TradeResponse};
use axis_protocol::pool::{PoolResponse, PositionBalance, QueryMsg as PoolQueryMsg};
use cosmwasm_std::{coin, Addr, Coin, Decimal, Uint128, Uint64};
use cw_multi_test::Executor;
use sei_cosmwasm::{DenomOracleExchangeRatePair, OracleExchangeRate};
use sei_integration_tests::helper::mock_app;

use crate::testing::app::{init_default_balances, setup_test};

pub const ADMIN: &str = "admin";
pub const ASSET_DENOM: &str = "ubtc";
pub const STABLE_DENOM: &str = "uusdc";
pub const TRADER1: &str = "trader1";
pub const TRADER2: &str = "trader2";
pub const TRADER3: &str = "trader3";
pub const TRADER4: &str = "trader4";
pub const TRADER5: &str = "trader5";
#[test]
pub fn test_proper_open_trade() {
    let mut app = mock_app(
        init_default_balances,
        vec![
            DenomOracleExchangeRatePair {
                denom: STABLE_DENOM.to_string(),
                oracle_exchange_rate: OracleExchangeRate {
                    exchange_rate: Decimal::from_str("1").unwrap(),
                    last_update: Uint64::zero(),
                },
            },
            DenomOracleExchangeRatePair {
                denom: ASSET_DENOM.to_string(),
                oracle_exchange_rate: OracleExchangeRate {
                    exchange_rate: Decimal::from_str("10").unwrap(),
                    last_update: Uint64::zero(),
                },
            },
        ],
    );
    let (pool_contract, market_contract) = setup_test(&mut app);

    let msg = ExecuteMsg::Open {
        position: true,
        leverage: 10,
        limit_profit_price: None,
        limit_loss_price: None,
    };
    let result = app.execute_contract(
        Addr::unchecked(TRADER1),
        market_contract.clone(),
        &msg,
        &vec![coin(90_000_000, ASSET_DENOM)],
    );

    assert!(result.is_ok());
    let result: TradeResponse = app
        .wrap()
        .query_wasm_smart(
            market_contract,
            &QueryMsg::GetTrade {
                trader: TRADER1.to_string(),
            },
        )
        .unwrap();

    assert_eq!(
        result,
        TradeResponse {
            trader: Addr::unchecked(TRADER1),
            entry_price: Uint128::new(10000000000000000000),
            liquidation_price: Uint128::new(9098989898989898990),
            limit_profit_price: Uint128::MAX,
            limit_loss_price: Uint128::MAX,
            collateral_denom: "ubtc".to_string(),
            collateral_amount: Uint128::new(89100000), //90 - (90*10 * 0.001)
            position: true,
            position_size: Uint128::new(891000000),
            leverage: 10,
            leverage_amount: Uint128::new(891000000)
        }
    );

    let msg = PoolQueryMsg::GetPositionBalance { position: true };
    let result: PositionBalance = app
        .wrap()
        .query_wasm_smart(pool_contract.clone(), &msg)
        .unwrap();

    assert_eq!(
        result.amount,
        Uint128::new(1_000_000_000_000) - Uint128::new(891_000_000)
    );
    let pool_balance: Coin = app
        .wrap()
        .query_balance(pool_contract.clone(), ASSET_DENOM)
        .unwrap();

    let pool_query_msg = PoolQueryMsg::GetPool {};
    let pool_response: PoolResponse = app
        .wrap()
        .query_wasm_smart(pool_contract, &pool_query_msg)
        .unwrap();

    assert_eq!(pool_balance.amount, pool_response.asset_amount);
    assert_eq!(
        pool_balance,
        coin(
            (Uint128::new(1_000_000_000_000) - Uint128::new(891_000_000)).u128(),
            ASSET_DENOM
        ),
    );
}

#[test]
fn test_close() {
    let mut app = mock_app(
        init_default_balances,
        vec![
            DenomOracleExchangeRatePair {
                denom: STABLE_DENOM.to_string(),
                oracle_exchange_rate: OracleExchangeRate {
                    exchange_rate: Decimal::from_str("1").unwrap(),
                    last_update: Uint64::zero(),
                },
            },
            DenomOracleExchangeRatePair {
                denom: ASSET_DENOM.to_string(),
                oracle_exchange_rate: OracleExchangeRate {
                    exchange_rate: Decimal::from_str("10").unwrap(),
                    last_update: Uint64::zero(),
                },
            },
        ],
    );
    let (pool_contract, market_contract) = setup_test(&mut app);

    let msg = ExecuteMsg::Open {
        position: true,
        leverage: 10,
        limit_profit_price: None,
        limit_loss_price: None,
    };
    let result = app.execute_contract(
        Addr::unchecked(TRADER1),
        market_contract.clone(),
        &msg,
        &vec![coin(10_000_000, ASSET_DENOM)],
    );
    assert!(result.is_ok());

    //close
    let msg = ExecuteMsg::Close {};
    let result = app.execute_contract(
        Addr::unchecked(TRADER1),
        market_contract.clone(),
        &msg,
        &Vec::new(),
    );
    assert!(result.is_ok());
    let trader1_balance = app.wrap().query_balance(TRADER1, ASSET_DENOM).unwrap();

    //@@trader1_balance = 100_000_000;
    //@@collateral balance = 10_000_000
    //@@leverage = 10
    //@@open fee = inital_balance * leverage *0.001  = 100_000
    //@@collateral = 9 900 000
    //@@close fee = collateral + profit or - loss * 0.001  = 9 900
    //@@user balance = 9_900_000 - 9_900
    assert_eq!(trader1_balance, coin(99_890_100, ASSET_DENOM));

    let pool_balance = app
        .wrap()
        .query_balance(pool_contract, ASSET_DENOM)
        .unwrap();
    //@@ FEE distribution is distributed from hook_liquidated.
    assert_eq!(pool_balance, coin(1_000_000_000_000, ASSET_DENOM));

    //@@@ Fail test
    //Trader2 Close
    let msg = ExecuteMsg::Close {};
    let result = app.execute_contract(
        Addr::unchecked(TRADER2),
        market_contract.clone(),
        &msg,
        &vec![],
    );
    assert!(result.is_err());
}

#[test]
fn test_hook_liquidated() {}
