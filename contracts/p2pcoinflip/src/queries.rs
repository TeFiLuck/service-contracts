use cosmwasm_std::{Addr, Deps, StdResult};

use crate::{
    msg::{
        AddrPendingBetsResponse, ConfigResponse, HistoricalBetResponse, OngoingBetResponse,
        PendingBetResponse, PendingBetsFilter, TotalPendingBetsResponse,
    },
    state::{
        load_config, load_historical_bets, load_ongoing_bet, load_pending_bets,
        load_pending_bets_count, read_ongoing_bets_by_addr, read_pending_bets,
        read_public_liquidatable_bets, HistoricalBet,
    },
};

pub fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = load_config(deps.storage)?;
    let response = ConfigResponse {
        owner: deps.api.addr_humanize(&config.owner)?.to_string(),
        treasury: deps.api.addr_humanize(&config.treasury)?.to_string(),
        treasury_tax_percent: config.treasury_tax_percent,
        max_bets_by_addr: config.max_bets_by_addr,
        min_bet_amounts: config.min_bet_amounts,
        min_blocks_until_liquidation: config.min_blocks_until_liquidation,
        max_blocks_until_liquidation: config.max_blocks_until_liquidation,
        blocks_for_responder_liquidation: config.blocks_for_responder_liquidation,
        bet_responder_liquidation_percent: config.bet_responder_liquidation_percent,
        bet_liquidator_percent: config.bet_liquidator_percent,
        treasury_liquidation_percent: config.treasury_liquidation_percent,
        historical_bets_max_storage_size: config.historical_bets_max_storage_size,
        historical_bets_clear_batch_size: config.historical_bets_clear_batch_size,
    };

    Ok(response)
}

pub fn query_pending_bets_by_addr(deps: Deps, addr: Addr) -> StdResult<AddrPendingBetsResponse> {
    let bets = load_pending_bets(deps.storage, &addr)?;
    let resp = bets
        .bets
        .iter()
        .map(|bet| {
            PendingBetResponse::new(deps.api.addr_humanize(&bet.owner).unwrap().to_string(), bet)
        })
        .collect();

    Ok(AddrPendingBetsResponse { bets: resp })
}

pub fn query_pending_bet_by_id(
    deps: Deps,
    addr: Addr,
    bet_id: String,
) -> StdResult<PendingBetResponse> {
    let mut bets = load_pending_bets(deps.storage, &addr)?;
    let bet = bets.find_by_id(&bet_id)?;
    let resp = PendingBetResponse::new(deps.api.addr_humanize(&bet.owner)?.to_string(), &bet);

    Ok(resp)
}

pub fn query_pending_bets(
    deps: Deps,
    filter: PendingBetsFilter,
) -> StdResult<Vec<PendingBetResponse>> {
    let bets = read_pending_bets(deps.storage, deps.api, &filter)?;
    bets.iter()
        .map(|bet| {
            Ok(PendingBetResponse::new(
                deps.api.addr_humanize(&bet.owner)?.to_string(),
                bet,
            ))
        })
        .collect()
}

pub fn query_pending_bets_count(deps: Deps) -> StdResult<TotalPendingBetsResponse> {
    let bets_count = load_pending_bets_count(deps.storage)?;
    Ok(TotalPendingBetsResponse {
        count: bets_count.u64(),
    })
}

pub fn query_ongoing_bet(deps: Deps, bet_id: String) -> StdResult<OngoingBetResponse> {
    let bet = load_ongoing_bet(deps.storage, bet_id.clone())?;
    let resp = OngoingBetResponse::new(bet_id, &bet);

    Ok(resp)
}

pub fn query_ongoing_bets_by_addr(deps: Deps, addr: Addr) -> StdResult<Vec<OngoingBetResponse>> {
    let bets = read_ongoing_bets_by_addr(deps.storage, &addr)?;
    Ok(bets
        .iter()
        .map(|v| {
            let (bet_id, bet) = v;
            OngoingBetResponse::new(bet_id.clone(), bet)
        })
        .collect())
}

pub fn query_public_liquidatable_bets(
    deps: Deps,
    block: u64,
    skip: u32,
    limit: Option<u32>,
    exclude_addr: Option<String>,
) -> StdResult<Vec<OngoingBetResponse>> {
    let exclude_addr = match exclude_addr {
        Some(exclude_addr) => Some(deps.api.addr_validate(&exclude_addr)?),
        None => None,
    };

    let bets = read_public_liquidatable_bets(deps.storage, block, skip, limit, exclude_addr)?;
    Ok(bets
        .iter()
        .map(|v| {
            let (bet_id, bet) = v;
            OngoingBetResponse::new(bet_id.clone(), bet)
        })
        .collect())
}

pub fn query_historical_bet(
    deps: Deps,
    skip: u32,
    limit: u32,
    addr: Addr,
) -> StdResult<HistoricalBetResponse> {
    let addr = addr.to_string();
    let mut bets: Vec<HistoricalBet> = load_historical_bets(deps.storage)?
        .into_iter()
        .filter(|bet| {
            bet.owner.eq(&addr) || bet.responder.eq(&addr) || bet.liquidator.eq(&Some(addr.clone()))
        })
        .skip(skip as usize)
        .take(limit as usize)
        .collect();

    bets.sort_by(|a, b| b.completed_at.cmp(&a.completed_at));

    Ok(HistoricalBetResponse { history: bets })
}
