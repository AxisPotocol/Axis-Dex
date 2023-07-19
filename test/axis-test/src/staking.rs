use cosmwasm_std::{Addr, Uint128};
use cw_multi_test::{Executor, SudoMsg};

use sei_cosmwasm::{SeiMsg, SeiRoute, SudoMsg as SeiSudoMsg};
use sei_integration_tests::{helper::mock_app, module::SeiModule};

use crate::{
    app::{init_default_balances, init_exchange_rates, setup_init, ADMIN, BTC_DENOM, USDC_DENOM},
    staking,
    utils::{setting, staking, un_staking},
};

use axis_protocol::{
    axis::{ConfigResponse as AxisConfigResponse, QueryMsg as AxisQueryMsg},
    core::{ConfigResponse as CoreConfigResponse, QueryMsg as CoreQueryMsg},
    staking::{
        ConfigResponse as StakingConfigResponse, ExecuteMsg, QueryMsg, StakeInfoResponse,
        StakeResponse, StateResponse, UnStakeInfoResponse, UnStakeResponse,
    },
};
#[test]
fn valid_staking() {
    let mut app = mock_app(init_default_balances, init_exchange_rates());

    let contracts = setup_init(&mut app, BTC_DENOM, USDC_DENOM);

    let staking_contract = contracts.staking_contract;

    let axis_contract = contracts.axis_contract;
    let axis_res: AxisConfigResponse = app
        .wrap()
        .query_wasm_smart(axis_contract, &AxisQueryMsg::GetConfig {})
        .unwrap();

    let axis_denom = axis_res.axis_denom;
    let before_staking_admin_axis = app
        .wrap()
        .query_balance(ADMIN, axis_denom.to_owned())
        .unwrap();
    let result = staking(
        &mut app,
        &staking_contract,
        &Addr::unchecked(ADMIN),
        &vec![before_staking_admin_axis.to_owned()],
    );

    assert!(result.is_ok());
    let after_admin_axis = app.wrap().query_balance(ADMIN, axis_denom).unwrap();
    assert_eq!(after_admin_axis.amount, Uint128::zero());

    let staking_res: StateResponse = app
        .wrap()
        .query_wasm_smart(staking_contract.to_owned(), &QueryMsg::GetState {})
        .unwrap();

    assert_eq!(
        staking_res.pending_staking_total,
        before_staking_admin_axis.amount
    );
    assert_eq!(staking_res.staking_total, Uint128::zero());

    let stake_info_res: StakeInfoResponse = app
        .wrap()
        .query_wasm_smart(
            staking_contract.to_owned(),
            &QueryMsg::GetStakeInfo { addr: ADMIN.into() },
        )
        .unwrap();
    //@@ current epoch = 0
    assert_eq!(stake_info_res.stake_infos[0].start_epoch, 1);
    assert_eq!(
        stake_info_res.stake_infos[0].staking_amount,
        before_staking_admin_axis.amount
    );
}

#[test]
fn valid_unstaking() {
    let mut app = mock_app(init_default_balances, init_exchange_rates());

    let contracts = setup_init(&mut app, BTC_DENOM, USDC_DENOM);
    let staker = Addr::unchecked(ADMIN);
    let staking_contract = contracts.staking_contract;

    let axis_contract = contracts.axis_contract;
    let axis_res: AxisConfigResponse = app
        .wrap()
        .query_wasm_smart(axis_contract, &AxisQueryMsg::GetConfig {})
        .unwrap();

    let axis_denom = axis_res.axis_denom;
    let staking_admin_axis = app
        .wrap()
        .query_balance(ADMIN, axis_denom.to_owned())
        .unwrap();
    let result = staking(
        &mut app,
        &staking_contract,
        &staker,
        &vec![staking_admin_axis.to_owned()],
    );
    assert!(result.is_ok());

    let result = un_staking(&mut app, &staking_contract, &staker);
    assert!(result.is_ok());
    let un_stake_info_res: UnStakeInfoResponse = app
        .wrap()
        .query_wasm_smart(
            staking_contract,
            &QueryMsg::GetUnStakeInfo {
                addr: staker.to_string(),
            },
        )
        .unwrap();
    let UnStakeInfoResponse { un_stake_infos } = un_stake_info_res;

    assert_eq!(un_stake_infos[0].unlock_epoch, 1);
    assert_eq!(
        un_stake_infos[0].withdraw_pending_amount,
        staking_admin_axis.amount
    );
    let staking_admin_axis = app
        .wrap()
        .query_balance(ADMIN, axis_denom.to_owned())
        .unwrap();
    assert_eq!(staking_admin_axis.amount, Uint128::zero());
}

#[test]
fn test_withdraw() {
    let mut app = mock_app(init_default_balances, init_exchange_rates());

    let contracts = setup_init(&mut app, BTC_DENOM, USDC_DENOM);
    let staker = Addr::unchecked(ADMIN);
    let staking_contract = contracts.staking_contract;

    let axis_contract = contracts.axis_contract;
    let axis_res: AxisConfigResponse = app
        .wrap()
        .query_wasm_smart(axis_contract, &AxisQueryMsg::GetConfig {})
        .unwrap();

    let axis_denom = axis_res.axis_denom;
    let staking_admin_axis = app
        .wrap()
        .query_balance(ADMIN, axis_denom.to_owned())
        .unwrap();
    let result = staking(
        &mut app,
        &staking_contract,
        &staker,
        &vec![staking_admin_axis.to_owned()],
    );
    assert!(result.is_ok());

    let result = un_staking(&mut app, &staking_contract, &staker);
    assert!(result.is_ok());

    let result = app.execute_contract(
        staker.to_owned(),
        staking_contract.to_owned(),
        &ExecuteMsg::Withdraw {},
        &vec![],
    );

    assert!(result.is_ok());

    //@@ witdhraw is success
    //but Funds will not be dissolved
    //if the Un_lock epoch is not reached.
    let un_stake_info_res: UnStakeInfoResponse = app
        .wrap()
        .query_wasm_smart(
            staking_contract,
            &QueryMsg::GetUnStakeInfo {
                addr: staker.to_string(),
            },
        )
        .unwrap();
    let UnStakeInfoResponse { un_stake_infos } = un_stake_info_res;

    assert_eq!(un_stake_infos[0].unlock_epoch, 1);
    assert_eq!(
        un_stake_infos[0].withdraw_pending_amount,
        staking_admin_axis.amount
    );

    let staking_admin_axis = app
        .wrap()
        .query_balance(ADMIN, axis_denom.to_owned())
        .unwrap();

    assert_eq!(staking_admin_axis.amount, Uint128::zero());
}
