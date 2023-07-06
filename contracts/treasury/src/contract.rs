#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    coin, to_binary, BankMsg, Binary, CosmosMsg, Decimal, Deps, DepsMut, Env, MessageInfo,
    Response, StdResult, SubMsg, Uint128, WasmMsg,
};
use cw2::set_contract_version;
use sei_cosmwasm::{SeiMsg, SeiQueryWrapper};

use crate::error::ContractError;
use crate::state::{State, STATE};
use axis::treasury::{ExecuteMsg, InstantiateMsg, QueryMsg};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:gmx";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

const TWENTY_FOUR_SECONDS: u64 = 86400;
const MAX_TOTAL_SUPPLY: u128 = 1_000_000_000_000_000;
const DENOM: &str = "AXIS";
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
    let denom = "factory/".to_string() + env.contract.address.to_string().as_ref() + "/" + DENOM;

    let state = State {
        owner: msg.owner.clone(),
        denom: denom.clone(),
        epoch: 0,
        core_contract: info.sender.clone(),
        total_fee: Uint128::zero(),
        last_update_timestamp: env.block.time.seconds(),
        total_supply: owner_mint_amount,
        mint_amount_per_epoch: (Uint128::new(600_000_000_000_000) / Uint128::new(365 * 5)),
        // is_airdrop: false,
    };
    STATE.save(deps.storage, &state)?;
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let rune_create_msg = SeiMsg::CreateDenom {
        subdenom: DENOM.to_owned(),
    };
    let mint_token = coin(owner_mint_amount.into(), denom);
    let rune_mint_msg = SeiMsg::MintTokens {
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
            .add_message(rune_create_msg)
            .add_message(rune_mint_msg)
            .add_submessage(owner_send_msg), // .add_submessage(community_send_msg)
    )
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut<SeiQueryWrapper>,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<SeiMsg>, ContractError> {
    match msg {
        ExecuteMsg::AddFeeAmount {
            asset_denom,
            stable_denom,
            trader,
            fee_usd_amount,
        } => execute::add_fee_amount(
            deps,
            info,
            asset_denom,
            stable_denom,
            trader,
            fee_usd_amount,
        ),
        ExecuteMsg::Setting {} => execute::setting(deps, env, info),
        ExecuteMsg::ClaimMint {} => execute::claim_minting(deps, info),
        // ExecuteMsg::RegisterAirDrop { air_drop_contract } => {
        //     execute::register_airdrop(deps, info, air_drop_contract)
        // }
    }
}

pub mod execute {
    use cosmwasm_std::{coin, Addr, BankMsg, Decimal, SubMsg};

    use sei_cosmwasm::SeiQueryWrapper;

    use crate::{
        helpers::{check_core_contract, check_last_updated, check_market_contract},
        query::query_pair_market,
        state::{
            load_state, load_treasurys, save_state, update_treasury, Treasury, TOTAL_FEE_AMOUNT,
            TREASURY,
        },
    };

    use super::*;
    pub fn add_fee_amount(
        deps: DepsMut<SeiQueryWrapper>,
        info: MessageInfo,
        asset_denom: String,
        stable_denom: String,
        trader: String,
        amount: Uint128,
    ) -> Result<Response<SeiMsg>, ContractError> {
        let mut state = load_state(deps.storage)?;
        let market_contract = query_pair_market(
            deps.querier,
            &state.core_contract,
            &asset_denom,
            &stable_denom,
        )?;
        check_market_contract(&market_contract, &info.sender.to_string())?;
        update_treasury(deps.storage, trader, amount, &state)?;
        state.total_fee += amount;
        save_state(deps.storage, &state)?;
        Ok(Response::new())
    }

    pub fn setting(
        deps: DepsMut<SeiQueryWrapper>,
        env: Env,
        info: MessageInfo,
    ) -> Result<Response<SeiMsg>, ContractError> {
        let mut state = load_state(deps.storage)?;
        check_core_contract(&state.core_contract, &info.sender)?;
        //24 hour
        check_last_updated(env.block.time.seconds(), state.last_update_timestamp)?;
        TOTAL_FEE_AMOUNT.save(deps.storage, state.epoch, &state.total_fee)?;
        state.epoch += 1;
        state.total_supply += state.mint_amount_per_epoch;
        state.total_fee = Uint128::zero();
        state.last_update_timestamp = state.last_update_timestamp + TWENTY_FOUR_SECONDS;
        save_state(deps.storage, &state)?;
        Ok(Response::default())
    }

    pub fn claim_minting(
        deps: DepsMut<SeiQueryWrapper>,
        info: MessageInfo,
    ) -> Result<Response<SeiMsg>, ContractError> {
        let state = load_state(deps.storage)?;
        let treasurys = load_treasurys(deps.storage, info.sender.clone())?;

        let mint_amount: Uint128 = treasurys
            .iter()
            .map(|treasury| {
                let total_fee_amount = TOTAL_FEE_AMOUNT.load(deps.storage, treasury.epoch)?;
                let ratio = Decimal::from_ratio(treasury.fee_amount, total_fee_amount);
                Ok(state.mint_amount_per_epoch * ratio)
            })
            .sum::<Result<Uint128, ContractError>>()?;

        TREASURY.remove(deps.storage, info.sender.clone());
        let token = coin(mint_amount.into(), "AXIS");
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
    // pub fn burn()
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps<SeiQueryWrapper>, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {}
}

pub mod query {
    use super::*;
}

#[cfg(test)]
mod tests {}
