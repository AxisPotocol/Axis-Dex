use cosmwasm_std::{Addr, Uint128};
use cw_multi_test::Executor;
use sei_integration_tests::helper::mock_app;

use crate::{
    app::{
        init_default_balances, init_exchange_rates, setup_init, Contracts, ADMIN, BTC_DENOM,
        TRADER1, USDC_DENOM,
    },
    utils::setting,
};

use axis_protocol::{
    axis::{
        ConfigResponse, ExecuteMsg, PendingFeeResponse, PoolAllowedMintAmountResponse, QueryMsg,
    },
    lp_staking::{ExecuteMsg as LpStakingExeucteMsg, QueryMsg as LpStakingQueryMsg},
    market::{GetConfigResponse as MarketConfigResponse, QueryMsg as MarketQueryMsg},
    pool::{ConfigResponse as PoolConfigResponse, QueryMsg as PoolQueryMsg},
};

#[test]
fn test_add_fee_amount() {
    let mut app = mock_app(init_default_balances, init_exchange_rates());

    let contracts = setup_init(&mut app, BTC_DENOM, USDC_DENOM);
    let market_contract = contracts.market_contract;
    let axis_contract = contracts.axis_contract;
    let trader1 = Addr::unchecked(TRADER1);
    let market_config_res: MarketConfigResponse = app
        .wrap()
        .query_wasm_smart(market_contract.to_owned(), &MarketQueryMsg::GetConfig {})
        .unwrap();
    let base_denom = market_config_res.base_denom;
    let price_denom = market_config_res.price_denom;
    //@@Invalid Market Contract
    let add_fee_amount_result = app.execute_contract(
        contracts.core_contract.to_owned(),
        axis_contract.to_owned(),
        &ExecuteMsg::AddFeeAmount {
            base_denom: base_denom.to_owned(),
            price_denom: price_denom.to_owned(),
            trader: trader1.to_owned(),
            fee_usd_amount: Uint128::new(100),
        },
        &vec![],
    );
    assert!(add_fee_amount_result.is_err());

    //@@Valid Add Fee Amount Function
    let add_fee_amount_result = app.execute_contract(
        market_contract.to_owned(),
        axis_contract.to_owned(),
        &ExecuteMsg::AddFeeAmount {
            base_denom: base_denom.to_owned(),
            price_denom: price_denom.to_owned(),
            trader: trader1,
            fee_usd_amount: Uint128::new(100),
        },
        &vec![],
    );
    assert!(add_fee_amount_result.is_ok());

    let axis_pending_usd: PendingFeeResponse = app
        .wrap()
        .query_wasm_smart(axis_contract.to_owned(), &QueryMsg::GetPendingTotalFee {})
        .unwrap();
    assert_eq!(axis_pending_usd.pending_total_fee, Uint128::new(100));
}

#[test]
fn test_claim_minting_trader() {
    let mut app = mock_app(init_default_balances, init_exchange_rates());

    let contracts = setup_init(&mut app, BTC_DENOM, USDC_DENOM);
    let market_contract = contracts.market_contract;
    let axis_contract = contracts.axis_contract;
    let core_contract = contracts.core_contract;
    let admin = Addr::unchecked(ADMIN);
    let trader1 = Addr::unchecked(TRADER1);
    let market_config_res: MarketConfigResponse = app
        .wrap()
        .query_wasm_smart(market_contract.to_owned(), &MarketQueryMsg::GetConfig {})
        .unwrap();
    let base_denom = market_config_res.base_denom;
    let price_denom = market_config_res.price_denom;

    //@@Invalid Market Contract
    let add_fee_amount_result = app.execute_contract(
        market_contract.to_owned(),
        axis_contract.to_owned(),
        &ExecuteMsg::AddFeeAmount {
            base_denom: base_denom.to_owned(),
            price_denom: price_denom.to_owned(),
            trader: trader1.to_owned(),
            fee_usd_amount: Uint128::new(100),
        },
        &vec![],
    );

    assert!(add_fee_amount_result.is_ok());

    app.update_block(|block| block.time = block.time.plus_days(1));
    //epoch is 1
    let setting_result = setting(&mut app, &core_contract, &admin);

    assert!(setting_result.is_ok());

    //@@Valid
    let claim_mint_trader_result = app.execute_contract(
        trader1.to_owned(),
        axis_contract.to_owned(),
        &ExecuteMsg::ClaimMintTrader {},
        &vec![],
    );

    assert!(claim_mint_trader_result.is_ok());

    let axis_config_res: ConfigResponse = app
        .wrap()
        .query_wasm_smart(axis_contract.to_owned(), &QueryMsg::GetConfig {})
        .unwrap();

    let axis_denom = axis_config_res.axis_denom;

    let trader_axis = app
        .wrap()
        .query_balance(trader1.to_owned(), axis_denom.to_owned())
        .unwrap();

    assert_eq!(
        trader_axis.amount,
        axis_config_res.mint_per_epoch_trader_amount
    )
}

#[test]
pub fn test_claim_minting_maker() {
    let mut app = mock_app(init_default_balances, init_exchange_rates());
    let admin = Addr::unchecked(ADMIN);
    let trader1 = Addr::unchecked(TRADER1);

    let contracts = setup_init(&mut app, BTC_DENOM, USDC_DENOM);
    let Contracts {
        market_contract,
        axis_contract,
        lp_staking_contract,
        core_contract,
        pool_contract,
        ..
    } = contracts;

    let pool_config_res: PoolConfigResponse = app
        .wrap()
        .query_wasm_smart(pool_contract.to_owned(), &PoolQueryMsg::GetConfig {})
        .unwrap();
    let lp_denom = pool_config_res.lp_denom;

    let admin_lp = app
        .wrap()
        .query_balance(admin.to_owned(), lp_denom.to_owned())
        .unwrap();

    let lp_staking_result = app.execute_contract(
        admin.to_owned(),
        lp_staking_contract.to_owned(),
        &LpStakingExeucteMsg::Staking {},
        &vec![admin_lp.to_owned()],
    );
    assert!(lp_staking_result.is_ok());

    let market_config_res: MarketConfigResponse = app
        .wrap()
        .query_wasm_smart(market_contract.to_owned(), &MarketQueryMsg::GetConfig {})
        .unwrap();
    let base_denom = market_config_res.base_denom;
    let price_denom = market_config_res.price_denom;

    let add_fee_amount_result = app.execute_contract(
        market_contract.to_owned(),
        axis_contract.to_owned(),
        &ExecuteMsg::AddFeeAmount {
            base_denom: base_denom.to_owned(),
            price_denom: price_denom.to_owned(),
            trader: trader1.to_owned(),
            fee_usd_amount: Uint128::new(100),
        },
        &vec![],
    );
    assert!(add_fee_amount_result.is_ok());

    app.update_block(|block| block.time = block.time.plus_days(1));
    //epoch is 1
    let setting_result = setting(&mut app, &core_contract, &admin);
    assert!(setting_result.is_ok());
    app.update_block(|block| block.time = block.time.plus_days(1));

    let get_pool_allowed_mint_amount: Vec<PoolAllowedMintAmountResponse> = app
        .wrap()
        .query_wasm_smart(
            axis_contract.to_owned(),
            &QueryMsg::GetPoolAllowedMintAmount {
                base_denom: base_denom.to_owned(),
                price_denom: price_denom.to_owned(),
                start_epoch: 0,
            },
        )
        .unwrap();
    let mintable_amount: Uint128 = get_pool_allowed_mint_amount
        .into_iter()
        .map(|response| response.mint_amount) // Extract the `amount` from each response.
        .sum();

    //Invalid Claim
    let claim_minting_maker_result = app.execute_contract(
        core_contract.to_owned(),
        axis_contract.to_owned(),
        &ExecuteMsg::ClaimMintMaker {
            base_denom: base_denom.to_owned(),
            price_denom: price_denom.to_owned(),
            sender: admin.to_owned(),
            amount: mintable_amount,
        },
        &vec![],
    );
    assert!(claim_minting_maker_result.is_err());

    //@@Valid claim
    let claim_minting_maker_result = app.execute_contract(
        lp_staking_contract.to_owned(),
        axis_contract.to_owned(),
        &ExecuteMsg::ClaimMintMaker {
            base_denom: base_denom.to_owned(),
            price_denom: price_denom.to_owned(),
            sender: admin.to_owned(),
            amount: mintable_amount,
        },
        &vec![],
    );

    assert!(claim_minting_maker_result.is_ok());

    let axis_config_res: ConfigResponse = app
        .wrap()
        .query_wasm_smart(axis_contract.to_owned(), &QueryMsg::GetConfig {})
        .unwrap();

    let axis_denom = axis_config_res.axis_denom;

    let admin_axis = app
        .wrap()
        .query_balance(admin.to_owned(), axis_denom.to_owned())
        .unwrap();
    //admin has axis 200_000_000_000_000 + 200164383571643

    assert_eq!(
        admin_axis.amount,
        Uint128::new(200_000_000_000_000) + mintable_amount
    )
}
