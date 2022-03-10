use cosmwasm_std::{OverflowError, StdError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("[980]: {0}")]
    OverflowError(#[from] OverflowError),

    #[error("[990]: {0}")]
    Std(#[from] StdError),

    #[error("[1000]: Validation error: {message}")]
    ValidationErr { message: String },

    #[error("[1001]: Unauthorized")]
    Unauthorized {},

    #[error("[1002]: Responder amount or denom mismatch")]
    ResponderAssetMismatch {},

    #[error("[1003]: You are not allowed to play vs yourself")]
    ForbiddenToPlayVSYourself {},

    #[error("[1004]: Only bet creator allowed to resolve bet")]
    OnlyBetCreatorAllowedToResolve {},

    #[error("[1005]: Signatures mismatch")]
    SignatureMismatch {},

    #[error("[1006]: Bet creator cannot liquidate himself")]
    ForbiddenForBetCreatorToLiquidateHimself {},

    #[error("[1007]: Bet is not liquidatable yet")]
    BetIsNotLiquidatableYet {},

    #[error("[1008]: Responder liquidation gap is not passed yet")]
    ResponderLiquidationGapIsNotPassedYet {},

    #[error("[1009]: Execute this method without providing any funds")]
    ExecuteWithoutFunds {},

    #[error("[1010]: This game was either canceled or accepted by another player")]
    BetWasCancledOrAccepted {},

    #[error("[1011]: This game has already been accepted")]
    GameWasAlreadyAccepted {},

    #[error("[1012]: This game has already been liquidated")]
    GameWasAlreadyLiquidated {},

    #[error("[1013]: This game has already been resolved")]
    GameWasAlreadyResolved {},
}
