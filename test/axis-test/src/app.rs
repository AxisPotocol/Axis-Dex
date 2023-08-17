use std::str::FromStr;

use anyhow::Error;
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    coin,
    testing::{MockApi, MockStorage},
    Addr, Api, Decimal, Empty, GovMsg, IbcMsg, IbcQuery, Storage, Uint64,
};
use cw_multi_test::{
    App, BankKeeper, ContractWrapper, DistributionKeeper, Executor, FailingModule, Router,
    StakeKeeper, WasmKeeper,
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
    pool::InstantiateMsg as PoolInstantiateMsg,
    staking::{
        ConfigResponse as StakingConfigResponse, InstantiateMsg as StakingInstatiateMsg,
        QueryMsg as StakingQueryMsg,
    },
    vault::InstantiateMsg as VaultInstantiateMsg,
};

use sei_cosmwasm::{DenomOracleExchangeRatePair, OracleExchangeRate, SeiMsg, SeiQueryWrapper};
use sei_integration_tests::module::SeiModule;

use axis::contract::{
    execute as axis_execute, instantiate as axis_instantiate, query as axis_query,
};
use core::contract::{
    execute as core_execute, instantiate as core_instantiate, query as core_query,
    reply as core_reply,
};
use market::contract::{
    execute as market_execute, instantiate as market_instantiate, query as market_query,
};
use pool::contract::{
    execute as pool_execute, instantiate as pool_instantiate, query as pool_query,
    reply as pool_reply,
};
use vault::contract::{
    execute as vault_execute, instantiate as vault_instantiate, query as vault_query,
};

use staking::contract::{
    execute as staking_execute, instantiate as staking_instantiate, query as staking_query,
    reply as staking_reply,
};

use es_axis::contract::{
    execute as es_axis_execute, instantiate as es_axis_instantiate, query as es_axis_query,
};
use lp_staking::contract::{
    execute as lp_staking_execute, instantiate as lp_staking_instantiate, query as lp_staking_query,
};
#[cw_serde]
pub struct Contracts {
    pub market_contract: Addr,
    pub core_contract: Addr,
    pub pool_contract: Addr,
    pub staking_contract: Addr,
    pub vault_contract: Addr,
    pub es_axis_contract: Addr,
    pub axis_contract: Addr,
    pub lp_staking_contract: Addr,
}
pub const ADMIN: &str = "admin";
pub const BTC_DENOM: &str = "ubtc";
pub const USDC_DENOM: &str = "uusdc";
pub const ETH_DENOM: &str = "ueth";
pub const USDT_DENOM: &str = "uusdt";
pub const TRADER1: &str = "trader1";
pub const TRADER2: &str = "trader2";
pub const TRADER3: &str = "trader3";
pub const TRADER4: &str = "trader4";
pub const TRADER5: &str = "trader5";
pub fn init_default_balances(
    router: &mut Router<
        BankKeeper,
        SeiModule,
        WasmKeeper<SeiMsg, SeiQueryWrapper>,
        StakeKeeper,
        DistributionKeeper,
        FailingModule<IbcMsg, IbcQuery, Empty>,
        FailingModule<GovMsg, Empty, Empty>,
    >,
    api: &dyn Api,
    storage: &mut dyn Storage,
) {
    router
        .bank
        .init_balance(
            storage,
            &Addr::unchecked(ADMIN),
            vec![
                coin(1_000_000_000_000_000_000, USDC_DENOM.to_string()),
                coin(1_000_000_000_000_000_000, BTC_DENOM.to_string()),
                coin(1_000_000_000_000_000_000, ETH_DENOM.to_string()),
                coin(1_000_000_000_000_000_000, USDT_DENOM.to_string()),
            ],
        )
        .unwrap();

    router
        .bank
        .init_balance(
            storage,
            &Addr::unchecked(TRADER1),
            vec![
                coin(10_000_000, BTC_DENOM.to_string()),
                coin(10_000_000, USDC_DENOM.to_string()),
                coin(10_000_000, ETH_DENOM.to_string()),
                coin(10_000_000, USDT_DENOM.to_string()),
            ],
        )
        .unwrap();

    router
        .bank
        .init_balance(
            storage,
            &Addr::unchecked(TRADER2),
            vec![
                coin(10_000_000, BTC_DENOM.to_string()),
                coin(10_000_000, USDC_DENOM.to_string()),
                coin(10_000_000, ETH_DENOM.to_string()),
                coin(10_000_000, USDT_DENOM.to_string()),
            ],
        )
        .unwrap();

    router
        .bank
        .init_balance(
            storage,
            &Addr::unchecked(TRADER3),
            vec![
                coin(10_000_000, BTC_DENOM.to_string()),
                coin(10_000_000, USDC_DENOM.to_string()),
            ],
        )
        .unwrap();
    router
        .bank
        .init_balance(
            storage,
            &Addr::unchecked(TRADER4),
            vec![
                coin(10_000_000, BTC_DENOM.to_string()),
                coin(10_000_000, USDC_DENOM.to_string()),
                coin(10_000_000, ETH_DENOM.to_string()),
                coin(10_000_000, USDT_DENOM.to_string()),
            ],
        )
        .unwrap();
    router
        .bank
        .init_balance(
            storage,
            &Addr::unchecked(TRADER5),
            vec![
                coin(10_000_000, BTC_DENOM.to_string()),
                coin(10_000_000, USDC_DENOM.to_string()),
                coin(10_000_000, ETH_DENOM.to_string()),
                coin(10_000_000, USDT_DENOM.to_string()),
            ],
        )
        .unwrap();
}

pub fn setup_init(
    app: &mut App<
        BankKeeper,
        MockApi,
        MockStorage,
        SeiModule,
        WasmKeeper<SeiMsg, SeiQueryWrapper>,
        StakeKeeper,
        DistributionKeeper,
        FailingModule<IbcMsg, IbcQuery, Empty>,
        FailingModule<GovMsg, Empty, Empty>,
    >,
    base_denom: &str,
    price_denom: &str,
) -> Contracts {
    let axis_code = app.store_code(Box::new(ContractWrapper::new(
        axis_execute,
        axis_instantiate,
        axis_query,
    )));
    let pool_code = app.store_code(Box::new(
        Box::new(ContractWrapper::new(
            pool_execute,
            pool_instantiate,
            pool_query,
        ))
        .with_reply(pool_reply),
    ));

    let vault_code = app.store_code(Box::new(ContractWrapper::new(
        vault_execute,
        vault_instantiate,
        vault_query,
    )));
    let staking_code = app.store_code(Box::new(
        Box::new(ContractWrapper::new(
            staking_execute,
            staking_instantiate,
            staking_query,
        ))
        .with_reply(staking_reply),
    ));
    let es_axis_code = app.store_code(Box::new(ContractWrapper::new(
        es_axis_execute,
        es_axis_instantiate,
        es_axis_query,
    )));
    let lp_staking_code = app.store_code(Box::new(ContractWrapper::new(
        lp_staking_execute,
        lp_staking_instantiate,
        lp_staking_query,
    )));
    let market_code = app.store_code(Box::new(ContractWrapper::new(
        market_execute,
        market_instantiate,
        market_query,
    )));
    let core_code = app.store_code(Box::new(
        Box::new(ContractWrapper::new(
            core_execute,
            core_instantiate,
            core_query,
        ))
        .with_reply(core_reply),
    ));

    let core_contract = app
        .instantiate_contract(
            core_code,
            Addr::unchecked(ADMIN),
            &CoreInstantiateMsg {
                accept_price_denoms: vec![USDC_DENOM.to_string()],
                axis_code_id: axis_code,
                next_update_timestamp: 10,
            },
            &vec![],
            "Axis Core",
            Some(ADMIN.to_string()),
        )
        .unwrap();

    let core_config_res: CoreConfigResponse = app
        .wrap()
        .query_wasm_smart(core_contract.to_owned(), &CoreQueryMsg::GetConfig {})
        .unwrap();
    let axis_contract = core_config_res.axis_contract;

    let axis_config_res: AxisConfigResponse = app
        .wrap()
        .query_wasm_smart(axis_contract.to_owned(), &AxisQueryMsg::GetConfig {})
        .unwrap();
    let staking_contract = app
        .instantiate_contract(
            staking_code,
            Addr::unchecked(ADMIN),
            &StakingInstatiateMsg {
                core_contract: core_contract.to_owned(),
                axis_denom: axis_config_res.axis_denom,
                es_axis_code,
            },
            &vec![],
            "axis contract",
            Some(ADMIN.to_string()),
        )
        .unwrap();
    let staking_config_res: StakingConfigResponse = app
        .wrap()
        .query_wasm_smart(staking_contract.to_owned(), &StakingQueryMsg::GetConfig {})
        .unwrap();
    let es_axis_contract = staking_config_res.es_axis_contract;
    let es_axis_config_res: EsAxisConfigResponse = app
        .wrap()
        .query_wasm_smart(es_axis_contract.to_owned(), &EsAxisQueryMsg::GetConfig {})
        .unwrap();
    let es_axis_denom = es_axis_config_res.es_axis_denom;
    let vault_contract = app
        .instantiate_contract(
            vault_code,
            Addr::unchecked(ADMIN),
            &VaultInstantiateMsg {
                core_contract: core_contract.to_string(),
                es_axis_contract: es_axis_contract.to_string(),
                es_axis_denom,
                denom_list: vec![USDC_DENOM.to_string(), BTC_DENOM.to_string()],
            },
            &vec![],
            "axis_vault",
            None,
        )
        .unwrap();
    //@@ core setting
    app.execute_contract(
        Addr::unchecked(ADMIN),
        core_contract.to_owned(),
        &CoreExecuteMsg::UpdateConfig {
            vault_contract: Some(vault_contract.to_string()),
            staking_contract: Some(staking_contract.to_string()),
        },
        &vec![],
    )
    .unwrap();

    app.execute_contract(
        Addr::unchecked(ADMIN),
        core_contract.clone(),
        &CoreExecuteMsg::CreatePair {
            pool_init_msg: PoolInstantiateMsg {
                base_denom: base_denom.to_string(),
                base_decimal: 6,
                price_denom: price_denom.to_string(),
                price_decimal: 6,
                maximum_borrow_rate: 10,
                market_code_id: market_code,
                market_instantiate_msg: MarketInstantiateMsg {
                    base_denom: base_denom.to_string(),
                    base_decimal: 6,
                    price_denom: price_denom.to_string(),
                    price_decimal: 6,
                    max_leverage: 10,
                    borrow_fee_rate: 1,
                    open_close_fee_rate: 1,
                    limit_profit_loss_open_fee_rate: 2,
                    axis_contract: axis_contract.to_owned(),
                    vault_contract: vault_contract.to_owned(),
                },
                lp_staking_code_id: lp_staking_code,
                maker: Addr::unchecked(ADMIN),
                axis_contract: axis_contract.to_owned(),
            },
            pool_code_id: pool_code,
        },
        &vec![
            coin(1_000_000_000_000, BTC_DENOM),
            coin(1_000_000_000_000, USDC_DENOM),
        ],
    )
    .unwrap();
    let pair_response: PairPoolContractResponse = app
        .wrap()
        .query_wasm_smart(
            core_contract.to_owned(),
            &CoreQueryMsg::GetPairPoolContract {
                base_denom: BTC_DENOM.to_owned(),
                price_denom: USDC_DENOM.to_owned(),
            },
        )
        .unwrap();

    let pool_contract = pair_response.pool_contract;
    let core_res: PairMarketContractResponse = app
        .wrap()
        .query_wasm_smart(
            core_contract.to_owned(),
            &CoreQueryMsg::GetPairMarketContract {
                base_denom: BTC_DENOM.to_string(),
                price_denom: USDC_DENOM.to_string(),
            },
        )
        .unwrap();
    let market_contract = core_res.market_contract;

    let core_res: PairLpStakingContractResponse = app
        .wrap()
        .query_wasm_smart(
            core_contract.to_owned(),
            &CoreQueryMsg::GetPairLpStakingContract {
                base_denom: BTC_DENOM.to_string(),
                price_denom: USDC_DENOM.to_string(),
            },
        )
        .unwrap();
    let lp_staking_contract = core_res.lp_staking_contract;
    Contracts {
        market_contract,
        core_contract,
        pool_contract,
        staking_contract,
        vault_contract,
        es_axis_contract,
        axis_contract,
        lp_staking_contract,
    }
}

pub fn init_exchange_rates() -> Vec<DenomOracleExchangeRatePair> {
    vec![
        DenomOracleExchangeRatePair {
            denom: USDC_DENOM.to_string(),
            oracle_exchange_rate: OracleExchangeRate {
                exchange_rate: Decimal::from_str("1").unwrap(),
                last_update: Uint64::zero(),
            },
        },
        DenomOracleExchangeRatePair {
            denom: BTC_DENOM.to_string(),
            oracle_exchange_rate: OracleExchangeRate {
                exchange_rate: Decimal::from_str("10000").unwrap(),
                last_update: Uint64::zero(),
            },
        },
        DenomOracleExchangeRatePair {
            denom: ETH_DENOM.to_string(),
            oracle_exchange_rate: OracleExchangeRate {
                exchange_rate: Decimal::from_str("1000").unwrap(),
                last_update: Uint64::zero(),
            },
        },
    ]
}

pub fn create_pair(
    app: &mut App<
        BankKeeper,
        MockApi,
        MockStorage,
        SeiModule,
        WasmKeeper<SeiMsg, SeiQueryWrapper>,
        StakeKeeper,
        DistributionKeeper,
        FailingModule<IbcMsg, IbcQuery, Empty>,
        FailingModule<GovMsg, Empty, Empty>,
    >,
    sender: &Addr,
    contracts: &Contracts,
    base_denom: &str,
    price_denom: &str,
    base_amount: u128,
    price_amount: u128,
) -> Result<Addr, Error> {
    let pool_code = app.store_code(Box::new(
        Box::new(ContractWrapper::new(
            pool_execute,
            pool_instantiate,
            pool_query,
        ))
        .with_reply(pool_reply),
    ));
    let market_code = app.store_code(Box::new(ContractWrapper::new(
        market_execute,
        market_instantiate,
        market_query,
    )));
    let lp_staking_code = app.store_code(Box::new(ContractWrapper::new(
        lp_staking_execute,
        lp_staking_instantiate,
        lp_staking_query,
    )));
    let result = app.execute_contract(
        sender.to_owned(),
        contracts.core_contract.to_owned(),
        &CoreExecuteMsg::CreatePair {
            pool_init_msg: PoolInstantiateMsg {
                base_denom: base_denom.to_owned(),
                base_decimal: 6,
                price_denom: price_denom.to_owned(),
                price_decimal: 6,
                maximum_borrow_rate: 10,
                market_code_id: market_code,

                market_instantiate_msg: MarketInstantiateMsg {
                    base_denom: base_denom.to_owned(),
                    base_decimal: 6,
                    price_denom: price_denom.to_owned(),
                    price_decimal: 6,
                    max_leverage: 10,
                    borrow_fee_rate: 1,
                    open_close_fee_rate: 1,
                    limit_profit_loss_open_fee_rate: 2,
                    axis_contract: contracts.axis_contract.to_owned(),
                    vault_contract: contracts.vault_contract.to_owned(),
                },
                lp_staking_code_id: lp_staking_code,
                maker: sender.to_owned(),
                axis_contract: contracts.axis_contract.to_owned(),
            },
            pool_code_id: pool_code,
        },
        &vec![
            coin(base_amount, base_denom),
            coin(price_amount, price_denom),
        ],
    );
    assert!(result.is_ok());
    let core_res: PairPoolContractResponse = app
        .wrap()
        .query_wasm_smart(
            contracts.core_contract.to_owned(),
            &CoreQueryMsg::GetPairPoolContract {
                base_denom: base_denom.to_string(),
                price_denom: price_denom.to_string(),
            },
        )
        .unwrap();
    let pool_contract = core_res.pool_contract;
    Ok(pool_contract)
}
