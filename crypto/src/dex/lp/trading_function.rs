use anyhow::{anyhow, Result};
use penumbra_proto::{core::dex::v1alpha1 as pb, DomainType};
use serde::{Deserialize, Serialize};

use crate::dex::TradingPair;
use crate::fixpoint::U128x128;
use crate::Amount;
use crate::Value;

use super::Reserves;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(try_from = "pb::TradingFunction", into = "pb::TradingFunction")]
pub struct TradingFunction {
    pub component: BareTradingFunction,
    pub pair: TradingPair,
}

impl TradingFunction {
    pub(super) fn new(pair: TradingPair, fee: u32, p: Amount, q: Amount) -> Self {
        Self {
            component: BareTradingFunction::new(fee, p, q),
            pair,
        }
    }

    /// Fills a trade of an input value against this position, returning the
    /// unfilled amount of the input asset, the updated reserves, and the output
    /// amount.
    ///
    /// Errors if the asset type of the input does not match either end of this
    /// `TradingFunction`'s `TradingPair`.
    pub fn fill(
        &self,
        input: Value,
        reserves: &Reserves,
    ) -> anyhow::Result<(Value, Reserves, Value)> {
        if input.asset_id == self.pair.asset_1() {
            let (unfilled, new_reserves, output) = self.component.fill(input.amount, reserves);
            Ok((
                Value {
                    amount: unfilled,
                    asset_id: self.pair.asset_1(),
                },
                new_reserves,
                Value {
                    amount: output,
                    asset_id: self.pair.asset_2(),
                },
            ))
        } else if input.asset_id == self.pair.asset_2() {
            let flipped_reserves = reserves.flip();
            let (unfilled, new_reserves, output) =
                self.component.flip().fill(input.amount, &flipped_reserves);
            Ok((
                Value {
                    amount: unfilled,
                    asset_id: self.pair.asset_2(),
                },
                new_reserves.flip(),
                Value {
                    amount: output,
                    asset_id: self.pair.asset_1(),
                },
            ))
        } else {
            Err(anyhow!(
                "input asset id {:?} did not match either end of trading pair {:?}",
                input.asset_id,
                self.pair
            ))
        }
    }
}

impl TryFrom<pb::TradingFunction> for TradingFunction {
    type Error = anyhow::Error;

    fn try_from(phi: pb::TradingFunction) -> Result<Self, Self::Error> {
        Ok(Self {
            component: phi
                .component
                .ok_or_else(|| anyhow::anyhow!("missing BareTradingFunction"))?
                .try_into()?,
            pair: phi
                .pair
                .ok_or_else(|| anyhow::anyhow!("missing TradingPair"))?
                .try_into()?,
        })
    }
}

impl From<TradingFunction> for pb::TradingFunction {
    fn from(phi: TradingFunction) -> Self {
        Self {
            component: Some(phi.component.into()),
            pair: Some(phi.pair.into()),
        }
    }
}

impl DomainType for TradingFunction {
    type Proto = pb::TradingFunction;
}

/// The data describing a trading function.
///
/// This implicitly treats the trading function as being between assets 1 and 2,
/// without specifying what those assets are, to avoid duplicating data (each
/// asset ID alone is twice the size of the trading function). Which assets correspond
/// to asset 1 and 2 is given by the canonical ordering of the pair.
///
/// The trading function `phi(R) = p*R_1 + q*R_2` is a CFMM with a constant-sum,
/// and a fee (`0 <= fee < 10_000`) expressed in basis points.
///
/// The valuations (`p`, `q`) for each asset inform the rate (or price) at which these
/// assets trade against each other.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(try_from = "pb::BareTradingFunction", into = "pb::BareTradingFunction")]
pub struct BareTradingFunction {
    /// The fee, expressed in basis points.
    ///
    /// The fee percentage of the trading function (`gamma`) is normalized
    /// according to its maximum value (10_000 bps, i.e. 100%):
    /// `gamma = (10_000 - fee) / 10_000`
    pub fee: u32,
    /// The valuation for the first asset of the pair, according to canonical ordering.
    pub p: Amount,
    /// The valuation for the second asset of the pair, according to canonical ordering.
    pub q: Amount,
}

impl BareTradingFunction {
    pub fn new(fee: u32, p: Amount, q: Amount) -> Self {
        Self { fee, p, q }
    }

    pub fn flip(&self) -> Self {
        Self {
            fee: self.fee,
            p: self.q,
            q: self.p,
        }
    }

    /// Fills a trade of asset 1 to asset 2 against the given reserves,
    /// returning the unfilled amount of asset 1, the updated reserves, and the
    /// output amount of asset 2.
    pub fn fill(&self, delta_1: Amount, reserves: &Reserves) -> (Amount, Reserves, Amount) {
        // We distinguish two cases, which only differ in their rounding
        // behavior.
        //
        // If the desired fill is less than the original reserves, we want to
        // work "forward" from the input amount `delta_1` to the output amount
        // `lambda_2`, consuming exactly `delta_1` and rounding `lambda_2`
        // (down, so that we're burning the rounding error).
        //
        // If the desired fill is greater than the original reserves, however,
        // we want to work "backward" from the available reserves `R_2` (the
        // "max fill") to the input amount `delta_1`, producing exactly
        // `lambda_2 = R_2` output and rounding `delta_1` (up, so that we're
        // burning the rounding error).
        //
        // We want to be sure that in either case, we only round once, and derive
        // other quantities exactly from the rounded quantity. This ensures
        // conservation of value.
        //
        // This also ensures that we cleanly fill the position, rather than
        // leaving some dust amount of reserves in it. Otherwise, we might try
        // executing against it again on a subsequent iteration, even though it
        // was essentially filled.

        // The trade output `lambda_2` is given by `effective_price * delta_1`, however, to avoid
        // rounding loss, we prefer to first compute the numerator `(gamma * delta_1 * q)`, and then
        // perform division.
        let tentative_lambda_2 = self
            .effective_price_with_precompute(delta_1.into(), 1u64.into())
            .unwrap();

        if tentative_lambda_2 <= reserves.r2.into() {
            // Observe that for the case when `tentative_lambda_2` equals
            // `reserves.r1`, rounding it down does not change anything since
            // `reserves.r1` is integral. Therefore `reserves.r1 - lambda_2 >= 0`.
            let lambda_2: Amount = tentative_lambda_2.round_down().try_into().unwrap();
            let new_reserves = Reserves {
                r1: reserves.r1 + delta_1,
                r2: reserves.r2 - lambda_2,
            };
            (0u64.into(), new_reserves, lambda_2)
        } else {
            let r2: U128x128 = reserves.r2.into();
            // In this case, we don't have enough reserves to completely execute
            // the fill. So we know that `lambda_2 = r2` or that the output will
            // consist of all the reserves available.
            //
            // We must work backwards to infer what `delta_1` (input) correspond
            // exactly to a fill of `lambda_2 = r2`.
            //
            // Normally, we would have:
            //
            // lambda_2 = effective_price * delta_1
            // since lambda_2 = r2, we have:
            //
            // r2 = effective_price * delta_1, and since effective_price != 0:
            // delta_1 = r2 * effective_price^-1
            // since effective_price = (p/(q*gamma)), we have:
            // delta_1 = r2 * q * gamma * p^-1
            //
            // We burn the rouding error by apply `ceil` to delta_1:
            //
            // delta_1_star = Ceil(delta_1)
            let p = U128x128::from(self.p);
            let q = U128x128::from(self.q);
            let gamma = self.gamma();
            let numerator = (gamma * q).unwrap();
            let numerator = (r2 * numerator).unwrap();

            let fillable_delta_1 = numerator.checked_div(&p).unwrap();

            let fillable_delta_1_exact: Amount = fillable_delta_1.round_up().try_into().unwrap();

            // How to show that: `unfilled_amount >= 0`:
            // In this branch, we have:
            //      delta_1 * effective_price > R_2, in other words:
            //      delta_1 * (p/q)*(1/gamma) > R_2
            //  <=> delta_1 > R_2 * (effective_price)^-1, in other words:
            //      delta_1 > R_2 * (q*gamma)/p
            //
            //  fillable_delta_1_exact = ceil(RHS) is integral (rounded), and
            //  delta_1 is integral by definition.
            //
            //  Therefore, we have:
            //
            //  delta_1 > fillable_delta_1_exact, or in other words:
            //
            //  unfilled_amount > 0.
            let unfilled_amount = delta_1 - fillable_delta_1_exact;

            let new_reserves = Reserves {
                r1: reserves.r1 + fillable_delta_1_exact,
                r2: 0u64.into(),
            };
            (unfilled_amount, new_reserves, reserves.r2)
        }
    }

    /// Returns a byte key for this trading function with the property that the
    /// lexicographic ordering on byte keys is the same as ordering the
    /// corresponding trading functions by effective price.
    ///
    /// This allows trading functions to be indexed by price using a key-value store.
    pub fn effective_price_key_bytes(&self) -> [u8; 32] {
        self.effective_price().to_bytes()
    }

    /// Returns the effective price for the trading function inclusive of fees.
    ///
    /// The effective price is the exchange rate between asset 1 and asset 2, after
    /// applying the fee discount factor (aka. gamma).
    ///
    /// A lower effective price means than one gets more of asset 2 per asset 1.
    /// A higher fee means a lower discount factor, which leads to a higher price.
    pub fn effective_price(&self) -> U128x128 {
        self.effective_price_with_precompute(1u64.into(), 1u64.into())
            .expect("gamma, q != 0")
    }

    /// Applies the effective price formula with extra-parameters for the numerator
    /// and denominator. This is useful to conserve precision by operating multiplications
    /// before the checked division is applied.
    pub fn effective_price_with_precompute(
        &self,
        num: U128x128,
        denom: U128x128,
    ) -> Result<U128x128> {
        let p = U128x128::from(self.p);
        let q = U128x128::from(self.q);

        let numerator = (num * p).ok_or_else(|| anyhow!("numerator overflow"))?;
        let denominator = (q * self.gamma()).expect("gamma <= 1, so no overflow is possible");
        let denominator = (denominator * denom).ok_or_else(|| anyhow!("denominator overflow"))?;
        numerator
            .checked_div(&denominator)
            .ok_or_else(|| anyhow!("failed to perform checked division"))
    }

    /// Returns `gamma` i.e. the discount factor of the fee.
    /// The fee is expressed in basis points (0 <= fee < 10_000), where 10_000 bps = 100%.
    ///
    /// ## Examples:
    ///     * A fee of 0% (0 bps) results in a discount factor of 1.
    ///     * A fee of 30 bps (30 bps) results in a discount factor of 0.997.
    ///     * A fee of 100% (10_000bps) results in a discount factor of 0.
    pub fn gamma(&self) -> U128x128 {
        (U128x128::from(10_000 - self.fee) / U128x128::from(10_000u64)).expect("10_000 != 0")
    }

    /// Returns the composition of two trading functions.
    pub fn compose(&self, phi: BareTradingFunction) -> BareTradingFunction {
        let fee = self.fee * phi.fee;
        let r1 = self.p * phi.p;
        let r2 = self.q * phi.q;
        BareTradingFunction::new(fee, r1, r2)
    }
}

impl DomainType for BareTradingFunction {
    type Proto = pb::BareTradingFunction;
}

impl TryFrom<pb::BareTradingFunction> for BareTradingFunction {
    type Error = anyhow::Error;

    fn try_from(value: pb::BareTradingFunction) -> Result<Self, Self::Error> {
        Ok(Self {
            fee: value.fee,
            p: value
                .p
                .ok_or_else(|| anyhow::anyhow!("missing p"))?
                .try_into()?,
            q: value
                .q
                .ok_or_else(|| anyhow::anyhow!("missing q"))?
                .try_into()?,
        })
    }
}

impl From<BareTradingFunction> for pb::BareTradingFunction {
    fn from(value: BareTradingFunction) -> Self {
        Self {
            fee: value.fee,
            p: Some(value.p.into()),
            q: Some(value.q.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// Test that effective prices are encoded in a way that preserves their
    /// numerical ordering. Numerical ordering should transfer over lexicographic order
    /// of the encoded prices.
    fn test_trading_function_to_bytes() {
        let btf = BareTradingFunction {
            fee: 0,
            p: 2_000_000u32.into(),
            q: 1_000_000u32.into(),
        };

        assert_eq!(btf.gamma(), U128x128::from(1u64));
        assert_eq!(
            btf.effective_price(),
            U128x128::ratio(btf.p, btf.q).unwrap()
        );
        let bytes1 = btf.effective_price_key_bytes();

        let btf = BareTradingFunction {
            fee: 100,
            p: 2_000_000u32.into(),
            q: 1_000_000u32.into(),
        };

        // Compares the `BareTradingFunction::gamma` to a scaled ratio (10^4)
        let gamma_term =
            U128x128::ratio::<Amount>(99_000_000u64.into(), 100_000_000u64.into()).unwrap();
        assert_eq!(btf.gamma(), gamma_term,);

        let price_without_fee = U128x128::ratio(btf.p, btf.q).unwrap();
        let reciprocal = (U128x128::from(1u64) / gamma_term).unwrap();
        let price_with_fee = (price_without_fee * reciprocal).unwrap();

        assert_eq!(btf.effective_price(), price_with_fee);
        let bytes2 = btf.effective_price_key_bytes();

        // Assert that the effective price ordering (cheaper is better) matches the
        // encoded lexicographic ordering.
        //
        // The cheaper price (bytes1) vs. the more expensive price (bytes2)

        assert!(bytes2 > bytes1);
    }

    #[test]
    /// Test that filling a position follows the asset conservation law,
    /// meaning that the R + Delta = R + Lambda
    ///
    /// There is two branches of the `BareTradingFunction::fill` method that we
    /// want to exercise. The first one is executed when there are enough reserves
    /// available to perform the fill.
    ///
    /// The second case, is when the output is constrained by the available reserves.
    fn fill_conserves_value() {
        let btf = BareTradingFunction {
            fee: 0,
            p: 1_u32.into(),
            q: 3_u32.into(),
        };

        // First, we want to test asset conservations in the case of a partial fill:
        // D_1 = 10,000,000
        // D_2 = 0
        //
        // price: p/q = 1/3, so you get 1 unit of asset 2 for 3 units of asset 1.
        //
        // L_1 = 0
        // L_2 = 3_333_333 (D_1/3)

        let old_reserves = Reserves {
            r1: 1_000_000u64.into(),
            r2: 100_000_000u64.into(),
        };

        let delta_1 = 10_000_000u64.into();
        let delta_2 = 0u64.into();
        let (lambda_1, new_reserves, lambda_2) = btf.fill(delta_1, &old_reserves);
        // Conservation of value:
        assert_eq!(old_reserves.r1 + delta_1, new_reserves.r1 + lambda_1);
        assert_eq!(old_reserves.r2 + delta_2, new_reserves.r2 + lambda_2);

        // Exact amount checks:
        assert_eq!(lambda_1, 0u64.into());
        assert_eq!(lambda_2, 3_333_333u64.into());

        // Here we test trying to swap or more output than what is available in
        // the reserves:
        // lambda_1 = delta_1/3
        // lambda_2 = r2
        let old_reserves = Reserves {
            r1: 1_000_000u64.into(),
            r2: 100_000_000u64.into(),
        };
        let delta_1 = 600_000_000u64.into();
        let delta_2 = 0u64.into();

        let (lambda_1, new_reserves, lambda_2) = btf.fill(delta_1, &old_reserves);
        // Conservation of value:
        assert_eq!(old_reserves.r1 + delta_1, new_reserves.r1 + lambda_1);
        assert_eq!(old_reserves.r2 + delta_2, new_reserves.r2 + lambda_2);

        // Exact amount checks:
        assert_eq!(lambda_1, 300_000_000u64.into());
        assert_eq!(lambda_2, old_reserves.r2);
        assert_eq!(new_reserves.r2, 0u64.into());
    }

    #[test]
    fn fill_bad_rounding() {
        let btf = BareTradingFunction {
            fee: 0,
            p: 12u32.into(),
            q: 10u32.into(),
        };

        let old_reserves = Reserves {
            r1: 0u64.into(),
            r2: 120u64.into(),
        };

        let delta_1 = 100u64.into();
        let (lambda_1, new_reserves, lambda_2) = btf.fill(delta_1, &old_reserves);

        // Conservation of value:
        assert_eq!(old_reserves.r1 + delta_1, new_reserves.r1 + lambda_1);
        assert_eq!(old_reserves.r2 + 0u64.into(), new_reserves.r2 + lambda_2);
        // Exact amount checks:
        assert_eq!(lambda_1, 0u64.into());
        assert_eq!(lambda_2, 120u64.into());
    }
}
