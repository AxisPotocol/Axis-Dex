use std::str::FromStr;

use crate::{
    contract::{execute, instantiate, query, reply},
    error::ContractError,
    state::register_market_contract,
};

use cosmwasm_std::{
    coin,
    testing::{MockApi, MockStorage},
    Addr, Api, Decimal, Empty, GovMsg, IbcMsg, IbcQuery, Response, Storage, SubMsgResult, Uint128,
    Uint64,
};
use cw_multi_test::{
    App, BankKeeper, ContractWrapper, DistributionKeeper, Executor, FailingModule, Router,
    StakeKeeper, WasmKeeper,
};

use axis::{
    core::{
        ConfigResponse as CoreConfigResponse, ExecuteMsg as CoreExecuteMsg,
        InstantiateMsg as CoreInstantiateMsg, PairContractResponse, QueryMsg as CoreQueryMsg,
    },
    market::InstantiateMsg as MarketInstantiateMsg,
    pool::{ConfigResponse, InstantiateMsg, QueryMsg},
};

use sei_cosmwasm::{DenomOracleExchangeRatePair, OracleExchangeRate, SeiMsg, SeiQueryWrapper};
use sei_integration_tests::module::SeiModule;

use super::test_market::contract::{
    execute as market_execute, instantiate as market_instantiate, query as market_query,
};
use core::{
    contract::{execute as core_execute, instantiate as core_instantiate, query as core_query},
    error::ContractError as CoreContractError,
    helpers::find_attribute_value,
    state::{register_pair, register_treasury},
};
use treasury::contract::{
    execute as treasury_execute, instantiate as treasury_instantiate, query as treasury_query,
};

pub const ADMIN: &str = "admin";
pub const ASSET_DENOM: &str = "ubtc";
pub const STABLE_DENOM: &str = "uusdc";
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
    _api: &dyn Api,
    storage: &mut dyn Storage,
) {
    router
        .bank
        .init_balance(
            storage,
            &Addr::unchecked(ADMIN),
            vec![
                coin(1_000_000_000_000_000_000, STABLE_DENOM.to_string()),
                coin(1_000_000_000_000_000_000, ASSET_DENOM.to_string()),
            ],
        )
        .unwrap();

    router
        .bank
        .init_balance(
            storage,
            &Addr::unchecked(TRADER1),
            vec![
                coin(100_000, ASSET_DENOM.to_string()),
                coin(100_000, STABLE_DENOM.to_string()),
            ],
        )
        .unwrap();

    router
        .bank
        .init_balance(
            storage,
            &Addr::unchecked(TRADER2),
            vec![
                coin(10_000_000, ASSET_DENOM.to_string()),
                coin(10_000_000, STABLE_DENOM.to_string()),
            ],
        )
        .unwrap();

    router
        .bank
        .init_balance(
            storage,
            &Addr::unchecked(TRADER3),
            vec![
                coin(10_000_000, ASSET_DENOM.to_string()),
                coin(10_000_000, STABLE_DENOM.to_string()),
            ],
        )
        .unwrap();
    router
        .bank
        .init_balance(
            storage,
            &Addr::unchecked(TRADER4),
            vec![
                coin(10_000_000, ASSET_DENOM.to_string()),
                coin(10_000_000, STABLE_DENOM.to_string()),
            ],
        )
        .unwrap();
    router
        .bank
        .init_balance(
            storage,
            &Addr::unchecked(TRADER5),
            vec![
                coin(10_000_000, ASSET_DENOM.to_string()),
                coin(10_000_000, STABLE_DENOM.to_string()),
            ],
        )
        .unwrap();
}

pub fn setup_test(
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
    asset_amount: Uint128,
    stable_amount: Uint128,
) -> (Addr, Addr) {
    let core_code = app.store_code(Box::new(
        Box::new(ContractWrapper::new(
            core_execute,
            core_instantiate,
            core_query,
        ))
        .with_reply(|deps, _env, msg| match msg.id {
            1 => match msg.result {
                SubMsgResult::Ok(res) => match res.data {
                    Some(data) => {
                        let traesury_contract_addr =
                            String::from_utf8(data.0[2..].to_vec()).unwrap();
                        println!("treasury {:?}", traesury_contract_addr);
                        register_treasury(deps.storage, Addr::unchecked(traesury_contract_addr))?;
                        Ok(Response::new())
                    }
                    None => Err(CoreContractError::TreasuryContractInstantiationFailed {}),
                },
                SubMsgResult::Err(_) => {
                    Err(CoreContractError::TreasuryContractInstantiationFailed {})
                }
            },
            2 => match msg.result {
                SubMsgResult::Ok(res) => match res.data {
                    Some(data) => {
                        let pool_contract_addr = String::from_utf8(data.0[2..].to_vec()).unwrap();
                        println!("pool_contract{:?}", pool_contract_addr);
                        let asset_denom =
                            find_attribute_value(&res.events[1].attributes, "asset_denom")?;

                        let stable_denom =
                            find_attribute_value(&res.events[1].attributes, "stable_denom")?;

                        register_pair(
                            deps.storage,
                            asset_denom,
                            stable_denom,
                            Addr::unchecked(pool_contract_addr),
                        )?;
                        Ok(Response::new())
                    }
                    None => Err(CoreContractError::PoolContractInstantiationFailed {}),
                },
                SubMsgResult::Err(_) => Err(CoreContractError::PoolContractInstantiationFailed {}),
            },
            _ => Err(CoreContractError::InvalidReplyId {}),
        }),
    ));
    let treasury_code = app.store_code(Box::new(ContractWrapper::new(
        treasury_execute,
        treasury_instantiate,
        treasury_query,
    )));
    let core_contract = app
        .instantiate_contract(
            core_code,
            Addr::unchecked(ADMIN),
            &CoreInstantiateMsg {
                accept_stable_denoms: vec!["uusdc".to_string()],
                rune_treasury_code_id: treasury_code,
            },
            &vec![],
            "RUNE CORE",
            Some(ADMIN.to_string()),
        )
        .unwrap();

    let core_config_res: CoreConfigResponse = app
        .wrap()
        .query_wasm_smart(core_contract.to_owned(), &CoreQueryMsg::GetConfig {})
        .unwrap();
    let treasury_contract = core_config_res.treausry_contract_address;

    let market_code = app.store_code(Box::new(ContractWrapper::new(execute, instantiate, query))); //::<SeiMsg, SeiQueryWrapper>
    let pool_code = app.store_code(Box::new(
        Box::new(ContractWrapper::new(execute, instantiate, query)).with_reply(
            |deps, _env, msg| match msg.id {
                1 => match msg.result {
                    SubMsgResult::Ok(res) => match res.data {
                        Some(data) => {
                            let string_data = &data.0[2..];
                            let market_contract_addr =
                                String::from_utf8(string_data.to_vec()).unwrap();

                            register_market_contract(
                                deps.storage,
                                Addr::unchecked(market_contract_addr),
                            )?;
                            Ok(Response::new())
                        }
                        None => Err(ContractError::MissingMarketContractAddr {}),
                    },
                    SubMsgResult::Err(_) => {
                        Err(ContractError::MarketContractInstantiationFailed {})
                    }
                },
                _ => Err(ContractError::InvalidReplyId {}),
            },
        ),
    ));
    let create_pair_msg = CoreExecuteMsg::CreatePair {
        pool_init_msg: InstantiateMsg {
            asset_denom: ASSET_DENOM.to_string(),
            asset_decimal: 6,
            stable_denom: STABLE_DENOM.to_string(),
            stable_decimal: 6,
            maximum_borrow_rate: 10,
            market_code_id: market_code,

            market_instantiate_msg: MarketInstantiateMsg {
                asset_denom: ASSET_DENOM.to_string(),
                asset_decimal: 6,
                stable_denom: STABLE_DENOM.to_string(),
                stable_decimal: 6,
                max_leverage: 10,
                borrow_fee_rate: 1,
                open_close_fee_rate: 1,
                limit_profit_loss_open_fee_rate: 2,
                treasury_contract,
                // pool_contract: "abc".to_string(),
                // fee_vault_contract: "bdd".to_string(),
            },
        },
        pool_code_id: pool_code,
    };
    let create_pair_tx = app
        .execute_contract(
            Addr::unchecked(ADMIN),
            core_contract.clone(),
            &create_pair_msg,
            &vec![
                coin(1_000_000_000_000, ASSET_DENOM),
                coin(1_000_000_000_000, STABLE_DENOM),
            ],
        )
        .unwrap();
    let pair_response: PairContractResponse = app
        .wrap()
        .query_wasm_smart(
            core_contract,
            &CoreQueryMsg::GetPairContract {
                asset_denom: ASSET_DENOM.to_owned(),
                stable_denom: STABLE_DENOM.to_owned(),
            },
        )
        .unwrap();

    let pool_contract = pair_response.pool_contract;

    let msg = QueryMsg::GetConfig {};
    let pool_config: ConfigResponse = app
        .wrap()
        .query_wasm_smart(pool_contract.clone(), &msg)
        .unwrap();
    let market_contract = pool_config.market_contract;
    (
        Addr::unchecked(pool_contract),
        Addr::unchecked(market_contract),
    )
}

pub fn init_exchange_rates() -> Vec<DenomOracleExchangeRatePair> {
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
        DenomOracleExchangeRatePair {
            denom: ASSET_DENOM.to_string(),
            oracle_exchange_rate: OracleExchangeRate {
                exchange_rate: Decimal::from_str("11").unwrap(),
                last_update: Uint64::one(),
            },
        },
    ]
}

//@@ repay and leverage_borrow function is market contract test complete
