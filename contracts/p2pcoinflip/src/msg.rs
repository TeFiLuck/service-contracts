use std::collections::HashMap;

use cosmwasm_std::Uint128;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tefiluck::asset::Asset;

use crate::state::{CoinLimit, HistoricalBet, OngoingBet, PendingBet};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub treasury: String,
    pub treasury_tax_percent: u8,
    pub max_bets_by_addr: u64,
    pub min_bet_amounts: Vec<InstantiateCoinLimitMsg>,
    pub min_blocks_until_liquidation: u64,
    pub max_blocks_until_liquidation: u64,
    pub blocks_for_responder_liquidation: u64,
    pub bet_responder_liquidation_percent: u8,
    pub bet_liquidator_percent: u8,
    pub treasury_liquidation_percent: u8,
    pub historical_bets_max_storage_size: u64,
    pub historical_bets_clear_batch_size: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateCoinLimitMsg {
    pub denom: String,
    pub min_amount: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    PlaceBet {
        signature: String,
        blocks_until_liquidation: u64,
    },
    RespondBet {
        bet_owner: String,
        bet_id: String,
        side: u8,
    },
    ResolveBet {
        bet_id: String,
        passphrase: String,
    },
    LiquidateBet {
        bet_id: String,
    },
    WithdrawPendingBet {
        bet_id: String,
    },
    UpdateConfig {
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
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
    PendingBetsByAddr {
        address: String,
    },
    PendingBetById {
        address: String,
        bet_id: String,
    },
    PendingBets {
        filter: PendingBetsFilter,
    },
    PendingBetsCount {},
    OngoingBet {
        bet_id: String,
    },
    OngoingBetsByAddr {
        address: String,
    },
    PublicLiquidatable {
        skip: u32,
        limit: Option<u32>,
        exclude_address: Option<String>,
    },
    HistoricalBets {
        skip: u32,
        limit: u32,
        address: String,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct ConfigResponse {
    pub owner: String,
    pub treasury: String,
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

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PendingBetsFilter {
    pub skip: u32,
    pub limit: Option<u32>,
    pub exclude_address: Option<String>,
    pub assets: Option<Vec<AssetFilter>>,
    pub liquidation: Option<LiquidationFilter>,
    pub sort_by: PendingBetsSort,
}

impl PendingBetsFilter {
    pub fn to_asset_map(&self) -> HashMap<String, (Option<Uint128>, Option<Uint128>)> {
        match &self.assets {
            Some(assets) => {
                let mut m = HashMap::new();
                for asset_filter in assets {
                    m.insert(
                        asset_filter.denom.clone(),
                        (
                            asset_filter.bet_size_from.clone(),
                            asset_filter.bet_size_to.clone(),
                        ),
                    );
                }

                m
            }
            None => HashMap::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AssetFilter {
    pub denom: String,
    pub bet_size_from: Option<Uint128>,
    pub bet_size_to: Option<Uint128>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct LiquidationFilter {
    pub blocks_until_liquidation_from: Option<u64>,
    pub blocks_until_liquidation_to: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PendingBetsSort {
    Creation { asc: bool },
    Price { asc: bool },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct AddrPendingBetsResponse {
    pub bets: Vec<PendingBetResponse>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct PendingBetResponse {
    pub owner: String,
    pub id: String,
    pub signature: String,
    pub blocks_until_liquidation: u64,
    pub asset: Asset,
    pub created_at: u64,
}

impl PendingBetResponse {
    pub fn new(owner: String, bet: &PendingBet) -> Self {
        Self {
            owner: owner,
            id: bet.id.clone(),
            signature: bet.signature.clone(),
            blocks_until_liquidation: bet.blocks_until_liquidation,
            asset: bet.asset.clone(),
            created_at: bet.created_at.seconds(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct OngoingBetResponse {
    pub id: String,
    pub signature: String,
    pub owner: String,
    pub responder: String,
    pub responder_side: u8,
    pub asset: Asset,
    pub started_at_block: u64,
    pub blocks_until_liquidation: u64,
    pub liquidation_block: u64,
    pub responder_liquidation_blocks_gap: u64,
    pub created_at: u64,
}

impl OngoingBetResponse {
    pub fn new(bet_id: String, bet: &OngoingBet) -> Self {
        Self {
            id: bet_id,
            signature: bet.signature.clone(),
            owner: bet.bet_creator.to_string(),
            responder: bet.bet_responder.to_string(),
            responder_side: bet.responder_side.u8(),
            asset: bet.asset.clone(),
            started_at_block: bet.started_at_block,
            blocks_until_liquidation: bet.blocks_until_liquidation.clone(),
            liquidation_block: bet.liquidation_block.clone(),
            responder_liquidation_blocks_gap: bet.responder_liquidation_blocks_gap.clone(),
            created_at: bet.created_at.seconds(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct HistoricalBetResponse {
    pub history: Vec<HistoricalBet>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TotalPendingBetsResponse {
    pub count: u64,
}
