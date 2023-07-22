use axis::state::TRADER;
use cosmwasm_std::{coin, Addr, Uint128};
use cw_multi_test::{custom_app, Executor, SudoMsg};

use ::staking::state::StakeInfo;
use sei_cosmwasm::{SeiMsg, SeiRoute, SudoMsg as SeiSudoMsg};
use sei_integration_tests::{helper::mock_app, module::SeiModule};

use crate::{
    app::{
        self, init_default_balances, init_exchange_rates, setup_init, ADMIN, BTC_DENOM, TRADER1,
        USDC_DENOM,
    },
    staking,
    utils::{setting, staking, un_staking},
};

use axis_protocol::{
    axis::{ConfigResponse as AxisConfigResponse, QueryMsg as AxisQueryMsg},
    core::{
        ConfigResponse as CoreConfigResponse, QueryMsg as CoreQueryMsg, SudoMsg as CoreSudoMsg,
    },
    es_axis::{ConfigResponse as EsAxisConfigResponse, QueryMsg as EsAxisQueryMsg},
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
    //@@ current epoch = 1
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
    let core_contract = contracts.core_contract;
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

    app.update_block(|block| block.time = block.time.plus_days(1));
    let setting_result = setting(&mut app, &core_contract, &staker);

    assert!(setting_result.is_ok());

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

    assert_eq!(un_stake_infos[0].unlock_epoch, 2);
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
    let core_contract = contracts.core_contract;
    let axis_contract = contracts.axis_contract;
    let es_axis_contract = contracts.es_axis_contract;
    let axis_res: AxisConfigResponse = app
        .wrap()
        .query_wasm_smart(axis_contract, &AxisQueryMsg::GetConfig {})
        .unwrap();

    let axis_denom = axis_res.axis_denom;

    let ex_axis_config_res: EsAxisConfigResponse = app
        .wrap()
        .query_wasm_smart(es_axis_contract.to_owned(), &EsAxisQueryMsg::GetConfig {})
        .unwrap();
    let es_axis_denom = ex_axis_config_res.es_axis_denom;
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
    {
        app.update_block(|block| block.time = block.time.plus_days(1));
        //epoch is 1
        let setting_result = setting(&mut app, &core_contract, &staker);
        assert!(setting_result.is_ok());

        app.update_block(|block| block.time = block.time.plus_days(1));
        //epoch is 2
        let setting_result = setting(&mut app, &core_contract, &staker);
        assert!(setting_result.is_ok());
        let es_axis_contract_balance = app
            .wrap()
            .query_balance(es_axis_contract, es_axis_denom.to_owned())
            .unwrap();
        assert_eq!(
            es_axis_contract_balance.amount.u128(),
            1_000_000_000_000_000
        );
    }

    let result = un_staking(&mut app, &staking_contract, &staker);

    assert!(result.is_ok());

    let withdraw_result = app.execute_contract(
        staker.to_owned(),
        staking_contract.to_owned(),
        &ExecuteMsg::Withdraw {},
        &vec![],
    );

    assert!(withdraw_result.is_ok());

    //@@ witdhraw is success
    //but Funds will not be dissolved
    //if the Un_lock epoch is not reached.
    let un_stake_info_res: UnStakeInfoResponse = app
        .wrap()
        .query_wasm_smart(
            staking_contract.to_owned(),
            &QueryMsg::GetUnStakeInfo {
                addr: staker.to_string(),
            },
        )
        .unwrap();
    let UnStakeInfoResponse { un_stake_infos } = un_stake_info_res;

    assert_eq!(un_stake_infos[0].unlock_epoch, 3);
    assert_eq!(
        un_stake_infos[0].withdraw_pending_amount,
        staking_admin_axis.amount
    );

    let return_staking_admin_axis = app
        .wrap()
        .query_balance(ADMIN, axis_denom.to_owned())
        .unwrap();

    assert_eq!(return_staking_admin_axis.amount, Uint128::zero());

    //epoch 3
    let setting_result = setting(&mut app, &core_contract, &staker);

    assert!(setting_result.is_ok());

    let withdraw_result = app.execute_contract(
        staker.to_owned(),
        staking_contract.to_owned(),
        &ExecuteMsg::Withdraw {},
        &vec![],
    );

    assert!(withdraw_result.is_ok());
    let return_staking_admin_axis = app
        .wrap()
        .query_balance(ADMIN, axis_denom.to_owned())
        .unwrap();
    assert_eq!(staking_admin_axis.amount, return_staking_admin_axis.amount);

    let admin_es_axis = app
        .wrap()
        .query_balance(ADMIN, es_axis_denom.to_owned())
        .unwrap();

    assert_eq!(admin_es_axis.amount, Uint128::new(1_000_000_000_000_000));
}

#[test]
fn test_cliam_reward() {
    let mut app = mock_app(init_default_balances, init_exchange_rates());

    let contracts = setup_init(&mut app, BTC_DENOM, USDC_DENOM);
    let staker = Addr::unchecked(ADMIN);
    let staking_contract = contracts.staking_contract;
    let core_contract = contracts.core_contract;
    let axis_contract = contracts.axis_contract;
    let es_axis_contract = contracts.es_axis_contract;
    let axis_res: AxisConfigResponse = app
        .wrap()
        .query_wasm_smart(axis_contract.to_owned(), &AxisQueryMsg::GetConfig {})
        .unwrap();
    let es_axis_mint_per_day = 1_000_000_000_000_000u128;
    let axis_denom = axis_res.axis_denom;

    let ex_axis_config_res: EsAxisConfigResponse = app
        .wrap()
        .query_wasm_smart(es_axis_contract.to_owned(), &EsAxisQueryMsg::GetConfig {})
        .unwrap();

    let es_axis_denom = ex_axis_config_res.es_axis_denom;

    let staker1_axis = app
        .wrap()
        .query_balance(ADMIN, axis_denom.to_owned())
        .unwrap();

    let staker2 = Addr::unchecked(TRADER1);
    let send_result = app.send_tokens(
        staker.to_owned(),
        staker2.to_owned(),
        &vec![coin(
            staker1_axis.amount.u128() / 2,
            staker1_axis.denom.to_owned(),
        )],
    );

    assert!(send_result.is_ok());

    let staker1_axis = app
        .wrap()
        .query_balance(ADMIN, axis_denom.to_owned())
        .unwrap();

    //@@ staker 1 staking
    let staker1_staking_result = staking(
        &mut app,
        &staking_contract,
        &staker,
        &vec![staker1_axis.to_owned()],
    );

    assert!(staker1_staking_result.is_ok());

    let staker2_axis = app
        .wrap()
        .query_balance(TRADER1, axis_denom.to_owned())
        .unwrap();

    //@@staker2 staking
    let staker2_staking_result = staking(
        &mut app,
        &staking_contract,
        &staker2,
        &vec![staker2_axis.to_owned()],
    );

    assert!(staker2_staking_result.is_ok());

    {
        app.update_block(|block| block.time = block.time.plus_days(1));
        //epoch is 1
        let setting_result = setting(&mut app, &core_contract, &staker);
        assert!(setting_result.is_ok());

        app.update_block(|block| block.time = block.time.plus_days(1));
        //epoch is 2
        let setting_result = setting(&mut app, &core_contract, &staker);
        assert!(setting_result.is_ok());
        let es_axis_contract_balance = app
            .wrap()
            .query_balance(es_axis_contract, es_axis_denom.to_owned())
            .unwrap();
        assert_eq!(
            es_axis_contract_balance.amount.u128(),
            (es_axis_mint_per_day)
        );
    }

    app.execute_contract(
        staker.to_owned(),
        staking_contract.to_owned(),
        &ExecuteMsg::ClaimReward {},
        &vec![],
    )
    .unwrap();

    app.execute_contract(
        staker2.to_owned(),
        staking_contract.to_owned(),
        &ExecuteMsg::ClaimReward {},
        &vec![],
    )
    .unwrap();

    //staker es-axis balance check
    let staker_ex_axis = app
        .wrap()
        .query_balance(staker.to_owned(), es_axis_denom.to_owned())
        .unwrap();

    assert_eq!(staker_ex_axis.amount.u128(), es_axis_mint_per_day / 2);

    //staker2 es-axis balance check
    let staker2_ex_axis = app
        .wrap()
        .query_balance(staker2.to_owned(), es_axis_denom)
        .unwrap();

    assert_eq!(staker2_ex_axis.amount.u128(), es_axis_mint_per_day / 2);

    //staking_amount_check
    let staker_staking_info: StakeInfoResponse = app
        .wrap()
        .query_wasm_smart(
            staking_contract.to_owned(),
            &QueryMsg::GetStakeInfo {
                addr: staker.to_string(),
            },
        )
        .unwrap();
    assert_eq!(
        staker_staking_info,
        StakeInfoResponse {
            stake_infos: vec![StakeResponse {
                start_epoch: 1,
                staking_amount: staker1_axis.amount
            }]
        }
    );

    let staker2_staking_info: StakeInfoResponse = app
        .wrap()
        .query_wasm_smart(
            staking_contract.to_owned(),
            &QueryMsg::GetStakeInfo {
                addr: staker2.to_string(),
            },
        )
        .unwrap();
    assert_eq!(
        staker2_staking_info,
        StakeInfoResponse {
            stake_infos: vec![StakeResponse {
                start_epoch: 1,
                staking_amount: staker2_axis.amount
            }]
        }
    )
}
