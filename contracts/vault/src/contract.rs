#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;
use sei_cosmwasm::{SeiMsg, SeiQueryWrapper};
// use cw2::set_contract_version;

use crate::{
    error::ContractError,
    state::{save_config, Config},
};
use axis_protocol::{
    query::query_epoch,
    vault::{ExecuteMsg, InstantiateMsg, QueryMsg},
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:vault";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut<SeiQueryWrapper>,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response<SeiMsg>, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    let InstantiateMsg {
        core_contract,
        es_axis_contract,
        es_axis_denom,
        denom_list,
    } = msg;
    let core_contract = deps.api.addr_validate(&core_contract)?;
    let es_axis_contract = deps.api.addr_validate(&es_axis_contract)?;
    let epoch = query_epoch(deps.querier, &core_contract)?;
    let config = Config {
        owner: info.sender,
        epoch,
        core_contract,
        es_axis_contract,
        es_axis_denom,
        denom_list,
    };
    save_config(deps.storage, &config)?;
    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut<SeiQueryWrapper>,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response<SeiMsg>, ContractError> {
    match msg {
        ExecuteMsg::RecievedFee {
            base_denom,
            base_amount,
            price_denom,
            price_amount,
        } => execute::recieved_fee(
            deps,
            info,
            base_denom,
            base_amount,
            price_denom,
            price_amount,
        ),
        ExecuteMsg::Swap {} => execute::swap(deps, info),
        ExecuteMsg::Setting { epoch } => execute::setting(deps, info, epoch),
    }
}
pub mod execute {
    use crate::{
        helpers::{
            check_core_contract, check_denom_and_amount, check_funds_and_get_token, check_owner,
            check_valid_denom,
        },
        query::query_ex_axis_total_supply,
        state::{
            load_balance, load_config, load_pending_balance, save_balance, save_config, BALANCE,
            PENDING_BALANCE,
        },
        ContractError,
    };
    use axis_protocol::es_axis::ExecuteMsg as ESAxisExecuteMsg;
    use cosmwasm_std::{
        coin, to_binary, BankMsg, Coin, CosmosMsg, Decimal, DepsMut, MessageInfo, Response,
        StdResult, SubMsg, Uint128, WasmMsg,
    };
    use sei_cosmwasm::{SeiMsg, SeiQueryWrapper};

    pub fn recieved_fee(
        deps: DepsMut<SeiQueryWrapper>,
        info: MessageInfo,
        base_denom: String,
        base_amount: Uint128,
        price_denom: String,
        price_amount: Uint128,
    ) -> Result<Response<SeiMsg>, ContractError> {
        let config = load_config(deps.storage)?;
        check_valid_denom(&config.denom_list, &base_denom, &price_denom)?;
        check_denom_and_amount(
            info.funds,
            &base_denom,
            base_amount,
            &price_denom,
            price_amount,
        )?;

        PENDING_BALANCE.update(deps.storage, &base_denom, |exsists| -> StdResult<Uint128> {
            match exsists {
                Some(denom_amount) => Ok(denom_amount + base_amount),
                None => Ok(base_amount),
            }
        })?;

        PENDING_BALANCE.update(
            deps.storage,
            &price_denom,
            |exsists| -> StdResult<Uint128> {
                match exsists {
                    Some(denom_amount) => Ok(denom_amount + price_amount),
                    None => Ok(price_amount),
                }
            },
        )?;
        Ok(Response::new())
    }
    pub fn swap(
        deps: DepsMut<SeiQueryWrapper>,
        info: MessageInfo,
    ) -> Result<Response<SeiMsg>, ContractError> {
        let config = load_config(deps.storage)?;
        //@@ 그냥 토큰들 다 줄까 아니면 하나의 토큰으로 교환해서 줄까?
        //es_axis 토큰 수량 체크
        let es_axis_token = check_funds_and_get_token(info.funds, &config.es_axis_denom)?;
        let es_axis_total_supply =
            query_ex_axis_total_supply(deps.querier, &config.es_axis_contract)?;
        let ratio = Decimal::from_ratio(es_axis_token.amount, es_axis_total_supply);

        let token_vec = config
            .denom_list
            .iter()
            .map(|denom| {
                let balance = BALANCE.load(deps.storage, denom)?;
                let sender_amount = balance * ratio;
                Ok(coin(sender_amount.into(), denom))
            })
            .collect::<Result<Vec<Coin>, ContractError>>()?;

        let send_msg = SubMsg::new(BankMsg::Send {
            to_address: info.sender.to_string(),
            amount: token_vec,
        });
        let burn_msg = CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: config.es_axis_contract.to_string(),
            msg: to_binary(&ESAxisExecuteMsg::Burn {})?,
            funds: vec![es_axis_token],
        });
        Ok(Response::new()
            .add_message(burn_msg)
            .add_submessage(send_msg))
    }

    pub fn update_denom_list(
        deps: DepsMut<SeiQueryWrapper>,
        info: MessageInfo,
        new_denom: String,
    ) -> Result<Response<SeiMsg>, ContractError> {
        let mut config = load_config(deps.storage)?;
        //pool_contract 에서 실행시킬수 있게 할까?
        //이건 reply 가 되는지 확인해봐야 알 수 있다.
        //일단 owner 만 실행할 수 있게 하자.
        check_owner(&info.sender, &config.owner)?;
        match config.denom_list.contains(&new_denom) {
            true => Err(ContractError::InvalidDenom {}),
            false => Ok(()),
        }?;
        config.denom_list.push(new_denom);
        save_config(deps.storage, &config)?;
        Ok(Response::new())
    }

    pub fn setting(
        deps: DepsMut<SeiQueryWrapper>,
        info: MessageInfo,
        epoch: u64,
    ) -> Result<Response<SeiMsg>, ContractError> {
        // check core contract
        // pending -> balance 로 이동

        let mut config = load_config(deps.storage)?;
        config.epoch = epoch;
        check_core_contract(&config.core_contract, &info.sender)?;

        for denom in config.denom_list.iter() {
            let pending_balance = load_pending_balance(deps.storage, denom)?;
            let mut balance = load_balance(deps.storage, denom)?;
            balance += pending_balance;
            save_balance(deps.storage, denom, &balance)?;
        }

        PENDING_BALANCE.clear(deps.storage);

        Ok(Response::new())
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps<SeiQueryWrapper>, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::GetDenomBalance { denom } => to_binary(&query::get_denom_balance(deps, denom)?),
        QueryMsg::GetDenomPendingBalance { denom } => {
            to_binary(&query::get_denom_pending_balance(deps, denom)?)
        }
        QueryMsg::GetAddressBalance { address } => {
            to_binary(&query::get_address_balance(deps, address)?)
        }
    }
}

pub mod query {
    use crate::state::{load_balance, load_config, load_pending_balance, BALANCE};
    use axis_protocol::vault::{
        GetAddressBalanceResponse, GetDenomBalanceResponse, GetDenomPendingBalanceResponse,
    };
    use cosmwasm_std::{Decimal, Deps, StdResult};
    use sei_cosmwasm::SeiQueryWrapper;

    pub fn get_denom_balance(
        deps: Deps<SeiQueryWrapper>,
        denom: String,
    ) -> StdResult<GetDenomBalanceResponse> {
        let amount = load_balance(deps.storage, &denom)?;
        Ok(GetDenomBalanceResponse { denom, amount })
    }

    pub fn get_denom_pending_balance(
        deps: Deps<SeiQueryWrapper>,
        denom: String,
    ) -> StdResult<GetDenomPendingBalanceResponse> {
        let amount = load_pending_balance(deps.storage, &denom)?;
        Ok(GetDenomPendingBalanceResponse { denom, amount })
    }

    pub fn get_address_balance(
        deps: Deps<SeiQueryWrapper>,
        address: String,
    ) -> StdResult<GetAddressBalanceResponse> {
        let config = load_config(deps.storage)?;
        let addr_es_axis_amount = deps
            .querier
            .query_balance(address, &config.es_axis_denom)?
            .amount;
        let es_axis_total_supply = deps.querier.query_supply(&config.es_axis_denom)?.amount;
        let ratio = Decimal::from_ratio(addr_es_axis_amount, es_axis_total_supply);
        let balance_vec = config
            .denom_list
            .into_iter()
            .map(|denom| {
                let balance = BALANCE.load(deps.storage, &denom)?;
                let amount = balance * ratio;
                Ok(GetDenomBalanceResponse { denom, amount })
            })
            .collect::<StdResult<Vec<_>>>()?;

        Ok(GetAddressBalanceResponse {
            balances: balance_vec,
        })
    }
}
