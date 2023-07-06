use std::str::FromStr;

use cosmwasm_std::{coin, testing::mock_dependencies, Api, Decimal, Uint128};

use crate::{
    helpers::{
        calculate_open_fee_amount, calculate_percentage, calculate_position_size,
        check::{
            check_collateral_value, check_funds_for_positions_get_funds, check_leverage_amount,
            check_leverage_rate,
        },
        fee_division, get_collateral_usd, get_leverage_amount, get_liquidation_price,
        get_trader_amount,
    },
    position::Position,
    state::Config,
};

const COLLATERAL_DECIMAL: u8 = 6;
const ENTRY_PRICE_DECIMAL: u8 = 18;
//@@Helper Test
#[test]
fn test_calculate_position_size() {
    let collateral_amount = Uint128::new(1_000_000);
    let leverage = 50;
    let position_size = calculate_position_size(collateral_amount, leverage);
    assert_eq!(position_size, Uint128::new(50_000_000));
}

#[test]
fn test_calculate_open_fee_amount() {
    let collateral_amount = Uint128::new(1_000_000);
    let leverage = 10;
    let fee_rate = 1;
    let fee_amount = calculate_open_fee_amount(collateral_amount, leverage, fee_rate);
    assert_eq!(fee_amount, Uint128::new(10_000));
}

#[test]
fn test_calculate_percentage() {
    let amount = Uint128::new(1_000);
    let three_percentage_amount = calculate_percentage(amount, 3);
    assert_eq!(three_percentage_amount, Uint128::new(30));
    let ninety_nine_percentage_amount = calculate_percentage(amount, 99);
    assert_eq!(ninety_nine_percentage_amount, Uint128::new(990));
}
pub struct LiquidationTest {
    pub entry_price: Decimal,
    pub stable_price: Decimal,
    pub leverage: u8,
    pub collateral_amount: Uint128,
    pub collateral_decimal: u8,
    pub position: Position,
    pub expect_result: Decimal,
}

pub struct LiquidationErr {
    pub entry_price: Decimal,
    pub stable_price: Decimal,
    pub leverage: u8,
    pub collateral_amount: Uint128,
    pub collateral_decimal: u8,
    pub position: Position,
}
#[test]
fn test_get_liquidation_price() {
    //Open Price +-   // Open Price * (Collateral * 0.9 +fee) / Collateral / Leverage.
    let test_pass_vec = vec![
        LiquidationTest {
            //nomal
            entry_price: Decimal::from_str("100").unwrap(), //100_000_000_000_000_000_000
            stable_price: Decimal::from_str("1").unwrap(),
            leverage: 10,
            collateral_amount: Uint128::new(100_000_000), //100
            collateral_decimal: COLLATERAL_DECIMAL,

            position: Position::Long,
            expect_result: Decimal::from_str("90.999").unwrap(),
        },
        // 0.1 - (0.1(10 *0.9 + 5)/10/50) fee is 5_000_000
        LiquidationTest {
            entry_price: Decimal::from_str("0.1").unwrap(),
            stable_price: Decimal::from_str("1").unwrap(),
            leverage: 50,
            collateral_amount: Uint128::new(100_000_000),
            collateral_decimal: COLLATERAL_DECIMAL,
            position: Position::Long,
            expect_result: Decimal::from_str("0.0972").unwrap(), //0.0981
        },
    ];
    for test in test_pass_vec.into_iter() {
        let LiquidationTest {
            entry_price,
            leverage,
            collateral_amount,
            collateral_decimal,
            position,
            expect_result,
            ..
        } = test;
        let fee_rate = 1;
        //fee is position size / 0.001
        // println!("entry_price = {:?}", entry_price);

        let open_fee_amount = calculate_open_fee_amount(collateral_amount, leverage, fee_rate);
        // println!("open_fee_amount {:?}", open_fee_amount);
        let liquidation_price = get_liquidation_price(
            entry_price,
            entry_price,
            collateral_amount,
            collateral_decimal,
            open_fee_amount,
            leverage,
            &position,
        )
        .unwrap();

        assert_eq!(liquidation_price, expect_result)
    }
    let test_err_vec = vec![LiquidationErr {
        //minimum_value
        entry_price: Decimal::from_str("100").unwrap(),
        stable_price: Decimal::from_str("1").unwrap(),
        leverage: 10,
        collateral_amount: Uint128::new(100_00), //0.1
        collateral_decimal: COLLATERAL_DECIMAL,
        position: Position::Long,
    }];
    for test in test_err_vec.into_iter() {
        let LiquidationErr {
            entry_price,
            leverage,
            collateral_amount,
            collateral_decimal,
            position,
            ..
        } = test;
        let fee_rate = 1;
        let open_fee_amount = calculate_open_fee_amount(collateral_amount, leverage, fee_rate);
        let result = get_liquidation_price(
            entry_price,
            entry_price,
            collateral_amount,
            collateral_decimal,
            open_fee_amount,
            leverage,
            &position,
        );

        assert!(result.is_err());
    }
}

pub struct TestGetCollateral {
    pub collateral_amount: Uint128,
    pub collateral_decimal: u8,
    pub collateral_price: Decimal,
    pub expect: Decimal,
}
#[test]
fn test_get_collateral_usd() {
    let test_vec = vec![
        //nomal test
        TestGetCollateral {
            collateral_amount: Uint128::new(1_000_000),
            collateral_decimal: COLLATERAL_DECIMAL,
            collateral_price: Decimal::from_str("1").unwrap(),
            expect: Decimal::from_str("1").unwrap(),
        },
        TestGetCollateral {
            collateral_amount: Uint128::new(100_000_000),
            collateral_decimal: COLLATERAL_DECIMAL,
            collateral_price: Decimal::from_str("0.1").unwrap(),
            expect: Decimal::from_str("10").unwrap(),
        },
    ];
    for test in test_vec.into_iter() {
        let TestGetCollateral {
            collateral_amount,
            collateral_decimal,
            collateral_price,
            expect,
        } = test;

        let result =
            get_collateral_usd(collateral_amount, collateral_decimal, collateral_price).unwrap();
        assert_eq!(result, expect)
    }
}

pub struct GetLeverageTest {
    pub collateral_amount: Uint128,
    pub leverage: u8,
    pub expect: Uint128,
}
#[test]
fn test_get_leverage_amount() {
    let test = GetLeverageTest {
        collateral_amount: Uint128::new(10_000_000),
        leverage: 10,
        expect: Uint128::new(100_000_000),
    };
    let result = get_leverage_amount(test.collateral_amount, test.leverage).unwrap();
    assert_eq!(result, test.expect);
    //overflow test
    let test = GetLeverageTest {
        collateral_amount: Uint128::new(340282366920938463463374607431768211455),
        leverage: 50,
        expect: Uint128::new(100_000_000),
    };
    let result = get_leverage_amount(test.collateral_amount, test.leverage);
    assert!(result.is_err())
}

pub struct GetTraderValueAmount {
    pub trader_position: Position,
    pub winning_position: Position,
    pub entry_price: Uint128,
    pub current_price: Uint128,
    pub collateral_amount: Uint128,
    pub collateral_decimal: u8,
    pub collateral_price: Decimal,
    pub leverage: u8,
    pub expect: Uint128,
}
#[test]
fn test_get_trader_amount() {
    let test_vec = vec![
        //profit = 20
        //price = 10
        //2 개
        GetTraderValueAmount {
            trader_position: Position::Long,
            winning_position: Position::Long,
            entry_price: Uint128::new(1_000_000_000_000_000_000), //1
            current_price: Uint128::new(2_000_000_000_000_000_000), //2
            collateral_amount: Uint128::new(1_000_000),
            collateral_decimal: COLLATERAL_DECIMAL,
            collateral_price: Decimal::from_str("10").unwrap(),
            leverage: 10,
            expect: Uint128::new(2_000_000),
        },
        GetTraderValueAmount {
            //profit = 5
            trader_position: Position::Short,
            winning_position: Position::Short,
            entry_price: Uint128::new(1_000_000_000_000_000_000), //1
            current_price: Uint128::new(500_000_000_000_000_000), //0.5
            collateral_amount: Uint128::new(1_000_000),           //$100
            collateral_decimal: COLLATERAL_DECIMAL,
            collateral_price: Decimal::from_str("10").unwrap(), //$10
            leverage: 10,
            expect: Uint128::new(1_500_000),
        },
        GetTraderValueAmount {
            //loss test
            //loss = 5
            trader_position: Position::Long,
            winning_position: Position::Short,
            entry_price: Uint128::new(1_000_000_000_000_000_000), //1
            current_price: Uint128::new(500_000_000_000_000_000), //0.5
            collateral_amount: Uint128::new(1_000_000),           //$100
            collateral_decimal: COLLATERAL_DECIMAL,
            collateral_price: Decimal::from_str("10").unwrap(), //$10
            leverage: 10,
            expect: Uint128::new(500_000), //$5
        },
        GetTraderValueAmount {
            //loss test
            //loss = 5
            trader_position: Position::Short,
            winning_position: Position::Long,
            entry_price: Uint128::new(500_000_000_000_000_000), //0.5
            current_price: Uint128::new(1_000_000_000_000_000_000), //1
            collateral_amount: Uint128::new(1_000_000),         //$100
            collateral_decimal: COLLATERAL_DECIMAL,
            collateral_price: Decimal::from_str("10").unwrap(), //$10
            leverage: 10,
            expect: Uint128::new(500_000), //$5
        },
    ];

    for test in test_vec.into_iter() {
        let GetTraderValueAmount {
            trader_position,
            winning_position,
            entry_price,
            current_price,
            collateral_amount,
            collateral_decimal,
            collateral_price,
            leverage,
            expect,
        } = test;
        let result = get_trader_amount(
            &trader_position,
            &winning_position,
            entry_price,
            current_price,
            collateral_amount,
            collateral_decimal,
            collateral_price,
            leverage,
        )
        .unwrap();

        assert_eq!(result, expect);
    }
}
#[test]
fn test_fee_division() {
    let deps = mock_dependencies();
    let mut mock_config = Config {
        owner: deps.api.addr_validate("owner").unwrap(),
        asset_denom: "ubtc".to_string(),
        stable_denom: "uusdc".to_string(),
        asset_decimal: 6,
        stable_decimal: 6,
        max_leverage: 50,
        pool_contract: deps.api.addr_validate("pool_contract").unwrap(),
        fee_vault_contract: deps.api.addr_validate("fee_valut_contract").unwrap(),
        treasury_contract: deps.api.addr_validate("treasury_contract").unwrap(),
        asset_total_fee: Uint128::new(100_000_000),
        stable_total_fee: Uint128::new(100_000_000),
        past_price: Decimal::from_str("100").unwrap(),
    };
    let (
        send_asset_fee_to_pool,
        send_stable_fee_to_pool,
        send_asset_fee_to_valut,
        send_stable_fee_to_valut,
    ) = fee_division(&mut mock_config);
    assert_eq!(send_asset_fee_to_pool, Uint128::new(70_000_000));
    assert_eq!(send_stable_fee_to_pool, Uint128::new(70_000_000));
    assert_eq!(send_asset_fee_to_valut, Uint128::new(30_000_000));
    assert_eq!(send_stable_fee_to_valut, Uint128::new(30_000_000));

    let mut mock_config = Config {
        owner: deps.api.addr_validate("owner").unwrap(),
        asset_denom: "ubtc".to_string(),
        stable_denom: "uusdc".to_string(),
        asset_decimal: 6,
        stable_decimal: 6,
        max_leverage: 50,
        pool_contract: deps.api.addr_validate("pool_contract").unwrap(),
        fee_vault_contract: deps.api.addr_validate("fee_valut_contract").unwrap(),
        treasury_contract: deps.api.addr_validate("treasury_contract").unwrap(),
        asset_total_fee: Uint128::new(10),
        stable_total_fee: Uint128::new(10),
        past_price: Decimal::from_str("100").unwrap(),
    };
    let (
        send_asset_fee_to_pool,
        send_stable_fee_to_pool,
        send_asset_fee_to_valut,
        send_stable_fee_to_valut,
    ) = fee_division(&mut mock_config);
    assert_eq!(send_asset_fee_to_pool, Uint128::new(7));
    assert_eq!(send_stable_fee_to_pool, Uint128::new(7));
    assert_eq!(send_asset_fee_to_valut, Uint128::new(3));
    assert_eq!(send_stable_fee_to_valut, Uint128::new(3));
}

//@@Implement this function when exchange_rates can be updated.
// #[test]
// fn test_control_desitinated_traders() {
// let mut deps = mock_init("ubtc", 6, "uusdc", 6, 50, 1, 1, 2, "owner");

// let trader1 = deps.api.addr_validate("trader1").unwrap();
// let trade1 = make_trade(
//     trader1.clone(),
//     "1",
//     "ubtc",
//     100_000_000,
//     "1",
//     6,
//     10,
//     1,
//     Position::Long,
//     None,
//     None,
// );

// let result = trade_save(deps.as_mut().storage, trader1, trade1);
// assert!(result.is_ok());

// let trader2 = deps.api.addr_validate("trader2").unwrap();
// let trade2 = make_trade(
//     trader2.clone(),
//     "1",
//     "ubtc",
//     100_000_000,
//     "1",
//     6,
//     50,
//     1,
//     Position::Long,
//     Some(Uint128::new(500_000_000_000_000_000)),
//     Some(Uint128::new(2_000_000_000_000_000_000)),
// );
// let result = trade_save(deps.as_mut().storage, trader2, trade2.clone());
// assert!(result.is_ok());

// let trader3 = deps.api.addr_validate("trader3").unwrap();
// let trade3 = make_trade(
//     trader3.clone(),
//     "1",
//     "uusdc",
//     100_000_000,
//     "1",
//     6,
//     10,
//     1,
//     Position::Short,
//     None,
//     None,
// );
// let result = trade_save(deps.as_mut().storage, trader3, trade3.clone());
// assert!(result.is_ok());

// let trader4 = deps.api.addr_validate("trader4").unwrap();
// let trade4 = make_trade(
//     trader4.clone(),
//     "1",
//     "uusdc",
//     100_000_000,
//     "1",
//     6,
//     10,
//     1,
//     Position::Short,
//     Some(Uint128::new(1_000_000_500_000_000_000)),
//     None,
// );
// let result = trade_save(deps.as_mut().storage, trader4, trade4.clone());
// assert!(result.is_ok());

// let before_price = Uint128::new(1_000_000_000_000_000_000);
// let now_price = Uint128::new(3_000_000_000_000_000_000);
// let price_destinated_trader =
//     get_desitinated_price_traders(deps.as_mut().storage, before_price, now_price).unwrap();

// assert_eq!(
//     price_destinated_trader,
//     PriceDestinatedTrader {
//         limit_loss: PriceDestinatedStatus::LimitLoss(vec![trade4]),
//         limit_profit: PriceDestinatedStatus::LimitProfit(vec![trade2]),
//         liquidated: PriceDestinatedStatus::Liquidated(vec![trade3]),
//     }
// )
// }

//@@ Check
#[test]
fn test_check_leverage_amount() {
    let pool_balance = Uint128::new(1000);
    let leverage_amount = Uint128::new(100);
    let result = check_leverage_amount(pool_balance, leverage_amount);
    assert!(result.is_ok());
    let leverage_amount = Uint128::new(101);
    let result = check_leverage_amount(pool_balance, leverage_amount);
    assert!(result.is_err());
}

#[test]
fn test_check_leverage_rate() {
    let leverage = 10;
    let max_leverage = 50;
    let result = check_leverage_rate(leverage, max_leverage);
    assert!(result.is_ok());
    let leverage = 55;
    let result = check_leverage_rate(leverage, max_leverage);
    assert!(result.is_err())
}

#[test]
fn test_check_funds_for_position_get_funds() {
    let deps = mock_dependencies();
    let mock_config = Config {
        owner: deps.api.addr_validate("owner").unwrap(),
        asset_denom: "ubtc".to_string(),
        stable_denom: "uusdc".to_string(),
        asset_decimal: 6,
        stable_decimal: 6,
        max_leverage: 50,
        pool_contract: deps.api.addr_validate("pool_contract").unwrap(),
        fee_vault_contract: deps.api.addr_validate("fee_valut_contract").unwrap(),
        treasury_contract: deps.api.addr_validate("treasury_contract").unwrap(),
        asset_total_fee: Uint128::default(),
        stable_total_fee: Uint128::default(),
        past_price: Decimal::from_str("100").unwrap(),
    };

    let funds = vec![coin(100_000_000, "ubtc")];
    let position = &Position::Long;
    let result = check_funds_for_positions_get_funds(funds, &mock_config, position);
    assert!(result.is_ok());

    let funds = vec![coin(100_000_000, "ueth")];
    let position = &Position::Long;
    let result = check_funds_for_positions_get_funds(funds, &mock_config, position);
    assert!(result.is_err());

    let funds = vec![];
    let position = &Position::Long;
    let result = check_funds_for_positions_get_funds(funds, &mock_config, position);
    assert!(result.is_err());
}

#[test]
fn test_check_collateral_value() {
    let collateral_usd = Decimal::from_str("8").unwrap();
    let minimum_usd = 10;
    let result = check_collateral_value(collateral_usd, minimum_usd);
    assert!(result.is_err());
    let collateral_usd = Decimal::from_str("11").unwrap();
    let result = check_collateral_value(collateral_usd, minimum_usd);
    assert!(result.is_ok());
}

// pub fn mock_init(
//     asset_denom: &str,
//     asset_decimal: u8,
//     stable_denom: &str,
//     stable_decimal: u8,
//     max_leverage: u8,
//     borrow_fee_rate: u8,
//     open_close_fee_rate: u8,
//     limit_profit_loss_open_fee_rate: u8,

//     owner: &str,
// ) -> OwnedDeps<MemoryStorage, MockApi, MockQuerier, SeiQueryWrapper> {
//     let mut deps = custom_mock_dependencies();

//     let msg = InstantiateMsg {
//         asset_denom: asset_denom.to_string(),
//         asset_decimal,
//         stable_denom: stable_denom.to_string(),
//         stable_decimal,
//         max_leverage,
//         borrow_fee_rate,
//         open_close_fee_rate,
//         pool_contract: "pool_contract".to_string(),
//         fee_vault_contract: "fee_vault_contract".to_string(),
//         limit_profit_loss_open_fee_rate,
//     };
//     let info = mock_info(owner, &[]);

//     let res = instantiate(deps.as_mut(), mock_env(), info, msg);
//     match res {
//         Ok(_) => deps,
//         Err(_) => panic!("init Fail"),
//     }
// }
// pub fn make_trade(
//     trader: Addr,
//     entry_price: &str,
//     collateral_denom: &str,
//     collateral_amount: u128,
//     collateral_price: &str,
//     collateral_decimal: u8,
//     leverage: u8,
//     fee_rate: u8,
//     position: Position,
//     limit_loss_price: Option<Uint128>,
//     limit_profit_price: Option<Uint128>,
// ) -> Trade {
//     let entry_price = Decimal::from_str(entry_price).unwrap();
//     let collateral_price = Decimal::from_str(collateral_price).unwrap();
//     let mut collateral_amount = Uint128::new(collateral_amount);
//     let open_fee_amount = calculate_open_fee_amount(collateral_amount, leverage, fee_rate);
//     collateral_amount -= open_fee_amount;
//     let position_size = calculate_position_size(collateral_amount, leverage);
//     let leverage_amount = get_leverage_amount(collateral_amount, leverage).unwrap();
//     let liquidation_price = get_liquidation_price(
//         entry_price,
//         collateral_price,
//         collateral_amount,
//         collateral_decimal,
//         open_fee_amount,
//         leverage,
//         &position,
//     )
//     .unwrap()
//     .atomics();
//     Trade::new(
//         trader,
//         entry_price.atomics(),
//         liquidation_price,
//         limit_profit_price,
//         limit_loss_price,
//         collateral_denom.to_string(),
//         collateral_amount,
//         position,
//         position_size,
//         leverage,
//         leverage_amount,
//     )
// }
