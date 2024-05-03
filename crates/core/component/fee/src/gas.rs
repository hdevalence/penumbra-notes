use std::{
    iter::Sum,
    ops::{Add, AddAssign},
};

use serde::{Deserialize, Serialize};

use penumbra_num::Amount;
use penumbra_proto::{core::component::fee::v1 as pb, DomainType};

/// Represents the different resources that a transaction can consume,
/// for purposes of calculating multidimensional fees based on real
/// transaction resource consumption.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Gas {
    pub block_space: u64,
    pub compact_block_space: u64,
    pub verification: u64,
    pub execution: u64,
}

impl Gas {
    pub fn zero() -> Self {
        Self {
            block_space: 0,
            compact_block_space: 0,
            verification: 0,
            execution: 0,
        }
    }
}

impl Add for Gas {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            block_space: self.block_space + rhs.block_space,
            compact_block_space: self.compact_block_space + rhs.compact_block_space,
            verification: self.verification + rhs.verification,
            execution: self.execution + rhs.execution,
        }
    }
}

impl AddAssign for Gas {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs;
    }
}

impl Sum for Gas {
    fn sum<I: Iterator<Item = Gas>>(iter: I) -> Gas {
        iter.fold(Gas::zero(), |acc, x| acc + x)
    }
}

/// Expresses the price of each unit of gas in terms of the staking token.
///
/// These prices have an implicit denominator of 1,000 relative to the base unit
/// of the staking token, so gas price 1,000 times 1 unit of gas is 1 base unit
/// of staking token.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Default, Serialize, Deserialize)]
#[serde(try_from = "pb::GasPrices", into = "pb::GasPrices")]
pub struct GasPrices {
    pub block_space_price: u64,
    pub compact_block_space_price: u64,
    pub verification_price: u64,
    pub execution_price: u64,
}

impl GasPrices {
    pub fn zero() -> Self {
        Self::default()
    }

    /// Use these gas prices to calculate the fee for a given gas vector.
    pub fn fee(&self, gas: &Gas) -> Amount {
        Amount::from(
            (self.block_space_price * gas.block_space) / 1_000
                + (self.compact_block_space_price * gas.compact_block_space) / 1_000
                + (self.verification_price * gas.verification) / 1_000
                + (self.execution_price * gas.execution) / 1_000,
        )
    }
}

impl DomainType for GasPrices {
    type Proto = pb::GasPrices;
}

impl From<GasPrices> for pb::GasPrices {
    fn from(prices: GasPrices) -> Self {
        pb::GasPrices {
            block_space_price: prices.block_space_price,
            compact_block_space_price: prices.compact_block_space_price,
            verification_price: prices.verification_price,
            execution_price: prices.execution_price,
        }
    }
}

impl TryFrom<pb::GasPrices> for GasPrices {
    type Error = anyhow::Error;

    fn try_from(proto: pb::GasPrices) -> Result<Self, Self::Error> {
        Ok(GasPrices {
            block_space_price: proto.block_space_price,
            compact_block_space_price: proto.compact_block_space_price,
            verification_price: proto.verification_price,
            execution_price: proto.execution_price,
        })
    }
}
