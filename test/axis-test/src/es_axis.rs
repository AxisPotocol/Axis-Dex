use cosmwasm_std::{Addr, Uint128};
use cw_multi_test::Executor;
use sei_integration_tests::helper::mock_app;

use crate::app::{
    init_default_balances, init_exchange_rates, setup_init, ADMIN, BTC_DENOM, USDC_DENOM,
};
use axis_protocol::es_axis::{ConfigResponse, ExecuteMsg, QueryMsg};
#[test]
pub fn test_mint() {
    let mut app = mock_app(init_default_balances, init_exchange_rates());

    let contracts = setup_init(&mut app, BTC_DENOM, USDC_DENOM);
    let staking_contract = contracts.staking_contract;
    let es_axis_contract = contracts.es_axis_contract;
    //@@ Invalid test
    let mint_result = app.execute_contract(
        contracts.core_contract.to_owned(),
        es_axis_contract.to_owned(),
        &ExecuteMsg::Mint {
            amount: Uint128::new(1000),
        },
        &vec![],
    );
    assert!(mint_result.is_err());

    //@@ valid test
    let mint_result = app.execute_contract(
        staking_contract.to_owned(),
        es_axis_contract.to_owned(),
        &ExecuteMsg::Mint {
            amount: Uint128::new(1000),
        },
        &vec![],
    );
    assert!(mint_result.is_ok());

    let es_axis_config_res: ConfigResponse = app
        .wrap()
        .query_wasm_smart(es_axis_contract.to_owned(), &QueryMsg::GetConfig {})
        .unwrap();
    assert_eq!(es_axis_config_res.es_axis_total_supply, Uint128::new(1000));

    let es_axis_denom = es_axis_config_res.es_axis_denom;

    let es_axis_balance = app
        .wrap()
        .query_balance(es_axis_contract.to_owned(), es_axis_denom.to_owned())
        .unwrap();

    assert_eq!(es_axis_balance.amount, Uint128::new(1000));
}

#[test]
fn test_claim() {
    let mut app = mock_app(init_default_balances, init_exchange_rates());

    let contracts = setup_init(&mut app, BTC_DENOM, USDC_DENOM);
    let staking_contract = contracts.staking_contract;
    let es_axis_contract = contracts.es_axis_contract;
    let addr1 = Addr::unchecked(ADMIN);
    //claim is fail.
    //Because mint function when core_setting.
    let claim_result = app.execute_contract(
        staking_contract.to_owned(),
        es_axis_contract.to_owned(),
        &ExecuteMsg::Claim {
            sender: addr1.to_owned(),
            amount: Uint128::new(10000),
        },
        &vec![],
    );
    assert!(claim_result.is_err());
    let es_axis_config_res: ConfigResponse = app
        .wrap()
        .query_wasm_smart(es_axis_contract.to_owned(), &QueryMsg::GetConfig {})
        .unwrap();
    assert_eq!(es_axis_config_res.es_axis_total_supply, Uint128::new(0));

    let es_axis_denom = es_axis_config_res.es_axis_denom;
    //test mint
    let mint_result = app.execute_contract(
        staking_contract.to_owned(),
        es_axis_contract.to_owned(),
        &ExecuteMsg::Mint {
            amount: Uint128::new(1000),
        },
        &vec![],
    );
    assert!(mint_result.is_ok());
    //@@ Invalid Claim Only Staking Contract
    let claim_result = app.execute_contract(
        contracts.core_contract.to_owned(),
        es_axis_contract.to_owned(),
        &ExecuteMsg::Claim {
            sender: addr1.to_owned(),
            amount: Uint128::new(1000),
        },
        &vec![],
    );

    assert!(claim_result.is_err());
    let claim_result = app.execute_contract(
        staking_contract.to_owned(),
        es_axis_contract.to_owned(),
        &ExecuteMsg::Claim {
            sender: addr1.to_owned(),
            amount: Uint128::new(1000),
        },
        &vec![],
    );
    assert!(claim_result.is_ok());

    let addr1_es_axis = app
        .wrap()
        .query_balance(addr1.to_owned(), es_axis_denom.to_owned())
        .unwrap();

    assert_eq!(addr1_es_axis.amount.u128(), 1000)
}

#[test]
fn test_burn() {
    let mut app = mock_app(init_default_balances, init_exchange_rates());

    let contracts = setup_init(&mut app, BTC_DENOM, USDC_DENOM);
    let staking_contract = contracts.staking_contract;
    let es_axis_contract = contracts.es_axis_contract;
    let addr1 = Addr::unchecked(ADMIN);

    let es_axis_config_res: ConfigResponse = app
        .wrap()
        .query_wasm_smart(es_axis_contract.to_owned(), &QueryMsg::GetConfig {})
        .unwrap();
    assert_eq!(es_axis_config_res.es_axis_total_supply, Uint128::new(0));

    let es_axis_denom = es_axis_config_res.es_axis_denom;
    //test mint
    let mint_result = app.execute_contract(
        staking_contract.to_owned(),
        es_axis_contract.to_owned(),
        &ExecuteMsg::Mint {
            amount: Uint128::new(1000),
        },
        &vec![],
    );
    assert!(mint_result.is_ok());

    let claim_result = app.execute_contract(
        staking_contract.to_owned(),
        es_axis_contract.to_owned(),
        &ExecuteMsg::Claim {
            sender: addr1.to_owned(),
            amount: Uint128::new(1000),
        },
        &vec![],
    );
    assert!(claim_result.is_ok());

    let addr1_es_axis = app
        .wrap()
        .query_balance(addr1.to_owned(), es_axis_denom.to_owned())
        .unwrap();

    let burn_result = app.execute_contract(
        addr1.to_owned(),
        es_axis_contract.to_owned(),
        &ExecuteMsg::Burn {},
        &vec![addr1_es_axis],
    );
    assert!(burn_result.is_ok());

    let es_axis_config_res: ConfigResponse = app
        .wrap()
        .query_wasm_smart(es_axis_contract.to_owned(), &QueryMsg::GetConfig {})
        .unwrap();
    assert_eq!(es_axis_config_res.es_axis_total_supply.u128(), 0);
    let addr1_es_axis = app
        .wrap()
        .query_balance(addr1.to_owned(), es_axis_denom.to_owned())
        .unwrap();
    assert_eq!(addr1_es_axis.amount.u128(), 0)
}
