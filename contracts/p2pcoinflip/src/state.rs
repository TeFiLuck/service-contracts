use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use cosmwasm_std::{
    Addr, Api, CanonicalAddr, Coin, Order, StdError, StdResult, Storage, Timestamp, Uint128, Uint64,
};
use cw_storage_plus::{Item, Map};

use crate::{
    error::ContractError,
    msg::{InstantiateCoinLimitMsg, PendingBetsFilter, PendingBetsSort},
};

use tefiluck::asset::Asset;

static CONFIG: Item<Config> = Item::new("config");
static PENDING_BETS: Map<&Addr, AddrPendingBets> = Map::new("pending_bets");
static PENDING_BETS_COUNT: Item<Uint64> = Item::new("pending_bets_count");
static ONGOING_BETS: Map<String, OngoingBet> = Map::new("ongoing_bets");
static HISTORICAL_BETS: Item<Vec<HistoricalBet>> = Item::new("historical_bets");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: CanonicalAddr,
    pub treasury: CanonicalAddr,
    pub treasury_tax_percent: u8,
    pub max_bets_by_addr: u64,
    pub min_bet_amounts: Vec<CoinLimit>,
    pub min_blocks_until_liquidation: u64,
    pub max_blocks_until_liquidation: u64,
    pub blocks_for_responder_liquidation: u64,
    pub bet_responder_liquidation_percent: u8,
    pub bet_liquidator_percent: u8,
    pub treasury_liquidation_percent: u8,
    pub historical_bets_max_storage_size: u64,
    pub historical_bets_clear_batch_size: u64,
}

impl Config {
    pub fn validate(&self) -> Result<(), ContractError> {
        if self.min_bet_amounts.is_empty() {
            return Err(ContractError::ValidationErr {
                message: "Config validation: min_bet_amounts must be a non-empty list".to_string(),
            });
        }

        if self.treasury_tax_percent > 10 {
            return Err(ContractError::ValidationErr {
                message: "Config validation: treasury percent must be less than 10".to_string(),
            });
        }

        let liquidation_percent = self.bet_responder_liquidation_percent
            + self.bet_liquidator_percent
            + self.treasury_liquidation_percent;
        if liquidation_percent != 100 {
            return Err(ContractError::ValidationErr {
                message: "Config validation: liquidation percent must be equal to 100".to_string(),
            });
        }

        if self.min_blocks_until_liquidation > self.max_blocks_until_liquidation {
            return Err(ContractError::ValidationErr {
                message: "Config validation: min_blocks_until_liquidation must be less than max_blocks_until_liquidation".to_string(),
            });
        }

        Ok(())
    }

    pub fn validate_place_bet_inputs(
        &self,
        blocks_until_liquidation: u64,
        addr_bets_count: usize,
        asset: &Asset,
    ) -> StdResult<()> {
        if self.min_blocks_until_liquidation > blocks_until_liquidation {
            return Err(StdError::generic_err(
                "blocks_before_liquidation must be higher than min allowed value",
            ));
        }

        if self.max_blocks_until_liquidation < blocks_until_liquidation {
            return Err(StdError::generic_err(
                "blocks_before_liquidation must be less than max allowed value",
            ));
        }

        if (addr_bets_count as u64) == self.max_bets_by_addr {
            return Err(StdError::generic_err(
                "max bets by address limit was reached",
            ));
        }

        let coin_limit: &CoinLimit =
            match self.min_bet_amounts.iter().find(|l| l.denom == asset.denom) {
                Some(l) => l,
                None => {
                    return Err(StdError::generic_err(
                        "coin limits for provided asset not found",
                    ))
                }
            };

        if asset.amount < coin_limit.min_amount {
            return Err(StdError::generic_err(
                "provided amount less than min limit for provided asset",
            ));
        }

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct CoinLimit {
    pub denom: String,
    pub min_amount: Uint128,
}

impl CoinLimit {
    pub fn validate(&self, coin: &Coin) -> Result<(), ContractError> {
        if self.min_amount > coin.amount {
            return Err(ContractError::ValidationErr {
                message: "CoinLimit validation: amount for bet must be higher than min limit"
                    .to_string(),
            });
        }

        Ok(())
    }
}

impl From<InstantiateCoinLimitMsg> for CoinLimit {
    fn from(coin: InstantiateCoinLimitMsg) -> Self {
        CoinLimit {
            denom: coin.denom,
            min_amount: Uint128::new(coin.min_amount.into()),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AddrPendingBets {
    pub bets: Vec<PendingBet>,
}

impl AddrPendingBets {
    pub fn store_bet(
        &mut self,
        owner: CanonicalAddr,
        bet_id: String,
        sig: String,
        blocks_until_liquidation: u64,
        asset: Asset,
        time: Timestamp,
    ) -> StdResult<()> {
        if self.bets.iter().any(|el| el.id == bet_id) {
            return Err(StdError::generic_err("bet with same id alreay exists"));
        }

        self.bets.push(PendingBet::new(
            owner,
            bet_id,
            sig,
            blocks_until_liquidation,
            asset,
            time,
        ));
        Ok(())
    }

    pub fn find_by_id(&mut self, bet_id: &String) -> StdResult<PendingBet> {
        match self.bets.iter().find(|bet| bet.id.eq(bet_id)) {
            Some(b) => Ok(b.clone()),
            None => Err(StdError::generic_err("pending bet by id not found")),
        }
    }

    pub fn remove_bet(&mut self, bet_id: &String) {
        self.bets.retain(|bet| bet.id.ne(bet_id))
    }
}

impl Default for AddrPendingBets {
    fn default() -> Self {
        let bets: Vec<PendingBet> = vec![];
        Self { bets: bets }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PendingBet {
    pub owner: CanonicalAddr,
    pub id: String,
    pub signature: String,
    pub blocks_until_liquidation: u64,
    pub asset: Asset,
    pub created_at: Timestamp,
}

impl PendingBet {
    pub fn new(
        owner: CanonicalAddr,
        id: String,
        sig: String,
        blocks_until_liquidation: u64,
        asset: Asset,
        time: Timestamp,
    ) -> Self {
        PendingBet {
            owner: owner,
            id: id,
            signature: sig,
            blocks_until_liquidation: blocks_until_liquidation,
            asset: asset,
            created_at: time,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FlipSide {
    Heads,
    Tails,
}

impl FlipSide {
    pub fn from_u8(side: u8) -> StdResult<FlipSide> {
        match side {
            0 => Ok(FlipSide::Heads),
            1 => Ok(FlipSide::Tails),
            _ => Err(StdError::generic_err("invalid flip side")),
        }
    }

    pub fn u8(&self) -> u8 {
        match self {
            FlipSide::Heads => 0,
            FlipSide::Tails => 1,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct OngoingBet {
    pub signature: String,
    pub bet_creator: Addr,
    pub bet_responder: Addr,
    pub responder_side: FlipSide,
    pub asset: Asset,
    pub started_at_block: u64,
    pub blocks_until_liquidation: u64,
    pub liquidation_block: u64,
    pub responder_liquidation_blocks_gap: u64,
    pub created_at: Timestamp,
}

impl OngoingBet {
    pub fn new(
        sig: String,
        bet_creator: Addr,
        bet_responder: Addr,
        side: FlipSide,
        asset: Asset,
        blocks_until_liquidation: u64,
        blocks_for_responder_liquidation: u64,
        current_block: u64,
        created_at: Timestamp,
    ) -> StdResult<Self> {
        let liquidation_block = match current_block.checked_add(blocks_until_liquidation) {
            Some(l) => l,
            None => return Err(StdError::generic_err("failed to compute liquidation_block")),
        };

        let responder_liquidation_blocks_gap =
            match liquidation_block.checked_add(blocks_for_responder_liquidation) {
                Some(l) => l,
                None => {
                    return Err(StdError::generic_err(
                        "failed to compute responder_liquidation_blocks_gap",
                    ))
                }
            };

        Ok(OngoingBet {
            signature: sig,
            bet_creator: bet_creator,
            bet_responder: bet_responder,
            responder_side: side,
            asset: asset,
            started_at_block: current_block,
            blocks_until_liquidation: blocks_until_liquidation,
            liquidation_block: liquidation_block,
            responder_liquidation_blocks_gap: responder_liquidation_blocks_gap,
            created_at: created_at,
        })
    }
}

impl OngoingBet {
    pub fn resolve_winner(&self, passphrase: &String) -> Addr {
        let split: Vec<&str> = passphrase.split("_").collect();
        if split.len() != 2 {
            return self.bet_responder.clone();
        }

        let side = match split[0].parse::<u8>() {
            Ok(s) => s,
            Err(_) => return self.bet_responder.clone(),
        };

        let flipside = match FlipSide::from_u8(side) {
            Ok(f) => f,
            Err(_) => return self.bet_responder.clone(),
        };

        if flipside == self.responder_side {
            self.bet_responder.clone()
        } else {
            self.bet_creator.clone()
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GameOutcome {
    Resolved,
    Liquidated,
}

impl ToString for GameOutcome {
    fn to_string(&self) -> String {
        match self {
            GameOutcome::Resolved => "resolved".to_string(),
            GameOutcome::Liquidated => "liquidated".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct HistoricalBet {
    pub id: String,
    pub owner: String,
    pub responder: String,
    pub winner: String,
    pub liquidator: Option<String>,
    pub responder_side: u8,
    pub asset: Asset,
    pub outcome: GameOutcome,
    pub created_at: u64,
    pub completed_at: u64,
}

impl HistoricalBet {
    pub fn new(
        id: String,
        owner: String,
        bet_responder: String,
        winner: String,
        liquidator: Option<String>,
        responder_side: FlipSide,
        asset: Asset,
        outcome: GameOutcome,
        created_at: u64,
        completed_at: u64,
    ) -> Self {
        HistoricalBet {
            id: id,
            owner: owner,
            responder: bet_responder,
            winner: winner,
            liquidator: liquidator,
            responder_side: responder_side.u8(),
            asset: asset,
            outcome: outcome,
            created_at: created_at,
            completed_at: completed_at,
        }
    }
}

pub fn store_config(storage: &mut dyn Storage, config: &Config) -> StdResult<()> {
    CONFIG.save(storage, config)
}

pub fn load_config(storage: &dyn Storage) -> StdResult<Config> {
    CONFIG.load(storage)
}

pub fn store_pending_bets(
    storage: &mut dyn Storage,
    addr: &Addr,
    pending_bets: &AddrPendingBets,
) -> StdResult<()> {
    PENDING_BETS.save(storage, addr, pending_bets)
}

pub fn may_load_pending_bets(
    storage: &dyn Storage,
    addr: &Addr,
) -> StdResult<Option<AddrPendingBets>> {
    PENDING_BETS.may_load(storage, addr)
}

pub fn load_pending_bets(storage: &dyn Storage, addr: &Addr) -> StdResult<AddrPendingBets> {
    may_load_pending_bets(storage, addr).map(|res| res.unwrap_or_default())
}

pub fn store_pending_bets_count(storage: &mut dyn Storage, count: Uint64) -> StdResult<()> {
    PENDING_BETS_COUNT.save(storage, &count)
}

pub fn load_pending_bets_count(storage: &dyn Storage) -> StdResult<Uint64> {
    PENDING_BETS_COUNT.load(storage)
}

pub fn store_ongoing_bet(
    storage: &mut dyn Storage,
    bet_id: String,
    ongoing_bet: &OngoingBet,
) -> StdResult<()> {
    ONGOING_BETS.save(storage, bet_id, ongoing_bet)
}

pub fn load_ongoing_bet(storage: &dyn Storage, bet_id: String) -> StdResult<OngoingBet> {
    ONGOING_BETS.load(storage, bet_id)
}

pub fn remove_ongoing_bet(storage: &mut dyn Storage, bet_id: String) {
    ONGOING_BETS.remove(storage, bet_id)
}

pub fn store_historical_bets(
    storage: &mut dyn Storage,
    bets: &Vec<HistoricalBet>,
) -> StdResult<()> {
    HISTORICAL_BETS.save(storage, bets)
}

pub fn load_historical_bets(storage: &dyn Storage) -> StdResult<Vec<HistoricalBet>> {
    HISTORICAL_BETS
        .may_load(storage)
        .map(|res| res.unwrap_or_default())
}

const MAX_LIMIT: u32 = 100;
const DEFAULT_LIMIT: u32 = 10;
pub fn read_pending_bets(
    storage: &dyn Storage,
    api: &dyn Api,
    filter: &PendingBetsFilter,
) -> StdResult<Vec<PendingBet>> {
    let skip = filter.skip as usize;
    let limit = filter.limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    let exclude_addr = match &filter.exclude_address {
        Some(v) => Some(api.addr_canonicalize(&v)?),
        None => None,
    };
    let asset_filters = filter.to_asset_map();

    let mut pending_bets: Vec<PendingBet> = PENDING_BETS
        .range(storage, None, None, Order::Ascending)
        .map(|item| {
            let (_, addr_bets) = item.unwrap_or_default();
            addr_bets.bets
        })
        .flatten()
        .filter(|bet| {
            if let Some(exclude_address) = &exclude_addr {
                if bet.owner.eq(exclude_address) {
                    return false;
                }
            }

            if !asset_filters.is_empty() {
                if let Some((from, to)) = asset_filters.get(&bet.asset.denom) {
                    if let Some(from) = from {
                        if bet.asset.amount.lt(from) {
                            return false;
                        }
                    }

                    if let Some(to) = to {
                        if bet.asset.amount.gt(to) {
                            return false;
                        }
                    }
                } else {
                    return false;
                }
            }

            if let Some(liquidation) = &filter.liquidation {
                if let Some(from) = liquidation.blocks_until_liquidation_from {
                    if bet.blocks_until_liquidation < from {
                        return false;
                    }
                }

                if let Some(to) = liquidation.blocks_until_liquidation_to {
                    if bet.blocks_until_liquidation > to {
                        return false;
                    }
                }
            }

            true
        })
        .collect();

    let sort = match filter.sort_by {
        PendingBetsSort::Creation { asc } => {
            if asc {
                |a: &PendingBet, b: &PendingBet| a.created_at.cmp(&b.created_at)
            } else {
                |a: &PendingBet, b: &PendingBet| b.created_at.cmp(&a.created_at)
            }
        }
        PendingBetsSort::Price { asc } => {
            if asc {
                |a: &PendingBet, b: &PendingBet| a.asset.amount.cmp(&b.asset.amount)
            } else {
                |a: &PendingBet, b: &PendingBet| b.asset.amount.cmp(&a.asset.amount)
            }
        }
    };

    pending_bets.sort_by(sort);
    Ok(pending_bets.into_iter().skip(skip).take(limit).collect())
}

pub fn read_ongoing_bets_by_addr(
    storage: &dyn Storage,
    addr: &Addr,
) -> StdResult<Vec<(String, OngoingBet)>> {
    ONGOING_BETS
        .range(storage, None, None, cosmwasm_std::Order::Ascending)
        .filter(|item| match item {
            Ok(v) => {
                let (_, bet) = v;
                bet.bet_creator.eq(addr) || bet.bet_responder.eq(addr)
            }
            Err(_) => false,
        })
        .map(|item| {
            let (k, v) = item?;
            let bet_id = std::str::from_utf8(&k)?.to_string();
            Ok((bet_id, v))
        })
        .collect()
}

pub fn read_public_liquidatable_bets(
    storage: &dyn Storage,
    current_block: u64,
    skip: u32,
    limit: Option<u32>,
    exclude_addr: Option<Addr>,
) -> StdResult<Vec<(String, OngoingBet)>> {
    let skip = skip as usize;
    let limit = limit.unwrap_or(DEFAULT_LIMIT).min(MAX_LIMIT) as usize;

    ONGOING_BETS
        .range(storage, None, None, cosmwasm_std::Order::Ascending)
        .filter(|item| match item {
            Ok(v) => {
                let (_, bet) = v;

                if let Some(exclude_addr) = &exclude_addr {
                    if bet.bet_creator.eq(exclude_addr) {
                        return false;
                    }
                }

                bet.responder_liquidation_blocks_gap < current_block
            }
            Err(_) => false,
        })
        .map(|item| {
            let (k, v) = item?;
            let bet_id = std::str::from_utf8(&k)?.to_string();
            Ok((bet_id, v))
        })
        .skip(skip)
        .take(limit)
        .collect()
}
