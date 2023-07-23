use axis_protocol::query::query_epoch;
#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    coin, to_binary, BankMsg, Binary, Decimal, Deps, DepsMut, Env, MessageInfo, Response,
    StdResult, SubMsg, Uint128,
};
use cw2::set_contract_version;
use sei_cosmwasm::{SeiMsg, SeiQueryWrapper};

use crate::error::ContractError;
use crate::state::{save_state, Config, State, CONFIG};
use axis_protocol::axis::{ExecuteMsg, InstantiateMsg, QueryMsg};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:gmx";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const MAX_TOTAL_SUPPLY: u128 = 1_000_000_000_000_000;
const AXIS_DENOM: &str = "AXIS";
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut<SeiQueryWrapper>,
    env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response<SeiMsg>, ContractError> {
    //@@Foundation amount
    let owner_mint_amount = Uint128::new(MAX_TOTAL_SUPPLY) * Decimal::percent(20);
    //@@Community amount
    // let community_mint_amount = Uint128::new(MAX_TOTAL_SUPPLY) * Decimal::percent(20);
    let axis_denom =
        "factory/".to_string() + env.contract.address.to_string().as_ref() + "/" + AXIS_DENOM;

    let config = Config {
        axis_denom: axis_denom.clone(),
        core_contract: info.sender.clone(),
        mint_per_epoch_trader_amount: (Uint128::new(300_000_000_000_000) / Uint128::new(365 * 5)),
        mint_per_epoch_maker_amount: (Uint128::new(300_000_000_000_000) / Uint128::new(365 * 5)),
    };
    CONFIG.save(deps.storage, &config)?;
    let epoch = query_epoch(deps.querier, &info.sender)?;
    let state = State {
        pending_total_fee_usd: Uint128::zero(),
        epoch,
    };
    save_state(deps.storage, &state)?;

    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let axis_create_msg = SeiMsg::CreateDenom {
        subdenom: AXIS_DENOM.to_owned(),
    };
    let mint_token = coin(owner_mint_amount.into(), axis_denom);
    let axis_mint_msg = SeiMsg::MintTokens {
        amount: mint_token.to_owned(),
    };
    let owner_send_msg = SubMsg::new(BankMsg::Send {
        to_address: msg.owner.to_string(),
        amount: vec![mint_token],
    });

    // let community_token = coin(community_mint_amount.into(), DENOM.to_owned());
    // // @@ Wasm 으로 컨트랙트 만들고 돈보내주는게 좋은데...
    // let community_send_msg = SubMsg::new(BankMsg::Send {
    //     to_address: msg.community_contract.to_string(),
    //     amount: vec![community_token],
    // });
    // let community_msg = SubMsg::new(WasmMsg::Instantiate {
    //     admin: Some(env.contract.address.to_string()),
    //     code_id: 1,
    //     msg: &msg.community_init_msg,
    //     funds: vec![community_token],
    //     label: format!("{:?} community_airdrop", DENOM),
    // });
    //airdrop contract code 를 받아서 instantiate 해주고 reply 로 보내주자.

    Ok(
        Response::new()
            .add_attribute("method", "instantiate")
            .add_attribute("owner", info.sender)
            .add_message(axis_create_msg)
            .add_message(axis_mint_msg)
            .add_submessage(owner_send_msg), // .add_submessage(community_send_msg)
    )
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut<SeiQueryWrapper>,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<SeiMsg>, ContractError> {
    use execute::*;
    match msg {
        ExecuteMsg::AddFeeAmount {
            base_denom,
            price_denom,
            trader,
            fee_usd_amount,
        } => add_fee_amount(deps, info, base_denom, price_denom, trader, fee_usd_amount),
        //ClaimMintTrader
        ExecuteMsg::ClaimMintTrader {} => claim_minting_trader(deps, info),
        //ClaimMintMaker
        ExecuteMsg::ClaimMintMaker {
            base_denom,
            price_denom,
            sender,
            amount,
        } => claim_minting_maker(deps, info, base_denom, price_denom, sender, amount),
        ExecuteMsg::Setting { epoch } => setting(deps, info, epoch),
        // ExecuteMsg::RegisterAirDrop { air_drop_contract } => {
        //     execute::register_airdrop(deps, info, air_drop_contract)
        // }
    }
}

pub mod execute {
    use cosmwasm_std::{coin, Addr, BankMsg, Decimal, Order, SubMsg};

    use sei_cosmwasm::SeiQueryWrapper;

    use crate::{
        helpers::{check_core_contract, check_lp_staking_contract, check_market_contract},
        query::{query_pair_lp_staking_contract, query_pair_market_contract},
        state::{
            load_config, load_state, load_trader, update_pool_fee, update_trader, TraderTreasury,
            EPOCH_TOTAL_FEE_AMOUNT, POOL_FEE, POOL_MINT_AMOUNT, STATE, TRADER,
        },
    };

    use super::*;
    pub fn add_fee_amount(
        deps: DepsMut<SeiQueryWrapper>,
        info: MessageInfo,
        base_denom: String,
        price_denom: String,
        trader: Addr,
        amount: Uint128,
    ) -> Result<Response<SeiMsg>, ContractError> {
        let config = load_config(deps.storage)?;
        let mut state = load_state(deps.storage)?;
        // @@ 굳이 market_contract 여야만함? 굳이굳이?????????????
        let epoch = state.epoch;

        let market_contract = query_pair_market_contract(
            deps.querier,
            &config.core_contract,
            &base_denom,
            &price_denom,
        )?;
        check_market_contract(&market_contract, &info.sender)?;
        update_trader(deps.storage, &trader, amount, epoch)?;
        // Update Maker

        update_pool_fee(
            deps.storage,
            &format!("{}:{}", base_denom, price_denom),
            amount,
        )?;
        state.pending_total_fee_usd += amount;
        save_state(deps.storage, &state)?;

        Ok(Response::new())
    }

    pub fn claim_minting_trader(
        deps: DepsMut<SeiQueryWrapper>,
        info: MessageInfo,
    ) -> Result<Response<SeiMsg>, ContractError> {
        let config = load_config(deps.storage)?;
        let traders = load_trader(deps.storage, &info.sender)?;
        let state = load_state(deps.storage)?;
        let mint_amount: Uint128 = traders
            .iter()
            .filter(|treasury| treasury.epoch < state.epoch)
            .map(|trader| {
                let total_fee_amount = EPOCH_TOTAL_FEE_AMOUNT.load(deps.storage, trader.epoch)?;
                let ratio = Decimal::from_ratio(trader.fee_amount, total_fee_amount);
                Ok(config.mint_per_epoch_trader_amount * ratio)
            })
            .sum::<Result<Uint128, ContractError>>()?;

        let remain_treasury = traders
            .into_iter()
            .filter(|treausry| treausry.epoch == state.epoch)
            .collect::<Vec<TraderTreasury>>();

        TRADER.save(deps.storage, &info.sender, &remain_treasury)?;

        let token = coin(mint_amount.into(), config.axis_denom);

        let mint_msg = SeiMsg::MintTokens {
            amount: token.to_owned(),
        };

        let send_msg = SubMsg::new(BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: vec![token],
        });

        Ok(Response::new()
            .add_message(mint_msg)
            .add_submessage(send_msg))
    }
    pub fn claim_minting_maker(
        deps: DepsMut<SeiQueryWrapper>,
        info: MessageInfo,
        base_denom: String,
        price_denom: String,
        sender: Addr,
        amount: Uint128,
    ) -> Result<Response<SeiMsg>, ContractError> {
        //pool에 얼마나 민팅되어있는지 확인,epoch 확인
        //lp staking contract 어디서 관리할거임?
        //core? or pool?
        let config = load_config(deps.storage)?;

        let pair_lp_staking_contract = query_pair_lp_staking_contract(
            deps.querier,
            &config.core_contract,
            &base_denom,
            &price_denom,
        )?;

        check_lp_staking_contract(&pair_lp_staking_contract, &info.sender)?;
        let axis_token = coin(amount.into(), config.axis_denom);
        let mint_msg = SeiMsg::MintTokens {
            amount: axis_token.to_owned(),
        };
        let send_msg = SubMsg::new(BankMsg::Send {
            to_address: sender.to_string(),
            amount: vec![axis_token],
        });

        Ok(Response::new()
            .add_message(mint_msg)
            .add_submessage(send_msg))
    }
    pub fn setting(
        deps: DepsMut<SeiQueryWrapper>,
        info: MessageInfo,
        epoch: u64,
    ) -> Result<Response<SeiMsg>, ContractError> {
        let config = load_config(deps.storage)?;
        check_core_contract(&config.core_contract, &info.sender)?;

        let mut state = load_state(deps.storage)?;

        EPOCH_TOTAL_FEE_AMOUNT.save(deps.storage, state.epoch, &state.pending_total_fee_usd)?;

        let pairs = POOL_FEE
            .range(deps.storage, None, None, Order::Ascending)
            .collect::<StdResult<Vec<(String, Uint128)>>>()?;

        for (pair, pool_epoch_fee) in pairs {
            let ratio = Decimal::from_ratio(pool_epoch_fee, state.pending_total_fee_usd);

            let mint_amount = ratio * config.mint_per_epoch_maker_amount; // Compute the mint amount based on the ratio

            POOL_MINT_AMOUNT.save(deps.storage, (&pair, epoch), &mint_amount)?;
        }

        POOL_FEE.clear(deps.storage);

        state.pending_total_fee_usd = Uint128::zero();
        state.epoch = epoch;
        save_state(deps.storage, &state)?;

        Ok(Response::new())
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps<SeiQueryWrapper>, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetConfig {} => to_binary(&query::get_config(deps)?),
        QueryMsg::GetPendingTotalFee {} => to_binary(&query::get_pending_fee(deps)?),
        QueryMsg::GetPoolAllowedMintAmount {
            base_denom,
            price_denom,
            start_epoch,
            end_epoch,
        } => to_binary(&query::get_pool_allowed_mint_amount(
            deps,
            base_denom,
            price_denom,
            start_epoch,
            end_epoch,
        )?),
        QueryMsg::GetTotalSupply {} => to_binary(&query::get_total_supply(deps)?),
        QueryMsg::GetEpochTotalFeeAmount {
            start_epoch,
            end_epoch,
        } => to_binary(&query::get_epoch_total_fee(deps, start_epoch, end_epoch)?),
    }
}

pub mod query {

    use super::*;
    use crate::state::{load_config, load_state, EPOCH_TOTAL_FEE_AMOUNT, POOL_MINT_AMOUNT};
    use axis_protocol::axis::{
        ConfigResponse, EpochTotalFeeAmountResponse, PendingFeeResponse,
        PoolAllowedMintAmountResponse, TotalSupplyResponse,
    };
    use cosmwasm_std::Order;
    use cw_storage_plus::Bound;

    pub fn get_config(deps: Deps<SeiQueryWrapper>) -> StdResult<ConfigResponse> {
        let config = load_config(deps.storage)?;

        Ok(ConfigResponse {
            core_contract: config.core_contract.to_string(),
            axis_denom: config.axis_denom.to_string(),
            mint_per_epoch_maker_amount: config.mint_per_epoch_maker_amount,
            mint_per_epoch_trader_amount: config.mint_per_epoch_trader_amount,
        })
    }

    pub fn get_pending_fee(deps: Deps<SeiQueryWrapper>) -> StdResult<PendingFeeResponse> {
        let state = load_state(deps.storage)?;
        Ok(PendingFeeResponse {
            pending_total_fee: state.pending_total_fee_usd,
        })
    }

    pub fn get_pool_allowed_mint_amount(
        deps: Deps<SeiQueryWrapper>,
        base_denom: String,
        price_denom: String,
        start_epoch: u64,
        end_epoch: u64,
    ) -> StdResult<Vec<PoolAllowedMintAmountResponse>> {
        let key = format!("{}:{}", base_denom, price_denom);
        let allow_mint_amounts: Vec<PoolAllowedMintAmountResponse> = POOL_MINT_AMOUNT
            .prefix(&key)
            .range(
                deps.storage,
                Some(Bound::inclusive(start_epoch)),
                Some(Bound::inclusive(end_epoch)),
                Order::Ascending,
            )
            .filter_map(|item| match item {
                Ok((epoch, mint_amount)) => {
                    Some(PoolAllowedMintAmountResponse { epoch, mint_amount })
                }
                Err(_) => None,
            })
            .collect();

        Ok(allow_mint_amounts)
    }

    pub fn get_total_supply(deps: Deps<SeiQueryWrapper>) -> StdResult<TotalSupplyResponse> {
        let config = load_config(deps.storage)?;
        let coin = deps.querier.query_supply(config.axis_denom)?;
        Ok(TotalSupplyResponse {
            denom: coin.denom,
            total_supply: coin.amount,
        })
    }

    pub fn get_epoch_total_fee(
        deps: Deps<SeiQueryWrapper>,
        start_epoch: u64,
        end_epoch: u64,
    ) -> StdResult<Vec<EpochTotalFeeAmountResponse>> {
        let result: Vec<EpochTotalFeeAmountResponse> = EPOCH_TOTAL_FEE_AMOUNT
            .range(
                deps.storage,
                Some(Bound::inclusive(start_epoch)),
                Some(Bound::inclusive(end_epoch)),
                Order::Ascending,
            )
            .filter_map(|item| match item {
                Ok((epoch, amount)) => Some(EpochTotalFeeAmountResponse { epoch, amount }),
                Err(_) => None,
            })
            .collect();

        Ok(result)
    }
}

#[cfg(test)]
mod tests {}
