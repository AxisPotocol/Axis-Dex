use anyhow::Error;
use cosmwasm_std::{coin, Addr, Coin};

use sei_cosmwasm::{SeiMsg, SeiQueryWrapper};
use sei_integration_tests::module::SeiModule;

use axis_protocol::{
    core::ExecuteMsg as CoreExecuteMsg, market::ExecuteMsg as MarketExecuteMsg,
    pool::ExecuteMsg as PoolExecuteMsg, staking::ExecuteMsg as StakingExecuteMsg,
};
use cosmwasm_std::{
    testing::{MockApi, MockStorage},
    Empty, GovMsg, IbcMsg, IbcQuery,
};
use cw_multi_test::{
    App, AppResponse, BankKeeper, DistributionKeeper, Executor, FailingModule, StakeKeeper,
    WasmKeeper,
};
pub fn deposit(
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
    pool_contract: &Addr,
    maker: &Addr,
    base_denom: &str,
    price_denom: &str,
    base_amount: u128,
    price_amount: u128,
) {
    let deposit_msg = PoolExecuteMsg::Deposit {};

    let deposit_result = app.execute_contract(
        maker.to_owned(),
        pool_contract.to_owned(),
        &deposit_msg,
        &vec![
            coin(base_amount, base_denom),
            coin(price_amount, price_denom),
        ],
    );
    assert!(deposit_result.is_ok());
}

pub fn withdraw(
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
    pool_contract: &Addr,
    maker: &Addr,
    lp_amount: u128,
    lp_denom: &str,
) {
    let withdraw_msg = PoolExecuteMsg::Withdraw {};
    let withdraw_result = app.execute_contract(
        maker.to_owned(),
        pool_contract.to_owned(),
        &withdraw_msg,
        &vec![coin(lp_amount, lp_denom)],
    );

    assert!(withdraw_result.is_ok());
}

pub fn position_open(
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
    market_contract: &Addr,
    trader: &Addr,
    position: bool,
    leverage: u8,
    position_amount: u128,
    position_denom: &str,
) -> Result<AppResponse, Error> {
    let open_msg = &MarketExecuteMsg::Open {
        position,
        leverage,
        limit_profit_price: None,
        limit_loss_price: None,
    };
    let result = app.execute_contract(
        trader.to_owned(),
        market_contract.to_owned(),
        open_msg,
        &vec![coin(position_amount, position_denom)],
    );
    result
}

pub fn position_close(
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
    market_contract: &Addr,
    trader: &Addr,
) -> Result<AppResponse, Error> {
    let close_msg = &MarketExecuteMsg::Close {};
    let result = app.execute_contract(
        trader.to_owned(),
        market_contract.to_owned(),
        close_msg,
        &vec![],
    );
    result
}

pub fn staking(
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
    staking_contract: &Addr,
    staker: &Addr,
    funds: &Vec<Coin>,
) -> Result<AppResponse, Error> {
    let staking_msg = &StakingExecuteMsg::Staking {};
    let result = app.execute_contract(
        staker.to_owned(),
        staking_contract.to_owned(),
        staking_msg,
        funds,
    );
    result
}

pub fn un_staking(
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
    staking_contract: &Addr,
    staker: &Addr,
) -> Result<AppResponse, Error> {
    let un_staking_msg = &StakingExecuteMsg::UnStaking {};
    let result = app.execute_contract(
        staker.to_owned(),
        staking_contract.to_owned(),
        un_staking_msg,
        &vec![],
    );
    result
}

pub fn setting(
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
    core_contract: &Addr,
    setter: &Addr,
) -> Result<AppResponse, Error> {
    let setting_msg = &CoreExecuteMsg::Setting {};
    let result = app.execute_contract(
        setter.to_owned(),
        core_contract.to_owned(),
        setting_msg,
        &vec![],
    );
    result
}
