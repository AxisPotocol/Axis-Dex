use cosmwasm_schema::cw_serde;

use cosmwasm_std::{Addr, StdResult, Storage, Uint128};

use cw_storage_plus::{Item, Map};

#[cw_serde]
pub struct Config {
    pub core_contract: Addr,
    pub axis_denom: String,
    pub pending_total_fee: Uint128,
    pub mint_per_epoch_trader_amount: Uint128,
    pub mint_per_epoch_maker_amount: Uint128,
}
pub fn save_config(storage: &mut dyn Storage, config: &Config) -> StdResult<()> {
    CONFIG.save(storage, config)
}
pub fn load_config(storage: &dyn Storage) -> StdResult<Config> {
    CONFIG.load(storage)
}
#[cw_serde]
pub struct Treasury {
    pub epoch: u64,
    pub fee_amount: Uint128,
}

pub fn update_trader(
    storage: &mut dyn Storage,
    trader: &Addr,
    amount: Uint128,
    epoch: u64,
) -> StdResult<()> {
    TRADER.update(storage, trader, |exsists| -> StdResult<Vec<Treasury>> {
        match exsists {
            Some(mut traders) => {
                if let Some(trader) = traders.iter_mut().find(|t| t.epoch == epoch) {
                    trader.fee_amount += amount;
                } else {
                    let new_trader = Treasury {
                        epoch,
                        fee_amount: amount,
                    };
                    traders.push(new_trader);
                }
                Ok(traders)
            }
            None => {
                let traders = vec![Treasury {
                    epoch,
                    fee_amount: amount,
                }];

                Ok(traders)
            }
        }
    })?;
    Ok(())
}
pub fn load_trader(storage: &mut dyn Storage, minter: &Addr) -> StdResult<Vec<Treasury>> {
    TRADER.load(storage, minter)
}

pub fn update_pool_fee(
    storage: &mut dyn Storage,
    pool_key: &str,
    amount: Uint128,
) -> StdResult<()> {
    POOL_FEE.update(storage, pool_key, |exsists| -> StdResult<Uint128> {
        match exsists {
            Some(total_amount) => Ok(total_amount + amount),
            None => Ok(amount),
        }
    })?;
    Ok(())
}
pub const TRADER: Map<&Addr, Vec<Treasury>> = Map::new("trader");

//pool key is ubtc:usdc, epoch

//pool 의 epoch 마다의 기여한 fee 양
// pub const POOL_FEE: Map<(u64, &str), Uint128> = Map::new("pool");
pub const POOL_FEE: Map<&str, Uint128> = Map::new("pool_fee");
//pool 이 가져갈 수 있는 민팅양
pub const POOL_MINT_AMOUNT: Map<(&str, u64), Uint128> = Map::new("pool_mint_amount");
pub const EPOCH_TOTAL_FEE_AMOUNT: Map<u64, Uint128> = Map::new("total_fee_amount");
pub const CONFIG: Item<Config> = Item::new("config");
