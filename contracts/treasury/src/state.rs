use cosmwasm_schema::cw_serde;

use cosmwasm_std::{Addr, StdResult, Storage, Uint128};

use cw_storage_plus::{Item, Map};

#[cw_serde]
pub struct State {
    pub owner: Addr,
    pub denom: String,
    pub epoch: u64,
    pub last_update_timestamp: u64,
    pub total_fee: Uint128,
    pub core_contract: Addr,
    pub total_supply: Uint128,
    pub mint_amount_per_epoch: Uint128,
    // pub is_airdrop: bool,
}
pub fn save_state(storage: &mut dyn Storage, state: &State) -> StdResult<()> {
    STATE.save(storage, state)
}
pub fn load_state(storage: &mut dyn Storage) -> StdResult<State> {
    STATE.load(storage)
}
#[cw_serde]
pub struct Treasury {
    pub epoch: u64,
    pub fee_amount: Uint128,
}

pub fn update_treasury(
    storage: &mut dyn Storage,
    trader: String,
    amount: Uint128,
    state: &State,
) -> StdResult<()> {
    TREASURY.update(
        storage,
        Addr::unchecked(trader),
        |exsists| -> StdResult<Vec<Treasury>> {
            match exsists {
                Some(mut treasurys) => {
                    treasurys
                        .iter_mut()
                        .find(|t| t.epoch == state.epoch)
                        .map(|t| t.fee_amount += amount);
                    Ok(treasurys)
                }
                None => {
                    let treasurys = vec![Treasury {
                        epoch: state.epoch,
                        fee_amount: amount,
                    }];

                    Ok(treasurys)
                }
            }
        },
    )?;
    Ok(())
}
pub fn load_treasurys(storage: &mut dyn Storage, minter: Addr) -> StdResult<Vec<Treasury>> {
    TREASURY.load(storage, minter)
}

pub const TREASURY: Map<Addr, Vec<Treasury>> = Map::new("treasury");
pub const TOTAL_FEE_AMOUNT: Map<u64, Uint128> = Map::new("total_fee_amount");
pub const STATE: Item<State> = Item::new("state");
