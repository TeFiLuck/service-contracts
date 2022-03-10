use cosmwasm_std::{
    Addr, BankMsg, Coin, CosmosMsg, Decimal, OverflowError, QuerierWrapper, StdError, StdResult,
    Uint128,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use terra_cosmwasm::TerraQuerier;

static DECIMAL_FRACTION: Uint128 = Uint128::new(1_000_000_000_000_000_000u128);

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Asset {
    pub denom: String,
    pub amount: Uint128,
}

impl Asset {
    pub fn from_coins(coins: Vec<Coin>) -> StdResult<Self> {
        if coins.len() != 1 {
            return Err(StdError::generic_err(
                "provide only one coin for playing in transaction",
            ));
        }

        let coin = coins[0].clone();
        Ok(coin.into())
    }

    pub fn checked_add(&mut self, other: &Asset) -> Result<&mut Self, OverflowError> {
        self.amount = self.amount.checked_add(other.amount)?;
        Ok(self)
    }

    pub fn checked_sub(&mut self, other: &Asset) -> Result<&mut Self, OverflowError> {
        self.amount = self.amount.checked_sub(other.amount)?;
        Ok(self)
    }

    pub fn take_percent(&self, percent: u8) -> Result<Self, StdError> {
        let amount = self
            .amount
            .checked_div(Uint128::new(100))?
            .checked_mul(Uint128::new(percent.into()))?;

        Ok(Asset {
            denom: self.denom.clone(),
            amount,
        })
    }

    pub fn into_bank_msg(
        &mut self,
        querier: &QuerierWrapper,
        receiver: &Addr,
    ) -> StdResult<CosmosMsg> {
        Ok(CosmosMsg::Bank(BankMsg::Send {
            to_address: receiver.to_string(),
            amount: vec![self.deduct_tax(querier)?],
        }))
    }

    fn deduct_tax(&mut self, querier: &QuerierWrapper) -> StdResult<Coin> {
        if self.denom != "uluna" {
            let terra_querier = TerraQuerier::new(querier);
            let tax_rate: Decimal = (terra_querier.query_tax_rate()?).rate;
            let tax_cap: Uint128 = (terra_querier.query_tax_cap(self.denom.clone())?).cap;

            let tax = std::cmp::min(
                self.amount.checked_sub(self.amount.multiply_ratio(
                    DECIMAL_FRACTION,
                    DECIMAL_FRACTION * tax_rate + DECIMAL_FRACTION,
                ))?,
                tax_cap,
            );

            self.amount = self.amount.checked_sub(tax)?;
        }

        Ok(Coin {
            denom: self.denom.clone(),
            amount: self.amount,
        })
    }
}

impl From<Coin> for Asset {
    fn from(coin: Coin) -> Self {
        Asset {
            denom: coin.denom,
            amount: coin.amount,
        }
    }
}
