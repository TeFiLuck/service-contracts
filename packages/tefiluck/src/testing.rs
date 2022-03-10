use crate::asset::Asset;
use crate::hash::calculate_sha256;
use crate::mock_querier::mock_dependencies;
use crate::querier::query_native_token_balance;
use cosmwasm_std::testing::MOCK_CONTRACT_ADDR;
use cosmwasm_std::{Addr, BankMsg, Coin, CosmosMsg, Decimal, Uint128};

#[test]
fn test_hash() {
    let input = "tefiluck";
    let hash = calculate_sha256(input);

    assert_eq!(
        hash,
        "b554718c4730292e410ee00fe7df5f6820045e526afaaba84a3f3ba2d94fccc9".to_string(),
    );
}

#[test]
fn test_balance_querier() {
    let deps = mock_dependencies(&[Coin {
        denom: "uluna".to_string(),
        amount: Uint128::from(666u128),
    }]);

    assert_eq!(
        query_native_token_balance(
            &deps.as_ref().querier,
            Addr::unchecked(MOCK_CONTRACT_ADDR),
            "uluna".to_string(),
        )
        .unwrap(),
        Uint128::from(666u128),
    )
}

#[test]
fn test_asset_from_coins() {
    let valid_coins = vec![Coin {
        denom: "uusd".to_string(),
        amount: Uint128::new(1u128),
    }];

    assert_eq!(
        Asset::from_coins(valid_coins).unwrap(),
        Asset {
            denom: "uusd".to_string(),
            amount: Uint128::new(1u128),
        },
    );

    assert_eq!(Asset::from_coins(vec![]).is_err(), true,);
}

#[test]
fn test_asset_checked_add_sub() {
    let mut asset = Asset {
        denom: "uusd".to_string(),
        amount: Uint128::new(1u128),
    };

    let another_asset = Asset {
        denom: "uusd".to_string(),
        amount: Uint128::new(1u128),
    };

    asset.checked_add(&another_asset).unwrap();
    assert_eq!(
        asset,
        Asset {
            denom: "uusd".to_string(),
            amount: Uint128::new(2u128),
        },
    );

    asset.checked_sub(&another_asset).unwrap();
    assert_eq!(
        asset,
        Asset {
            denom: "uusd".to_string(),
            amount: Uint128::new(1u128),
        },
    )
}

#[test]
fn test_asset_take_percent() {
    let asset = Asset {
        denom: "uusd".to_string(),
        amount: Uint128::new(100u128),
    };

    assert_eq!(
        asset.take_percent(10).unwrap(),
        Asset {
            denom: "uusd".to_string(),
            amount: Uint128::new(10u128),
        },
    )
}

#[test]
fn test_asset_bank_msg() {
    let mut deps = mock_dependencies(&[Coin {
        denom: "uusd".to_string(),
        amount: Uint128::from(666u128),
    }]);

    deps.querier.with_tax(
        Decimal::percent(1),
        &[(&"uusd".to_string(), &Uint128::from(1000000u128))],
    );

    let mut asset = Asset {
        denom: "uusd".to_string(),
        amount: Uint128::new(1000u128),
    };

    assert_eq!(
        asset
            .into_bank_msg(&deps.as_ref().querier, &Addr::unchecked("addr0000"))
            .unwrap(),
        CosmosMsg::Bank(BankMsg::Send {
            to_address: Addr::unchecked("addr0000").to_string(),
            amount: vec![Coin {
                denom: "uusd".to_string(),
                amount: Uint128::new(990u128),
            }],
        }),
    );
}
