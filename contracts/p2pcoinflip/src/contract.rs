#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult, Uint64,
};
use cw2::set_contract_version;

use crate::{
    commands,
    error::ContractError,
    msg::{ExecuteMsg, InstantiateMsg, QueryMsg},
    queries,
    state::{store_config, store_pending_bets_count, CoinLimit, Config},
};

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:p2pcoinflip";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let min_bet_amounts: Vec<CoinLimit> = msg
        .min_bet_amounts
        .into_iter()
        .map(|coin| coin.into())
        .collect();

    let config = Config {
        owner: deps.api.addr_canonicalize(info.sender.as_str())?,
        treasury: deps.api.addr_canonicalize(&msg.treasury)?,
        treasury_tax_percent: msg.treasury_tax_percent,
        max_bets_by_addr: msg.max_bets_by_addr,
        min_bet_amounts: min_bet_amounts,
        min_blocks_until_liquidation: msg.min_blocks_until_liquidation,
        max_blocks_until_liquidation: msg.max_blocks_until_liquidation,
        blocks_for_responder_liquidation: msg.blocks_for_responder_liquidation,
        bet_responder_liquidation_percent: msg.bet_responder_liquidation_percent,
        bet_liquidator_percent: msg.bet_liquidator_percent,
        treasury_liquidation_percent: msg.treasury_liquidation_percent,
        historical_bets_max_storage_size: msg.historical_bets_max_storage_size,
        historical_bets_clear_batch_size: msg.historical_bets_clear_batch_size,
    };

    let _ = config.validate()?;
    store_config(deps.storage, &config)?;

    store_pending_bets_count(deps.storage, Uint64::new(0u64))?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::PlaceBet {
            signature,
            blocks_until_liquidation,
        } => commands::place_bet(
            deps,
            env,
            info,
            signature.to_lowercase(),
            blocks_until_liquidation,
        ),
        ExecuteMsg::RespondBet {
            bet_owner,
            bet_id,
            side,
        } => {
            let bet_owner = deps.api.addr_validate(&bet_owner)?;
            commands::respond_bet(deps, env, info, bet_owner, bet_id.to_lowercase(), side)
        }
        ExecuteMsg::ResolveBet { bet_id, passphrase } => {
            commands::resolve_bet(deps, env, info, bet_id.to_lowercase(), passphrase)
        }
        ExecuteMsg::LiquidateBet { bet_id } => {
            commands::liquidate_bet(deps, env, info, bet_id.to_lowercase())
        }
        ExecuteMsg::WithdrawPendingBet { bet_id } => {
            commands::withdraw_pending_bet(deps, info, bet_id.to_lowercase())
        }
        ExecuteMsg::UpdateConfig {
            owner,
            treasury,
            treasury_tax_percent,
            max_bets_by_addr,
            min_bet_amounts,
            min_blocks_until_liquidation,
            max_blocks_until_liquidation,
            blocks_for_responder_liquidation,
            bet_responder_liquidation_percent,
            bet_liquidator_percent,
            treasury_liquidation_percent,
            historical_bets_max_storage_size,
            historical_bets_clear_batch_size,
        } => commands::update_config(
            deps,
            info,
            owner,
            treasury,
            treasury_tax_percent,
            max_bets_by_addr,
            min_bet_amounts,
            min_blocks_until_liquidation,
            max_blocks_until_liquidation,
            blocks_for_responder_liquidation,
            bet_responder_liquidation_percent,
            bet_liquidator_percent,
            treasury_liquidation_percent,
            historical_bets_max_storage_size,
            historical_bets_clear_batch_size,
        ),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&queries::query_config(deps)?),
        QueryMsg::PendingBetsByAddr { address } => {
            let address = deps.api.addr_validate(&address)?;
            to_binary(&queries::query_pending_bets_by_addr(deps, address)?)
        }
        QueryMsg::PendingBetById { address, bet_id } => {
            let address = deps.api.addr_validate(&address)?;
            to_binary(&queries::query_pending_bet_by_id(deps, address, bet_id)?)
        }
        QueryMsg::PendingBets { filter } => to_binary(&queries::query_pending_bets(deps, filter)?),
        QueryMsg::PendingBetsCount {} => to_binary(&queries::query_pending_bets_count(deps)?),
        QueryMsg::OngoingBet { bet_id } => {
            to_binary(&queries::query_ongoing_bet(deps, bet_id.to_lowercase())?)
        }
        QueryMsg::OngoingBetsByAddr { address } => {
            let addr = deps.api.addr_validate(&address)?;
            to_binary(&queries::query_ongoing_bets_by_addr(deps, addr)?)
        }
        QueryMsg::PublicLiquidatable {
            skip,
            limit,
            exclude_address,
        } => {
            let current_block = env.block.height;
            to_binary(&queries::query_public_liquidatable_bets(
                deps,
                current_block,
                skip,
                limit,
                exclude_address,
            )?)
        }
        QueryMsg::HistoricalBets {
            skip,
            limit,
            address,
        } => {
            let addr = deps.api.addr_validate(&address)?;
            to_binary(&queries::query_historical_bet(deps, skip, limit, addr)?)
        }
    }
}
