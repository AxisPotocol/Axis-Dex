use cosmwasm_std::{Addr, BlockInfo, Decimal, QueryRequest, Uint128};
use sei_cosmwasm::{ExchangeRatesResponse, SeiQuery, SeiQueryWrapper, SeiRoute, SudoMsg};
use sei_integration_tests::helper::mock_app;

use crate::{
    app::{
        init_default_balances, init_exchange_rates, setup_init, ADMIN, BTC_DENOM, ETH_DENOM,
        TRADER1, USDC_DENOM,
    },
    utils::{position_close, position_open},
};

use axis_protocol::{
    market::{GetConfigResponse, QueryMsg as MarketQueryMsg, TradeResponse},
    pool::{PoolResponse, QueryMsg as PoolQueryMsg},
};

#[test]
pub fn valid_position_open() {
    let mut app = mock_app(init_default_balances, init_exchange_rates());

    let contracts = setup_init(&mut app, BTC_DENOM, USDC_DENOM);
    let market_contract = contracts.market_contract;
    let base_denom = BTC_DENOM;
    let trader = Addr::unchecked(TRADER1);
    let position_amount = 10_000_000;
    let result = position_open(
        &mut app,
        &market_contract,
        &trader,
        true,
        10,
        position_amount,
        base_denom,
    );

    assert!(result.is_ok());
    let trade: TradeResponse = app
        .wrap()
        .query_wasm_smart(
            market_contract,
            &MarketQueryMsg::GetTrade {
                trader: trader.to_string(),
            },
        )
        .unwrap();
    println!("{:?}", trade);
    let res: ExchangeRatesResponse = app
        .wrap()
        .query(&QueryRequest::Custom(SeiQueryWrapper {
            route: SeiRoute::Oracle,
            query_data: SeiQuery::ExchangeRates {},
        }))
        .unwrap();
    let btc_amount = res
        .denom_oracle_exchange_rate_pairs
        .into_iter()
        .find(|c| c.denom == base_denom)
        .unwrap()
        .oracle_exchange_rate
        .exchange_rate;
    println!("{:?}", btc_amount);
    assert_eq!(trade.collateral_amount.u128(), 9_900_000);
    assert_eq!(trade.collateral_denom, base_denom);
    assert_eq!(trade.entry_price, btc_amount.atomics());
    assert_eq!(trade.position, true);
    assert_eq!(trade.limit_loss_price, Uint128::MAX);
    assert_eq!(trade.limit_profit_price, Uint128::MAX);
    //@@position size
    //fee = 0.1% = (10_000_000 * leverage) * 0.001) = 100_000
    //collateral =10_000_000 - fee
    //position_size = collateral * leverage
    assert_eq!(trade.position_size.u128(), 99_000_000);

    //@@liquidation_price
    // Open Price * (Collateral usd * 0.9 +fee usd) / Collateral usd / Leverage.
    //10000 * (999_000 * 0.9 + 1000) / 999_000 / 10
    //9_099_998_989_898_989_898_990
    assert_eq!(trade.liquidation_price.u128(), 9089898989898989898990)
}

#[test]
pub fn invalid_position_open() {
    let mut app = mock_app(init_default_balances, init_exchange_rates());

    let contracts = setup_init(&mut app, BTC_DENOM, USDC_DENOM);
    let market_contract = contracts.market_contract;
    let base_denom = BTC_DENOM;
    let trader = Addr::unchecked(TRADER1);
    //@@low funds
    let result = position_open(
        &mut app,
        &market_contract,
        &trader,
        true,
        10,
        10_00,
        base_denom,
    );
    assert!(result.is_err());

    //@@invalid denom test
    let result = position_open(
        &mut app,
        &market_contract,
        &trader,
        true,
        10,
        10_000_000,
        ETH_DENOM,
    );
    assert!(result.is_err());

    let pool_res: PoolResponse = app
        .wrap()
        .query_wasm_smart(contracts.pool_contract, &PoolQueryMsg::GetPool {})
        .unwrap();
    let pool_base_amount = pool_res.base_amount;
    // println!("pool_base_amount = {:?}", pool_base_amount);

    //@@overflow position size test
    let result = position_open(
        &mut app,
        &market_contract,
        &Addr::unchecked(ADMIN),
        true,
        10,
        (pool_base_amount * Decimal::percent(11)).into(),
        base_denom,
    );

    assert!(result.is_err());
    let market_config: GetConfigResponse = app
        .wrap()
        .query_wasm_smart(market_contract.to_owned(), &MarketQueryMsg::GetConfig {})
        .unwrap();

    //@@Inavlid leverage_rate test
    let result = position_open(
        &mut app,
        &market_contract,
        &trader,
        true,
        market_config.max_leverage + 1,
        10_000_000,
        base_denom,
    );
    assert!(result.is_err());
}

#[test]
pub fn valid_position_close() {
    let mut app = mock_app(init_default_balances, init_exchange_rates());

    let contracts = setup_init(&mut app, BTC_DENOM, USDC_DENOM);
    let market_contract = contracts.market_contract;
    let base_denom = BTC_DENOM;
    let trader = Addr::unchecked(TRADER1);
    let result = position_open(
        &mut app,
        &market_contract,
        &trader,
        true,
        10,
        10_000_000,
        base_denom,
    );

    assert!(result.is_ok());
    let result = position_close(&mut app, &market_contract, &trader);
    assert!(result.is_ok())
}
