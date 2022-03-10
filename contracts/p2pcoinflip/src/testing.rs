use cosmwasm_std::testing::{mock_env, mock_info};
use cosmwasm_std::{
    attr, Addr, Api, BankMsg, Coin, CosmosMsg, Decimal, DepsMut, Response, SubMsg, Timestamp,
    Uint128, Uint64,
};

use crate::commands::{liquidate_bet, place_bet, resolve_bet, respond_bet, withdraw_pending_bet};
use crate::contract::instantiate;
use crate::mock_querier::{mock_dependencies, mock_env_custom};
use crate::msg::{InstantiateCoinLimitMsg, InstantiateMsg};
use crate::state::{
    load_historical_bets, load_ongoing_bet, load_pending_bets, load_pending_bets_count, FlipSide,
    GameOutcome, HistoricalBet, OngoingBet, PendingBet,
};
use crate::ContractError;
use tefiluck::asset::Asset;

const MOCK_SIGNATURE: &'static str =
    "84dbedafb3b595f6d2d1c9719520f0e0a19fabd953afd4e5befc22a8fd0d7510";
const MOCK_PASSPHRASE: &'static str = "0_tefiluck";

fn proper_instantiate(deps: DepsMut) -> Result<Response, ContractError> {
    let msg = InstantiateMsg {
        treasury: "addr0000".to_string(),
        treasury_tax_percent: 1,
        max_bets_by_addr: 50,
        min_bet_amounts: vec![InstantiateCoinLimitMsg {
            denom: "uusd".to_string(),
            min_amount: 1000000u64,
        }],
        min_blocks_until_liquidation: 100,
        max_blocks_until_liquidation: 500,
        blocks_for_responder_liquidation: 20,
        bet_responder_liquidation_percent: 90,
        bet_liquidator_percent: 7,
        treasury_liquidation_percent: 3,
        historical_bets_max_storage_size: 100,
        historical_bets_clear_batch_size: 10,
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);

    instantiate(deps, env, info, msg)
}

#[test]
fn test_proper_initialization() {
    let mut deps = mock_dependencies(&[]);

    assert_eq!(proper_instantiate(deps.as_mut()).is_ok(), true,);
}

#[test]
fn test_invalid_initialization() {
    let mut deps = mock_dependencies(&[]);

    // test min_bet_amounts validation
    let msg = InstantiateMsg {
        treasury: "addr0000".to_string(),
        treasury_tax_percent: 1,
        max_bets_by_addr: 50,
        min_bet_amounts: vec![],
        min_blocks_until_liquidation: 100,
        max_blocks_until_liquidation: 500,
        blocks_for_responder_liquidation: 20,
        bet_responder_liquidation_percent: 90,
        bet_liquidator_percent: 7,
        treasury_liquidation_percent: 3,
        historical_bets_max_storage_size: 100,
        historical_bets_clear_batch_size: 10,
    };

    let env = mock_env();
    let info = mock_info("addr0000", &[]);
    match instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err() {
        ContractError::ValidationErr { message } => assert_eq!(
            message,
            "Config validation: min_bet_amounts must be a non-empty list".to_string()
        ),
        _ => panic!("Must return validation err"),
    };

    // test treasury_tax_percent max limit
    let msg = InstantiateMsg {
        treasury: "addr0000".to_string(),
        treasury_tax_percent: 11,
        max_bets_by_addr: 50,
        min_bet_amounts: vec![InstantiateCoinLimitMsg {
            denom: "uusd".to_string(),
            min_amount: 1000000u64,
        }],
        min_blocks_until_liquidation: 100,
        max_blocks_until_liquidation: 500,
        blocks_for_responder_liquidation: 20,
        bet_responder_liquidation_percent: 90,
        bet_liquidator_percent: 7,
        treasury_liquidation_percent: 3,
        historical_bets_max_storage_size: 100,
        historical_bets_clear_batch_size: 10,
    };

    match instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err() {
        ContractError::ValidationErr { message } => assert_eq!(
            message,
            "Config validation: treasury percent must be less than 10".to_string()
        ),
        _ => panic!("Must return validation err"),
    };

    // test liquidation percent
    let msg = InstantiateMsg {
        treasury: "addr0000".to_string(),
        treasury_tax_percent: 1,
        max_bets_by_addr: 50,
        min_bet_amounts: vec![InstantiateCoinLimitMsg {
            denom: "uusd".to_string(),
            min_amount: 1000000u64,
        }],
        min_blocks_until_liquidation: 100,
        max_blocks_until_liquidation: 500,
        blocks_for_responder_liquidation: 20,
        bet_responder_liquidation_percent: 90,
        bet_liquidator_percent: 7,
        treasury_liquidation_percent: 4,
        historical_bets_max_storage_size: 100,
        historical_bets_clear_batch_size: 10,
    };

    match instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err() {
        ContractError::ValidationErr { message } => assert_eq!(
            message,
            "Config validation: liquidation percent must be equal to 100".to_string()
        ),
        _ => panic!("Must return validation err"),
    };

    // test min-max blocks until liquidation limits
    let msg = InstantiateMsg {
        treasury: "addr0000".to_string(),
        treasury_tax_percent: 1,
        max_bets_by_addr: 50,
        min_bet_amounts: vec![InstantiateCoinLimitMsg {
            denom: "uusd".to_string(),
            min_amount: 1000000u64,
        }],
        min_blocks_until_liquidation: 501,
        max_blocks_until_liquidation: 500,
        blocks_for_responder_liquidation: 20,
        bet_responder_liquidation_percent: 90,
        bet_liquidator_percent: 7,
        treasury_liquidation_percent: 3,
        historical_bets_max_storage_size: 100,
        historical_bets_clear_batch_size: 10,
    };

    match instantiate(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err() {
        ContractError::ValidationErr { message } => assert_eq!(
            message,
            "Config validation: min_blocks_until_liquidation must be less than max_blocks_until_liquidation".to_string()
        ),
        _ => panic!("Must return validation err")
    };
}

#[test]
fn test_place_bet() {
    let mut deps = mock_dependencies(&[]);

    let _ = proper_instantiate(deps.as_mut()).unwrap();

    let mut env = mock_env();
    env.block.time = Timestamp::from_nanos(100000);
    let info = mock_info(
        "addr0001",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::new(1000000u128),
        }],
    );

    let place_bet_response =
        place_bet(deps.as_mut(), env, info, MOCK_SIGNATURE.to_string(), 200).unwrap();

    let log_action = place_bet_response.attributes.get(0).expect("no log");
    assert_eq!(log_action, &attr("action", "place_bet"));

    let log_sender = place_bet_response.attributes.get(1).expect("no log");
    assert_eq!(log_sender, &attr("sender", "addr0001".to_string()),);

    let log_bet_id = place_bet_response.attributes.get(2).expect("no log");
    let bet_id = log_bet_id.value.clone();

    let mut pending_bets = load_pending_bets(&deps.storage, &Addr::unchecked("addr0001")).unwrap();
    let bet = pending_bets.find_by_id(&bet_id).unwrap();
    assert_eq!(
        bet,
        PendingBet {
            owner: deps.api.addr_canonicalize("addr0001").unwrap(),
            id: bet_id,
            signature: MOCK_SIGNATURE.to_string(),
            blocks_until_liquidation: 200,
            asset: Asset {
                denom: "uusd".to_string(),
                amount: Uint128::new(1000000u128),
            },
            created_at: Timestamp::from_nanos(100000),
        }
    );

    let bet_count = load_pending_bets_count(&deps.storage).unwrap();
    assert_eq!(Uint64::from(1u64), bet_count,)
}

#[test]
fn test_place_bet_validation_error() {
    let mut deps = mock_dependencies(&[]);

    let _ = proper_instantiate(deps.as_mut()).unwrap();

    let env = mock_env();

    let info = mock_info("addr0001", &[]);
    match place_bet(
        deps.as_mut(),
        env.clone(),
        info,
        MOCK_SIGNATURE.to_string(),
        200,
    )
    .unwrap_err()
    {
        ContractError::Std(cosmwasm_std::StdError::GenericErr { msg, .. }) => assert_eq!(
            msg,
            "provide only one coin for playing in transaction".to_string(),
        ),
        _ => panic!("no error"),
    }

    let info = mock_info(
        "addr0001",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::new(1000000u128),
        }],
    );

    match place_bet(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        MOCK_SIGNATURE.to_string(),
        10,
    )
    .unwrap_err()
    {
        ContractError::Std(cosmwasm_std::StdError::GenericErr { msg, .. }) => assert_eq!(
            msg,
            "blocks_before_liquidation must be higher than min allowed value".to_string(),
        ),
        _ => panic!("no error"),
    }

    match place_bet(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        MOCK_SIGNATURE.to_string(),
        10000,
    )
    .unwrap_err()
    {
        ContractError::Std(cosmwasm_std::StdError::GenericErr { msg, .. }) => assert_eq!(
            msg,
            "blocks_before_liquidation must be less than max allowed value".to_string(),
        ),
        _ => panic!("no error"),
    }

    let info = mock_info(
        "addr0001",
        &[Coin {
            denom: "pinkguyasset".to_string(),
            amount: Uint128::new(1u128),
        }],
    );
    match place_bet(
        deps.as_mut(),
        env.clone(),
        info,
        MOCK_SIGNATURE.to_string(),
        200,
    )
    .unwrap_err()
    {
        ContractError::Std(cosmwasm_std::StdError::GenericErr { msg, .. }) => {
            assert_eq!(msg, "coin limits for provided asset not found".to_string(),)
        }
        _ => panic!("no error"),
    }

    let info = mock_info(
        "addr0001",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::new(1u128),
        }],
    );
    match place_bet(
        deps.as_mut(),
        env.clone(),
        info,
        MOCK_SIGNATURE.to_string(),
        200,
    )
    .unwrap_err()
    {
        ContractError::Std(cosmwasm_std::StdError::GenericErr { msg, .. }) => assert_eq!(
            msg,
            "provided amount less than min limit for provided asset".to_string(),
        ),
        _ => panic!("no error"),
    }

    // place proper bet
    let info = mock_info(
        "addr0001",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::new(1000000u128),
        }],
    );
    let _ = place_bet(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        MOCK_SIGNATURE.to_string(),
        200,
    )
    .unwrap();

    //try to place same bet(with same id)
    match place_bet(
        deps.as_mut(),
        env.clone(),
        info,
        MOCK_SIGNATURE.to_string(),
        200,
    )
    .unwrap_err()
    {
        ContractError::Std(cosmwasm_std::StdError::GenericErr { msg, .. }) => {
            assert_eq!(msg, "bet with same id alreay exists".to_string(),)
        }
        _ => panic!("no error"),
    }
}

#[test]
fn test_respond_bet() {
    let mut deps = mock_dependencies(&[]);

    let _ = proper_instantiate(deps.as_mut()).unwrap();

    let mut env = mock_env();
    env.block.time = Timestamp::from_seconds(10000);
    let info = mock_info(
        "addr0001",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::new(1000000u128),
        }],
    );

    let pb = place_bet(
        deps.as_mut(),
        env.clone(),
        info,
        MOCK_SIGNATURE.to_string(),
        200,
    )
    .unwrap();

    let bet_count = load_pending_bets_count(&deps.storage).unwrap();
    assert_eq!(Uint64::from(1u64), bet_count,);

    let bet_id = pb.attributes.get(2).expect("no bet_id").value.clone();

    let info = mock_info(
        "addr0002",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::new(1000000u128),
        }],
    );

    let respond_bet_response = respond_bet(
        deps.as_mut(),
        env.clone(),
        info,
        Addr::unchecked("addr0001"),
        bet_id.clone(),
        0,
    )
    .unwrap();

    let log_action = respond_bet_response.attributes.get(0).expect("no log");
    assert_eq!(log_action, &attr("action", "respond_bet"),);

    let log_bet_id = respond_bet_response.attributes.get(1).expect("no log");
    assert_eq!(log_bet_id, &attr("bet_id", bet_id.clone()),);

    let log_sender = respond_bet_response.attributes.get(4).expect("no log");
    assert_eq!(
        log_sender,
        &attr("bet_responder", Addr::unchecked("addr0002".to_string())),
    );

    let mut pending_bets = load_pending_bets(&deps.storage, &Addr::unchecked("addr0001")).unwrap();
    assert_eq!(pending_bets.find_by_id(&bet_id).is_err(), true,);

    let ongoing_bet = load_ongoing_bet(&deps.storage, bet_id.clone()).unwrap();
    assert_eq!(
        ongoing_bet,
        OngoingBet {
            signature: MOCK_SIGNATURE.to_string(),
            bet_creator: Addr::unchecked("addr0001"),
            bet_responder: Addr::unchecked("addr0002"),
            responder_side: FlipSide::Heads,
            asset: Asset {
                denom: "uusd".to_string(),
                amount: Uint128::new(2000000u128),
            },
            started_at_block: env.block.height,
            blocks_until_liquidation: 200,
            liquidation_block: 12345 + 200,
            responder_liquidation_blocks_gap: 12345 + 200 + 20,
            created_at: Timestamp::from_seconds(10000),
        },
    );

    let bet_count = load_pending_bets_count(&deps.storage).unwrap();
    assert_eq!(Uint64::from(0u64), bet_count,)
}

#[test]
fn test_respond_bet_validation_error() {
    let mut deps = mock_dependencies(&[]);

    let _ = proper_instantiate(deps.as_mut()).unwrap();

    let env = mock_env();
    let info = mock_info(
        "addr0001",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::new(1000000u128),
        }],
    );

    let pb = place_bet(
        deps.as_mut(),
        env.clone(),
        info,
        MOCK_SIGNATURE.to_string(),
        200,
    )
    .unwrap();

    let bet_id = pb.attributes.get(2).expect("no bet_id").value.clone();

    let info = mock_info(
        "addr0002",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::new(1000000u128),
        }],
    );
    match respond_bet(
        deps.as_mut(),
        env.clone(),
        info,
        Addr::unchecked("addr0001"),
        "nf".to_string(),
        0,
    )
    .unwrap_err()
    {
        ContractError::BetWasCancledOrAccepted {} => assert_eq!(true, true),
        _ => panic!("no error"),
    }

    let info = mock_info(
        "addr0002",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::new(2000000u128),
        }],
    );
    match respond_bet(
        deps.as_mut(),
        env.clone(),
        info,
        Addr::unchecked("addr0001"),
        bet_id.clone(),
        0,
    )
    .unwrap_err()
    {
        ContractError::ResponderAssetMismatch {} => assert_eq!(true, true,),
        _ => panic!("no error"),
    }

    let info = mock_info(
        "addr0002",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::new(1000000u128),
        }],
    );
    match respond_bet(
        deps.as_mut(),
        env.clone(),
        info,
        Addr::unchecked("addr0001"),
        bet_id.clone(),
        2,
    )
    .unwrap_err()
    {
        ContractError::Std(cosmwasm_std::StdError::GenericErr { msg, .. }) => {
            assert_eq!(msg, "invalid flip side".to_string(),)
        }
        _ => panic!("no error"),
    }
}

fn create_valid_pending_bet(deps: DepsMut) -> String {
    let env = mock_env();
    let info = mock_info(
        "addr0001",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::new(1000000u128),
        }],
    );

    let pb = place_bet(
        deps,
        env.clone(),
        info.clone(),
        MOCK_SIGNATURE.to_string(),
        200,
    )
    .unwrap();

    pb.attributes.get(2).expect("no bet_id").value.clone()
}

fn create_valid_ongoing_bet(deps: DepsMut, bet_id: String) {
    let env = mock_env();
    let info = mock_info(
        "addr0002",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::new(1000000u128),
        }],
    );

    let _ = respond_bet(
        deps,
        env.clone(),
        info.clone(),
        Addr::unchecked("addr0001"),
        bet_id,
        0,
    )
    .unwrap();
}

#[test]
fn test_resolve_bet() {
    let mut deps = mock_dependencies(&[]);

    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );

    let _ = proper_instantiate(deps.as_mut()).unwrap();

    let bet_id = create_valid_pending_bet(deps.as_mut());
    create_valid_ongoing_bet(deps.as_mut(), bet_id.clone());

    let env = mock_env();
    let info = mock_info("addr0001", &[]);

    let response = resolve_bet(
        deps.as_mut(),
        env.clone(),
        info.clone(),
        bet_id.clone(),
        MOCK_PASSPHRASE.to_string(),
    )
    .unwrap();

    let msg_send_winner_amount = response.messages.get(0).expect("no message");
    assert_eq!(
        msg_send_winner_amount,
        &SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: "addr0002".to_string(),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: Uint128::new(1960396u128),
            }]
        }))
    );
    let msg_send_treasury_amount = response.messages.get(1).expect("no message");
    assert_eq!(
        msg_send_treasury_amount,
        &SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: "addr0000".to_string(),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: Uint128::new(19801u128),
            }]
        }))
    );

    assert_eq!(
        load_ongoing_bet(&deps.storage, bet_id.clone()).is_err(),
        true,
    );

    let historical_bet = load_historical_bets(&deps.storage)
        .unwrap()
        .get(0)
        .unwrap()
        .clone();
    assert_eq!(
        historical_bet,
        HistoricalBet {
            id: bet_id.clone(),
            owner: "addr0001".to_string(),
            responder: "addr0002".to_string(),
            winner: "addr0002".to_string(),
            liquidator: None,
            responder_side: FlipSide::Heads.u8(),
            asset: Asset {
                denom: "uusd".to_string(),
                amount: Uint128::new(2000000u128),
            },
            outcome: GameOutcome::Resolved,
            created_at: env.block.time.seconds(),
            completed_at: env.block.time.seconds(),
        }
    );
}

#[test]
fn test_resolve_bet_validation_error() {
    let mut deps = mock_dependencies(&[]);

    let _ = proper_instantiate(deps.as_mut()).unwrap();

    let bet_id = create_valid_pending_bet(deps.as_mut());
    create_valid_ongoing_bet(deps.as_mut(), bet_id.clone());

    let env = mock_env();

    let info = mock_info(
        "addr0001",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::new(1u128),
        }],
    );
    match resolve_bet(
        deps.as_mut(),
        env.clone(),
        info,
        bet_id.clone(),
        MOCK_PASSPHRASE.to_string(),
    )
    .unwrap_err()
    {
        ContractError::ExecuteWithoutFunds {} => assert_eq!(true, true,),
        _ => panic!("no error"),
    }

    let info = mock_info("addr0009", &[]);
    match resolve_bet(
        deps.as_mut(),
        env.clone(),
        info,
        bet_id.clone(),
        MOCK_PASSPHRASE.to_string(),
    )
    .unwrap_err()
    {
        ContractError::OnlyBetCreatorAllowedToResolve {} => assert_eq!(true, true,),
        _ => panic!("no error"),
    }

    let info = mock_info("addr0001", &[]);
    match resolve_bet(
        deps.as_mut(),
        env.clone(),
        info,
        bet_id.clone(),
        "pinkguy".to_string(),
    )
    .unwrap_err()
    {
        ContractError::SignatureMismatch {} => assert_eq!(true, true,),
        _ => panic!("no error"),
    }
}

#[test]
fn test_liquidate_bet() {
    let mut deps = mock_dependencies(&[]);

    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );

    let _ = proper_instantiate(deps.as_mut()).unwrap();

    let bet_id = create_valid_pending_bet(deps.as_mut());
    create_valid_ongoing_bet(deps.as_mut(), bet_id.clone());

    let env = mock_env_custom(13_345);
    let info = mock_info("addr0003", &[]);

    let response = liquidate_bet(deps.as_mut(), env.clone(), info, bet_id.clone()).unwrap();

    let msg_send_responder_amount = response.messages.get(0).expect("no message");
    assert_eq!(
        msg_send_responder_amount,
        &SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: "addr0002".to_string(),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: Uint128::new(1782178u128),
            }]
        }))
    );

    let msg_send_liquidator_amount = response.messages.get(1).expect("no message");
    assert_eq!(
        msg_send_liquidator_amount,
        &SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: "addr0003".to_string(),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: Uint128::new(138613u128),
            }]
        }))
    );

    let msg_send_treasury_amount = response.messages.get(2).expect("no message");
    assert_eq!(
        msg_send_treasury_amount,
        &SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: "addr0000".to_string(),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: Uint128::new(59405u128),
            }]
        }))
    );

    assert_eq!(
        load_ongoing_bet(&deps.storage, bet_id.clone()).is_err(),
        true,
    );

    let historical_bet = load_historical_bets(&deps.storage)
        .unwrap()
        .get(0)
        .unwrap()
        .clone();
    assert_eq!(
        historical_bet,
        HistoricalBet {
            id: bet_id.clone(),
            owner: "addr0001".to_string(),
            responder: "addr0002".to_string(),
            winner: "addr0002".to_string(),
            liquidator: Some("addr0003".to_string()),
            responder_side: FlipSide::Heads.u8(),
            asset: Asset {
                denom: "uusd".to_string(),
                amount: Uint128::new(2000000u128),
            },
            outcome: GameOutcome::Liquidated,
            created_at: env.block.time.seconds(),
            completed_at: env.block.time.seconds(),
        }
    );
}

#[test]
fn test_liquidate_bet_validation_error() {
    let mut deps = mock_dependencies(&[]);

    let _ = proper_instantiate(deps.as_mut()).unwrap();

    let bet_id = create_valid_pending_bet(deps.as_mut());
    create_valid_ongoing_bet(deps.as_mut(), bet_id.clone());

    let env = mock_env_custom(13_345);

    let info = mock_info(
        "addr0003",
        &[Coin {
            denom: "uusd".to_string(),
            amount: Uint128::new(1u128),
        }],
    );
    match liquidate_bet(deps.as_mut(), env, info, bet_id.clone()).unwrap_err() {
        ContractError::ExecuteWithoutFunds {} => assert_eq!(true, true,),
        _ => panic!("no error"),
    };

    let env = mock_env_custom(12_400);
    let info = mock_info("addr0001", &[]);
    match liquidate_bet(deps.as_mut(), env, info, bet_id.clone()).unwrap_err() {
        ContractError::ForbiddenForBetCreatorToLiquidateHimself {} => assert_eq!(true, true,),
        _ => panic!("no error"),
    };

    let env = mock_env_custom(12_400);
    let info = mock_info("addr0003", &[]);
    match liquidate_bet(deps.as_mut(), env, info, bet_id.clone()).unwrap_err() {
        ContractError::BetIsNotLiquidatableYet {} => assert_eq!(true, true,),
        _ => panic!("no error"),
    };

    let env = mock_env_custom(12_564);
    let info = mock_info("addr0003", &[]);
    match liquidate_bet(deps.as_mut(), env, info, bet_id.clone()).unwrap_err() {
        ContractError::ResponderLiquidationGapIsNotPassedYet {} => assert_eq!(true, true,),
        _ => panic!("no error"),
    };
}

#[test]
fn test_withdraw_pending_bet() {
    let mut deps = mock_dependencies(&[]);

    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );

    let _ = proper_instantiate(deps.as_mut()).unwrap();

    let bet_id = create_valid_pending_bet(deps.as_mut());

    let bet_count = load_pending_bets_count(&deps.storage).unwrap();
    assert_eq!(Uint64::from(1u64), bet_count,);

    let info = mock_info("addr0001", &[]);
    let response = withdraw_pending_bet(deps.as_mut(), info, bet_id.clone()).unwrap();

    let bet_count = load_pending_bets_count(&deps.storage).unwrap();
    assert_eq!(Uint64::from(0u64), bet_count,);

    let msg_send_bet_owner_funds = response.messages.get(0).expect("no message");
    assert_eq!(
        msg_send_bet_owner_funds,
        &SubMsg::new(CosmosMsg::Bank(BankMsg::Send {
            to_address: "addr0001".to_string(),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: Uint128::new(990099u128),
            }]
        }))
    );

    let mut pending_bets = load_pending_bets(&deps.storage, &Addr::unchecked("addr0001")).unwrap();
    assert_eq!(pending_bets.find_by_id(&bet_id).is_err(), true,);
}
