use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, remove_schemas, schema_for};

use p2pcoinflip::{
    msg::{
        AddrPendingBetsResponse, AssetFilter, ConfigResponse, ExecuteMsg, HistoricalBetResponse,
        InstantiateCoinLimitMsg, InstantiateMsg, LiquidationFilter, OngoingBetResponse,
        PendingBetResponse, PendingBetsFilter, PendingBetsSort, QueryMsg, TotalPendingBetsResponse,
    },
    state::{
        AddrPendingBets, CoinLimit, Config, FlipSide, GameOutcome, HistoricalBet, OngoingBet,
        PendingBet,
    },
};

use tefiluck::asset::Asset;

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(InstantiateMsg), &out_dir);
    export_schema(&schema_for!(InstantiateCoinLimitMsg), &out_dir);

    export_schema(&schema_for!(Config), &out_dir);
    export_schema(&schema_for!(CoinLimit), &out_dir);
    export_schema(&schema_for!(Asset), &out_dir);
    export_schema(&schema_for!(FlipSide), &out_dir);
    export_schema(&schema_for!(AddrPendingBets), &out_dir);
    export_schema(&schema_for!(PendingBet), &out_dir);
    export_schema(&schema_for!(OngoingBet), &out_dir);
    export_schema(&schema_for!(GameOutcome), &out_dir);
    export_schema(&schema_for!(HistoricalBet), &out_dir);

    export_schema(&schema_for!(ExecuteMsg), &out_dir);
    export_schema(&schema_for!(QueryMsg), &out_dir);
    export_schema(&schema_for!(ConfigResponse), &out_dir);
    export_schema(&schema_for!(HistoricalBetResponse), &out_dir);
    export_schema(&schema_for!(OngoingBetResponse), &out_dir);
    export_schema(&schema_for!(AddrPendingBetsResponse), &out_dir);
    export_schema(&schema_for!(PendingBetResponse), &out_dir);
    export_schema(&schema_for!(TotalPendingBetsResponse), &out_dir);
    export_schema(&schema_for!(AssetFilter), &out_dir);
    export_schema(&schema_for!(LiquidationFilter), &out_dir);
    export_schema(&schema_for!(PendingBetsSort), &out_dir);
    export_schema(&schema_for!(PendingBetsFilter), &out_dir);
}
