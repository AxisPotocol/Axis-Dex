use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Order, StdResult, Storage, Uint128};
use cw_storage_plus::{Index, IndexList, IndexedMap, MultiIndex, PrefixBound};

use crate::{error::ContractError, position::Position};

pub enum IndexType {
    Loss,
    Profit,
    Liquidated,
}

#[cw_serde]
pub struct PriceDestinatedTrader {
    pub limit_loss: PriceDestinatedStatus,
    pub limit_profit: PriceDestinatedStatus,
    pub liquidated: PriceDestinatedStatus,
}
#[cw_serde]
pub enum PriceDestinatedStatus {
    LimitLoss(Vec<Trade>),
    Liquidated(Vec<Trade>),
    LimitProfit(Vec<Trade>),
}
#[cw_serde]
pub struct Trade {
    //user
    pub trader: Addr,
    //거래 시점 가격
    pub entry_price: Uint128,
    //청산 가격
    pub liquidation_price: Uint128,
    //no limit is Uint128::MAX
    pub limit_profit_price: Uint128,
    pub limit_loss_price: Uint128,

    //증거금 종류
    pub collateral_denom: String,
    //증거금 양 fee 공제 금액
    pub collateral_amount: Uint128,
    //포지션 Long or Short
    pub position: Position,
    //포지션 사이즈 = 증거금 * 레버리지 비율
    pub position_size: Uint128,
    //레버리지 비율
    pub leverage: u8,
    //레버리지한 금액
    pub leverage_amount: Uint128,
}

impl Trade {
    pub fn new(
        trader: Addr,
        entry_price: Uint128,
        liquidation_price: Uint128,
        limit_profit_price: Option<Uint128>,
        limit_loss_price: Option<Uint128>,
        collateral_denom: String,
        collateral_amount: Uint128,
        position: Position,
        position_size: Uint128,

        leverage: u8,
        leverage_amount: Uint128,
    ) -> Self {
        //indexed map index key
        let limit_loss_price = match limit_loss_price {
            Some(price) => price,
            None => Uint128::MAX,
        };
        let limit_profit_price = match limit_profit_price {
            Some(price) => price,
            None => Uint128::MAX,
        };
        Self {
            trader,
            entry_price,
            liquidation_price,
            limit_profit_price,
            limit_loss_price,
            collateral_denom,
            collateral_amount,
            position,
            position_size,

            leverage,
            leverage_amount,
        }
        //stop loss option 처리
    }
}

pub struct TradeIndexes<'a> {
    pub liquidation_price: MultiIndex<'a, u128, Trade, Addr>,
    pub limit_profit_price: MultiIndex<'a, u128, Trade, Addr>,
    pub limit_loss_price: MultiIndex<'a, u128, Trade, Addr>,
}

impl<'a> IndexList<Trade> for TradeIndexes<'a> {
    fn get_indexes(&'_ self) -> Box<dyn Iterator<Item = &'_ dyn Index<Trade>> + '_> {
        let v: Vec<&dyn Index<Trade>> = vec![
            &self.liquidation_price,
            &self.limit_profit_price,
            &self.limit_loss_price,
        ];
        Box::new(v.into_iter())
    }
}

pub fn trades<'a>() -> IndexedMap<'a, Addr, Trade, TradeIndexes<'a>> {
    let indexes = TradeIndexes {
        liquidation_price: MultiIndex::new(
            |_pk, d: &Trade| d.liquidation_price.u128(),
            "trade_trader",
            "trade_liquidation_price",
        ),
        limit_profit_price: MultiIndex::new(
            |_pk, d: &Trade| d.limit_profit_price.u128(),
            "trade_trader",
            "trade_limit_profit_price",
        ),
        limit_loss_price: MultiIndex::new(
            |_pk, d: &Trade| d.limit_loss_price.u128(),
            "trade_trader",
            "trade_limit_loss_price",
        ),
    };
    IndexedMap::new("trade_trader", indexes)
}

pub fn trade_update(
    storage: &mut dyn Storage,
    trader: Addr,
    trade: Trade,
) -> Result<(), ContractError> {
    trades().update(storage, trader, |t| match t {
        Some(_) => Err(ContractError::TraderOnlyOnePosition {}),
        None => Ok(trade),
    })?;
    Ok(())
}
pub fn trade_save(storage: &mut dyn Storage, trader: Addr, trade: Trade) -> StdResult<()> {
    trades().save(storage, trader, &trade)
}
pub fn trade_remove(storage: &mut dyn Storage, trader: Addr) -> StdResult<()> {
    trades().remove(storage, trader)
}

pub fn trade_load(storage: &mut dyn Storage, trader: Addr) -> StdResult<Trade> {
    trades().load(storage, trader)
}

fn get_traders_in_price_range(
    storage: &mut dyn Storage,
    min: Uint128,
    max: Uint128,
    index: IndexType,
) -> Result<Vec<Trade>, ContractError> {
    let trades_idx = match index {
        IndexType::Loss => trades().idx.limit_loss_price,
        IndexType::Profit => trades().idx.limit_profit_price,
        IndexType::Liquidated => trades().idx.liquidation_price,
    };

    trades_idx
        .prefix_range_raw(
            storage,
            Some(PrefixBound::inclusive(min)),
            Some(PrefixBound::inclusive(max)),
            Order::Ascending,
        )
        .collect::<Result<Vec<(_, Trade)>, _>>()
        .and_then(|result| result.into_iter().map(|(_, trade)| Ok(trade)).collect())
        .map_err(|_| ContractError::ParseError {})
}
pub fn get_desitinated_price_traders(
    storage: &mut dyn Storage,
    before_price: Uint128,
    now_price: Uint128,
) -> Result<PriceDestinatedTrader, ContractError> {
    let (min, max) = match before_price.ge(&now_price) {
        true => (now_price, before_price),
        false => (before_price, now_price),
    };

    let limit_loss = get_traders_in_price_range(storage, min, max, IndexType::Loss)?;
    for trade in limit_loss.iter() {
        trade_remove(storage, trade.trader.clone())?;
    }

    let limit_profit = get_traders_in_price_range(storage, min, max, IndexType::Profit)?;
    for trade in limit_profit.iter() {
        trade_remove(storage, trade.trader.clone())?;
    }

    let liquidated = get_traders_in_price_range(storage, min, max, IndexType::Liquidated)?;
    for trade in limit_profit.iter() {
        trade_remove(storage, trade.trader.clone())?;
    }
    Ok(PriceDestinatedTrader {
        limit_loss: PriceDestinatedStatus::LimitLoss(limit_loss),
        limit_profit: PriceDestinatedStatus::LimitProfit(limit_profit),
        liquidated: PriceDestinatedStatus::Liquidated(liquidated),
    })
}
