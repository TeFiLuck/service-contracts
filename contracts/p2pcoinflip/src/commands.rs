use cosmwasm_std::{Addr, DepsMut, Env, MessageInfo, Response, StdResult, Storage, Uint64};

use crate::{
    error::ContractError,
    state::{
        load_config, load_historical_bets, load_ongoing_bet, load_pending_bets,
        load_pending_bets_count, remove_ongoing_bet, store_config, store_historical_bets,
        store_ongoing_bet, store_pending_bets, store_pending_bets_count, CoinLimit, Config,
        FlipSide, GameOutcome, HistoricalBet, OngoingBet,
    },
};

use tefiluck::{asset::Asset, hash::calculate_sha256};

pub fn place_bet(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    signature: String,
    blocks_until_liquidation: u64,
) -> Result<Response, ContractError> {
    let config = load_config(deps.storage)?;
    let mut pending_bets = load_pending_bets(deps.storage, &info.sender)?;

    let bet_id = calculate_sha256(&format!(
        "{}{}{}",
        env.block.height,
        env.block.time.to_string(),
        &signature
    ));
    let asset = Asset::from_coins(info.funds)?;

    config.validate_place_bet_inputs(blocks_until_liquidation, pending_bets.bets.len(), &asset)?;

    pending_bets.store_bet(
        deps.api.addr_canonicalize(&info.sender.to_string())?,
        bet_id.clone(),
        signature.clone(),
        blocks_until_liquidation,
        asset.clone(),
        env.block.time,
    )?;

    store_pending_bets(deps.storage, &info.sender, &pending_bets)?;

    let current_bets_count = load_pending_bets_count(deps.storage)?;
    let bets_count = current_bets_count.checked_add(Uint64::new(1u64))?;
    store_pending_bets_count(deps.storage, bets_count)?;

    Ok(Response::new().add_attributes(vec![
        ("action", "place_bet"),
        ("sender", &info.sender.to_string()),
        ("bet_id", &bet_id),
        ("signature", &signature),
        (
            "blocks_until_liquidation",
            &blocks_until_liquidation.to_string(),
        ),
        ("denom", &asset.denom),
        ("amount", &asset.amount.to_string()),
        ("created_at", &env.block.time.seconds().to_string()),
    ]))
}

pub fn respond_bet(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    bet_owner: Addr,
    bet_id: String,
    side: u8,
) -> Result<Response, ContractError> {
    if info.sender == bet_owner {
        return Err(ContractError::ForbiddenToPlayVSYourself {});
    }

    let config = load_config(deps.storage)?;
    let mut pending_bets = load_pending_bets(deps.storage, &bet_owner)?;
    let pending_bet = match pending_bets.find_by_id(&bet_id) {
        Ok(b) => b,
        Err(_) => return Err(ContractError::BetWasCancledOrAccepted {}),
    };

    let mut asset = Asset::from_coins(info.funds)?;
    if asset.ne(&pending_bet.asset) {
        return Err(ContractError::ResponderAssetMismatch {});
    }

    asset.checked_add(&pending_bet.asset)?;

    let flip_side = FlipSide::from_u8(side)?;
    let ongoing_bet = OngoingBet::new(
        pending_bet.signature,
        bet_owner.clone(),
        info.sender.clone(),
        flip_side,
        asset,
        pending_bet.blocks_until_liquidation,
        config.blocks_for_responder_liquidation,
        env.block.height,
        env.block.time,
    )?;

    pending_bets.remove_bet(&bet_id);
    store_pending_bets(deps.storage, &bet_owner, &pending_bets)?;
    store_ongoing_bet(deps.storage, bet_id.clone(), &ongoing_bet)?;

    let current_bets_count = load_pending_bets_count(deps.storage)?;
    let bets_count = current_bets_count.checked_sub(Uint64::new(1u64))?;
    store_pending_bets_count(deps.storage, bets_count)?;

    Ok(Response::new().add_attributes(vec![
        ("action", "respond_bet"),
        ("bet_id", &bet_id),
        ("signature", &ongoing_bet.signature),
        ("bet_creator", &ongoing_bet.bet_creator.to_string()),
        ("bet_responder", &ongoing_bet.bet_responder.to_string()),
        (
            "responder_side",
            &ongoing_bet.responder_side.u8().to_string(),
        ),
        ("denom", &ongoing_bet.asset.denom),
        ("amount", &ongoing_bet.asset.amount.to_string()),
        (
            "started_at_block",
            &ongoing_bet.started_at_block.to_string(),
        ),
        (
            "blocks_until_liquidation",
            &ongoing_bet.blocks_until_liquidation.to_string(),
        ),
        (
            "liquidation_block",
            &ongoing_bet.liquidation_block.to_string(),
        ),
        (
            "responder_liquidation_blocks_gap",
            &ongoing_bet.responder_liquidation_blocks_gap.to_string(),
        ),
        ("created_at", &ongoing_bet.created_at.seconds().to_string()),
    ]))
}

pub fn resolve_bet(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    bet_id: String,
    passphrase: String,
) -> Result<Response, ContractError> {
    if !info.funds.is_empty() {
        return Err(ContractError::ExecuteWithoutFunds {});
    }

    let config = load_config(deps.storage)?;
    let ongoing_bet = match load_ongoing_bet(deps.storage, bet_id.clone()) {
        Ok(b) => b,
        Err(_) => return Err(ContractError::GameWasAlreadyLiquidated {}),
    };

    if info.sender.ne(&ongoing_bet.bet_creator) {
        return Err(ContractError::OnlyBetCreatorAllowedToResolve {});
    }

    let signature = calculate_sha256(&passphrase);
    if signature.ne(&ongoing_bet.signature) {
        return Err(ContractError::SignatureMismatch {});
    }

    let winner_addr = ongoing_bet.resolve_winner(&passphrase);
    let mut bet_amount = ongoing_bet.asset;
    let pot_size = bet_amount.clone();

    let mut treasury_amount = bet_amount.take_percent(config.treasury_tax_percent)?;
    let winner_amount = bet_amount.checked_sub(&treasury_amount)?;

    remove_ongoing_bet(deps.storage, bet_id.clone());

    let historical_bet = HistoricalBet::new(
        bet_id.clone(),
        ongoing_bet.bet_creator.to_string(),
        ongoing_bet.bet_responder.to_string(),
        winner_addr.to_string(),
        None,
        ongoing_bet.responder_side.clone(),
        pot_size,
        GameOutcome::Resolved,
        ongoing_bet.created_at.seconds(),
        env.block.time.seconds(),
    );
    save_historical_bet(deps.storage, &config, historical_bet.clone())?;

    let mut messages = vec![winner_amount.into_bank_msg(&deps.querier, &winner_addr)?];

    if !treasury_amount.amount.is_zero() {
        messages.push(
            treasury_amount
                .into_bank_msg(&deps.querier, &deps.api.addr_humanize(&config.treasury)?)?,
        );
    }

    Ok(Response::new().add_messages(messages).add_attributes(vec![
        ("action", "resolve_bet"),
        ("bet_id", &bet_id),
        ("owner", &historical_bet.owner),
        ("responder", &historical_bet.responder),
        ("winner", &historical_bet.winner),
        ("responder_side", &historical_bet.responder_side.to_string()),
        ("denom", &historical_bet.asset.denom),
        ("amount", &historical_bet.asset.amount.to_string()),
        ("outcome", &historical_bet.outcome.to_string()),
        ("created_at", &historical_bet.created_at.to_string()),
        ("completed_at", &historical_bet.completed_at.to_string()),
    ]))
}

pub fn liquidate_bet(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    bet_id: String,
) -> Result<Response, ContractError> {
    if !info.funds.is_empty() {
        return Err(ContractError::ExecuteWithoutFunds {});
    }

    let config = load_config(deps.storage)?;
    let ongoing_bet = match load_ongoing_bet(deps.storage, bet_id.clone()) {
        Ok(b) => b,
        Err(_) => return Err(ContractError::GameWasAlreadyResolved {}),
    };

    if info.sender.eq(&ongoing_bet.bet_creator) {
        return Err(ContractError::ForbiddenForBetCreatorToLiquidateHimself {});
    }

    if env.block.height <= ongoing_bet.liquidation_block {
        return Err(ContractError::BetIsNotLiquidatableYet {});
    }

    if info.sender.ne(&ongoing_bet.bet_responder)
        && env.block.height <= ongoing_bet.responder_liquidation_blocks_gap
    {
        return Err(ContractError::ResponderLiquidationGapIsNotPassedYet {});
    }

    let responder_addr = ongoing_bet.bet_responder;
    let mut bet_amount = ongoing_bet.asset;
    let pot_size = bet_amount.clone();

    let mut responder_amount = bet_amount.take_percent(config.bet_responder_liquidation_percent)?;
    let mut liquidator_amount = bet_amount.take_percent(config.bet_liquidator_percent)?;
    let treasury_amount = bet_amount
        .checked_sub(&responder_amount)?
        .checked_sub(&liquidator_amount)?;

    remove_ongoing_bet(deps.storage, bet_id.clone());

    let historical_bet = HistoricalBet::new(
        bet_id.clone(),
        ongoing_bet.bet_creator.to_string(),
        responder_addr.to_string(),
        responder_addr.to_string(),
        Some(info.sender.to_string()),
        ongoing_bet.responder_side.clone(),
        pot_size,
        GameOutcome::Liquidated,
        ongoing_bet.created_at.seconds(),
        env.block.time.seconds(),
    );
    save_historical_bet(deps.storage, &config, historical_bet.clone())?;

    let mut messages = vec![
        responder_amount.into_bank_msg(&deps.querier, &responder_addr)?,
        liquidator_amount.into_bank_msg(&deps.querier, &info.sender)?,
    ];

    if !treasury_amount.amount.is_zero() {
        messages.push(
            treasury_amount
                .into_bank_msg(&deps.querier, &deps.api.addr_humanize(&config.treasury)?)?,
        );
    }

    Ok(Response::new().add_messages(messages).add_attributes(vec![
        ("action", "liquidate_bet"),
        ("bet_id", &bet_id),
        ("owner", &historical_bet.owner),
        ("responder", &historical_bet.responder),
        ("winner", &historical_bet.winner),
        ("liquidator", &info.sender.to_string()),
        ("responder_side", &historical_bet.responder_side.to_string()),
        ("denom", &historical_bet.asset.denom),
        ("amount", &historical_bet.asset.amount.to_string()),
        ("outcome", &historical_bet.outcome.to_string()),
        ("created_at", &historical_bet.created_at.to_string()),
        ("completed_at", &historical_bet.completed_at.to_string()),
    ]))
}

pub fn withdraw_pending_bet(
    deps: DepsMut,
    info: MessageInfo,
    bet_id: String,
) -> Result<Response, ContractError> {
    if !info.funds.is_empty() {
        return Err(ContractError::ExecuteWithoutFunds {});
    }

    let mut pending_bets = load_pending_bets(deps.storage, &info.sender)?;
    let pending_bet = match pending_bets.find_by_id(&bet_id) {
        Ok(b) => b,
        Err(_) => return Err(ContractError::GameWasAlreadyAccepted {}),
    };

    let mut withdraw_amount = pending_bet.asset;
    let send_msg = withdraw_amount.into_bank_msg(&deps.querier, &info.sender)?;

    pending_bets.remove_bet(&bet_id);
    store_pending_bets(deps.storage, &info.sender, &pending_bets)?;

    let current_bets_count = load_pending_bets_count(deps.storage)?;
    let bets_count = current_bets_count.checked_sub(Uint64::new(1u64))?;
    store_pending_bets_count(deps.storage, bets_count)?;

    Ok(Response::new().add_message(send_msg).add_attributes(vec![
        ("action", "withdraw_pending_bet"),
        ("bet_id", &bet_id),
    ]))
}

// only owner allowed to change config params
pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    owner: Option<String>,
    treasury: Option<String>,
    treasury_tax_percent: Option<u8>,
    max_bets_by_addr: Option<u64>,
    min_bet_amounts: Option<Vec<CoinLimit>>,
    min_blocks_until_liquidation: Option<u64>,
    max_blocks_until_liquidation: Option<u64>,
    blocks_for_responder_liquidation: Option<u64>,
    bet_responder_liquidation_percent: Option<u8>,
    bet_liquidator_percent: Option<u8>,
    treasury_liquidation_percent: Option<u8>,
    historical_bets_max_storage_size: Option<u64>,
    historical_bets_clear_batch_size: Option<u64>,
) -> Result<Response, ContractError> {
    let mut config = load_config(deps.storage)?;

    if deps.api.addr_canonicalize(info.sender.as_str())? != config.owner {
        return Err(ContractError::Unauthorized {});
    }

    if let Some(owner) = owner {
        let _ = deps.api.addr_validate(&owner)?;
        config.owner = deps.api.addr_canonicalize(&owner)?;
    }

    if let Some(treasury) = treasury {
        let _ = deps.api.addr_validate(&treasury)?;
        config.treasury = deps.api.addr_canonicalize(&treasury)?;
    }

    if let Some(treasury_tax_percent) = treasury_tax_percent {
        config.treasury_tax_percent = treasury_tax_percent;
    }

    if let Some(max_bets_by_addr) = max_bets_by_addr {
        config.max_bets_by_addr = max_bets_by_addr;
    }

    if let Some(min_bet_amounts) = min_bet_amounts {
        config.min_bet_amounts = min_bet_amounts;
    }

    if let Some(min_blocks_until_liquidation) = min_blocks_until_liquidation {
        config.min_blocks_until_liquidation = min_blocks_until_liquidation;
    }

    if let Some(max_blocks_until_liquidation) = max_blocks_until_liquidation {
        config.max_blocks_until_liquidation = max_blocks_until_liquidation;
    }

    if let Some(blocks_for_responder_liquidation) = blocks_for_responder_liquidation {
        config.blocks_for_responder_liquidation = blocks_for_responder_liquidation;
    }

    if let Some(bet_responder_liquidation_percent) = bet_responder_liquidation_percent {
        config.bet_responder_liquidation_percent = bet_responder_liquidation_percent;
    }

    if let Some(bet_liquidator_percent) = bet_liquidator_percent {
        config.bet_liquidator_percent = bet_liquidator_percent;
    }

    if let Some(treasury_liquidation_percent) = treasury_liquidation_percent {
        config.treasury_liquidation_percent = treasury_liquidation_percent;
    }

    if let Some(historical_bets_max_storage_size) = historical_bets_max_storage_size {
        config.historical_bets_max_storage_size = historical_bets_max_storage_size;
    }

    if let Some(historical_bets_clear_batch_size) = historical_bets_clear_batch_size {
        config.historical_bets_clear_batch_size = historical_bets_clear_batch_size;
    }

    let _ = config.validate()?;
    store_config(deps.storage, &config)?;

    Ok(Response::new().add_attribute("action", "update_config"))
}

fn save_historical_bet(
    storage: &mut dyn Storage,
    config: &Config,
    bet: HistoricalBet,
) -> StdResult<()> {
    let mut historical_bets = load_historical_bets(storage)?;
    if historical_bets.len() >= config.historical_bets_max_storage_size as usize {
        let crear_batch_size = config.historical_bets_clear_batch_size as usize;
        historical_bets.drain(0..crear_batch_size);
    }

    historical_bets.push(bet);
    store_historical_bets(storage, &historical_bets)?;

    Ok(())
}
