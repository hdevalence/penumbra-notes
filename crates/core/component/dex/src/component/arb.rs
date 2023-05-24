use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use penumbra_chain::component::StateReadExt;
use penumbra_crypto::{asset, Value};
use penumbra_storage::{StateDelta, StateWrite};
use tracing::instrument;

use crate::SwapExecution;

use super::{
    router::{RouteAndFill, RoutingParams},
    StateWriteExt,
};

#[async_trait]
pub trait Arbitrage: StateWrite + Sized {
    /// Attempts to extract as much as possible of the `arb_token` from the available
    /// liquidity positions, and returns the amount of `arb_token` extracted.
    #[instrument(skip(self, arb_token, fixed_candidates))]
    async fn arbitrage(
        self: &mut Arc<Self>,
        arb_token: asset::Id,
        fixed_candidates: Vec<asset::Id>,
    ) -> Result<Value>
    where
        Self: 'static,
    {
        tracing::debug!(?arb_token, ?fixed_candidates, "beginning arb search");

        // Work in a new `StateDelta`, so we can transactionally apply any state
        // changes, and roll them back if we fail (e.g., if for some reason we
        // discover at the end that the arb wasn't profitable).
        let mut this = Arc::new(StateDelta::new(self.clone()));

        // TODO: Build an extended candidate set with:
        // - both ends of all trading pairs for which there were swaps in the block
        // - both ends of all trading pairs for which positions were opened
        let params = RoutingParams {
            max_hops: 5,
            price_limit: Some(1u64.into()),
            fixed_candidates: Arc::new(fixed_candidates),
        };

        // Create a flash-loan 2^64 of the arb token to ourselves.
        let flash_loan = Value {
            asset_id: arb_token,
            amount: u64::MAX.into(),
        };

        let (output, unfilled_input) = this
            .route_and_fill(arb_token, arb_token, flash_loan.amount, params)
            .await?;

        // Because we're trading the arb token to itself, the total output is the
        // output from the route-and-fill, plus the unfilled input.
        let total_output = output + unfilled_input;

        // Now "repay" the flash loan by subtracting it from the total output.
        let Some(arb_profit) = total_output.checked_sub(&flash_loan.amount) else {
            // This shouldn't happen, but because route-and-fill prioritizes
            // guarantees about forward progress over precise application of
            // price limits, it technically could occur.
            tracing::debug!("mis-estimation in route-and-fill led to unprofitable arb, discarding");
            return Ok(Value { amount: 0u64.into(), asset_id: arb_token });
        };

        if arb_profit == 0u64.into() {
            // If we didn't make any profit, we don't need to do anything,
            // and we can just discard the state delta entirely.
            tracing::debug!("found 0-profit arb, discarding");
            return Ok(Value {
                amount: 0u64.into(),
                asset_id: arb_token,
            });
        }

        tracing::debug!(
            ?arb_profit,
            "successfully arbitraged positions, burning profit"
        );

        // TODO: this is a bit nasty, can it be simplified?
        // should this even be done "inside" the method, or all the way at the top?
        let (self2, cache) = Arc::try_unwrap(this)
            .map_err(|_| ())
            .expect("no more outstanding refs to state after routing")
            .flatten();
        std::mem::drop(self2);
        // Now there is only one reference to self again
        let mut self_mut = Arc::get_mut(self).expect("self was unique ref");
        cache.apply_to(&mut self_mut);

        // Finally, record the arb execution in the state:
        let traces: im::Vector<Vec<Value>> = self_mut
            .object_get("trade_traces")
            .ok_or_else(|| anyhow::anyhow!("missing swap execution in object store2"))?;
        let height = self_mut.get_block_height().await?;
        self_mut.set_arb_execution(
            height,
            SwapExecution {
                traces: traces.into_iter().collect(),
                input: Value {
                    asset_id: arb_token,
                    amount: flash_loan
                        .amount
                        .checked_sub(&unfilled_input)
                        .unwrap_or_default(),
                },
                output: Value {
                    amount: arb_profit,
                    asset_id: arb_token,
                },
            },
        );

        return Ok(Value {
            amount: arb_profit,
            asset_id: arb_token,
        });
    }
}

impl<T: StateWrite> Arbitrage for T {}
