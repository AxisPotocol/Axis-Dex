use anyhow::Error;
use axis_protocol::core::ExecuteMsg;
use cosmwasm_std::{
    coin,
    testing::{MockApi, MockStorage},
    Addr, Decimal, Empty, GovMsg, IbcMsg, IbcQuery, Uint128,
};
use cw_multi_test::{
    App, AppResponse, BankKeeper, ContractWrapper, DistributionKeeper, Executor, FailingModule,
    StakeKeeper, WasmKeeper,
};

use sei_cosmwasm::{SeiMsg, SeiQueryWrapper};
use sei_integration_tests::{helper::mock_app, module::SeiModule};

use crate::app::{
    create_pair, init_default_balances, init_exchange_rates, setup_init, Contracts, ADMIN,
    BTC_DENOM, ETH_DENOM, TRADER1, USDC_DENOM, USDT_DENOM,
};
use axis_protocol::{
    axis::{ConfigResponse as AxisConfigResponse, QueryMsg as AxisQueryMsg},
    core::{
        ConfigResponse as CoreConfigResponse, ExecuteMsg as CoreExecuteMsg,
        InstantiateMsg as CoreInstantiateMsg, PairLpStakingContractResponse,
        PairMarketContractResponse, PairPoolContractResponse, QueryMsg as CoreQueryMsg,
    },
    es_axis::{ConfigResponse as EsAxisConfigResponse, QueryMsg as EsAxisQueryMsg},
    market::InstantiateMsg as MarketInstantiateMsg,
    pool::{
        ConfigResponse as PoolConfigResponse, InstantiateMsg as PoolInstantiateMsg,
        QueryMsg as PoolQueryMsg,
    },
    staking::{
        ConfigResponse as StakingConfigResponse, InstantiateMsg as StakingInstatiateMsg,
        QueryMsg as StakingQueryMsg,
    },
    vault::InstantiateMsg as VaultInstantiateMsg,
};
use lp_staking::contract::{
    execute as lp_staking_execute, instantiate as lp_staking_instantiate, query as lp_staking_query,
};
use market::contract::{
    execute as market_execute, instantiate as market_instantiate, query as market_query,
};
use pool::contract::{
    execute as pool_execute, instantiate as pool_instantiate, query as pool_query,
    reply as pool_reply,
};

#[test]
fn invalid_create_pair() {
    let mut app = mock_app(init_default_balances, init_exchange_rates());
    let contracts = setup_init(&mut app, BTC_DENOM, USDC_DENOM);
    let result = create_pair(
        &mut app,
        &Addr::unchecked(ADMIN),
        &contracts,
        ETH_DENOM,
        USDC_DENOM,
        1_000_000,
        1_000_000,
    );
    assert!(result.is_ok())
}

#[test]
fn register_price_denom() {
    let mut app = mock_app(init_default_balances, init_exchange_rates());
    let contracts = setup_init(&mut app, BTC_DENOM, USDC_DENOM);
    let admin = Addr::unchecked(ADMIN);
    let result = app.execute_contract(
        admin.to_owned(),
        contracts.core_contract.to_owned(),
        &ExecuteMsg::RegisterPriceDenom {
            price_denom: USDT_DENOM.to_string(),
        },
        &vec![],
    );
    assert!(result.is_ok());

    //An already existing denom
    let result = app.execute_contract(
        admin,
        contracts.core_contract.to_owned(),
        &ExecuteMsg::RegisterPriceDenom {
            price_denom: USDC_DENOM.to_string(),
        },
        &vec![],
    );
    assert!(result.is_err());

    let not_admin = Addr::unchecked(TRADER1);
    let result = app.execute_contract(
        not_admin,
        contracts.core_contract,
        &ExecuteMsg::RegisterPriceDenom {
            price_denom: USDT_DENOM.to_string(),
        },
        &vec![coin(1_000, BTC_DENOM), coin(1_000_000, USDC_DENOM)],
    );
    assert!(result.is_err());
}

#[test]
fn pool_lock_and_un_lock() {
    let mut app = mock_app(init_default_balances, init_exchange_rates());
    let contracts = setup_init(&mut app, BTC_DENOM, USDC_DENOM);
    let admin = Addr::unchecked(ADMIN);

    //not owner
    let result = create_pair(
        &mut app, &admin, &contracts, ETH_DENOM, USDC_DENOM, 1_000_000, 1_000_000,
    );

    assert!(result.is_ok());

    //@@@Lock
    let result = app.execute_contract(
        admin.to_owned(),
        contracts.core_contract.to_owned(),
        &ExecuteMsg::AllPoolLock {},
        &vec![],
    );
    assert!(result.is_ok());

    let core_res: PairPoolContractResponse = app
        .wrap()
        .query_wasm_smart(
            contracts.core_contract.to_owned(),
            &CoreQueryMsg::GetPairPoolContract {
                base_denom: ETH_DENOM.to_string(),
                price_denom: USDC_DENOM.to_string(),
            },
        )
        .unwrap();
    let pool_contract2 = core_res.pool_contract;

    let pool2_res: PoolConfigResponse = app
        .wrap()
        .query_wasm_smart(pool_contract2.to_owned(), &PoolQueryMsg::GetConfig {})
        .unwrap();
    assert_eq!(true, pool2_res.lock);
    let pool_res: PoolConfigResponse = app
        .wrap()
        .query_wasm_smart(
            contracts.pool_contract.to_owned(),
            &PoolQueryMsg::GetConfig {},
        )
        .unwrap();
    assert_eq!(true, pool_res.lock);

    //@@@@Unlock
    let result = app.execute_contract(
        admin.to_owned(),
        contracts.core_contract.to_owned(),
        &ExecuteMsg::AllPoolUnLock {},
        &vec![],
    );
    assert!(result.is_ok());

    let pool2_res: PoolConfigResponse = app
        .wrap()
        .query_wasm_smart(pool_contract2.to_owned(), &PoolQueryMsg::GetConfig {})
        .unwrap();
    assert_eq!(false, pool2_res.lock);
    let pool_res: PoolConfigResponse = app
        .wrap()
        .query_wasm_smart(
            contracts.pool_contract.to_owned(),
            &PoolQueryMsg::GetConfig {},
        )
        .unwrap();
    assert_eq!(false, pool_res.lock);
}

#[test]
fn setting() {
    let mut app = mock_app(init_default_balances, init_exchange_rates());
    let contracts = setup_init(&mut app, BTC_DENOM, USDC_DENOM);
    let admin = Addr::unchecked(ADMIN);

    //axis vault axis_staking 확인해야함.
    //어떻게? 트레이딩하고 다하는 함수 만들고 테스트하자.
}
