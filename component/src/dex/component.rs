use std::collections::BTreeMap;
use std::sync::Arc;

// use crate::shielded_pool::StateReadExt as _;
use crate::Component;
use anyhow::{Context as _, Result};
use ark_ff::Zero;
use async_trait::async_trait;
use decaf377::Fr;
use penumbra_chain::{genesis, StateReadExt as _};
use penumbra_crypto::dex::lp::Reserves;
use penumbra_crypto::{
    asset,
    dex::{BatchSwapOutputData, TradingPair},
    MockFlowCiphertext, SwapFlow, Value, STAKING_TOKEN_ASSET_ID,
};
use penumbra_storage::{State, StateRead, StateTransaction, StateWrite};
use penumbra_transaction::{Action, Transaction};
use tendermint::abci;
use tracing::instrument;

use super::state_key;
use super::StubCpmm;

pub struct Dex {}

#[async_trait]
impl Component for Dex {
    #[instrument(name = "dex", skip(state, _app_state))]
    async fn init_chain(state: &mut StateTransaction, _app_state: &genesis::AppState) {
        // Hardcode some AMMs
        let gm = asset::REGISTRY.parse_unit("gm");
        let gn = asset::REGISTRY.parse_unit("gn");
        let penumbra = asset::REGISTRY.parse_unit("penumbra");

        state.set_stub_cpmm_reserves(
            &TradingPair::canonical_order_for((gm.id(), gn.id())).unwrap(),
            Reserves {
                r1: (10000 * 10u64.pow(gm.exponent().into())).into(),
                r2: (10000 * 10u64.pow(gn.exponent().into())).into(),
            },
        );

        state.set_stub_cpmm_reserves(
            &TradingPair::canonical_order_for((gm.id(), penumbra.id())).unwrap(),
            Reserves {
                r1: (10000 * 10u64.pow(gm.exponent().into())).into(),
                r2: (10000 * 10u64.pow(penumbra.exponent().into())).into(),
            },
        );

        state.set_stub_cpmm_reserves(
            &TradingPair::canonical_order_for((gn.id(), penumbra.id())).unwrap(),
            Reserves {
                r1: (10000 * 10u64.pow(gn.exponent().into())).into(),
                r2: (10000 * 10u64.pow(penumbra.exponent().into())).into(),
            },
        );
    }

    #[instrument(name = "dex", skip(_state, _begin_block))]
    async fn begin_block(_state: &mut StateTransaction, _begin_block: &abci::request::BeginBlock) {}

    #[instrument(name = "dex", skip(tx))]
    fn check_tx_stateless(tx: Arc<Transaction>) -> Result<()> {
        // It's important to reject all LP actions for now, to prevent
        // inflation / minting bugs until we implement all required checks
        // (e.g., minting tokens by withdrawing reserves we don't check)
        for action in tx.transaction_body.actions.iter() {
            match action {
                Action::PositionOpen { .. }
                | Action::PositionClose { .. }
                | Action::PositionWithdraw { .. }
                | Action::PositionRewardClaim { .. } => {
                    return Err(anyhow::anyhow!("lp actions not supported yet"));
                }
                Action::Swap(swap) => {
                    // Check swap proof
                    swap.proof
                        .verify(
                            // TODO: no value commitments until flow encryption is available
                            // so we pass placeholder values here, the proof doesn't check these right now
                            // and will fail when checking is re-enabled.
                            Value {
                                amount: 0u64.into(),
                                asset_id: *STAKING_TOKEN_ASSET_ID,
                            }
                            .commit(Fr::zero()),
                            Value {
                                amount: 0u64.into(),
                                asset_id: *STAKING_TOKEN_ASSET_ID,
                            }
                            .commit(Fr::zero()),
                            swap.body.fee_commitment,
                            swap.body.swap_nft.note_commitment,
                            swap.body.swap_nft.ephemeral_key,
                        )
                        .context("A swap proof did not verify")?;

                    // TODO: are any other checks necessary?

                    return Ok(());
                }
                Action::SwapClaim(swap_claim) => {
                    let fee = swap_claim.body.fee.clone();

                    // Check swap claim proof
                    let anchor = tx.anchor;
                    swap_claim
                        .proof
                        .verify(
                            anchor,
                            swap_claim.body.nullifier,
                            swap_claim.body.output_data,
                            swap_claim.body.epoch_duration,
                            swap_claim.body.output_1.note_commitment,
                            swap_claim.body.output_2.note_commitment,
                            fee,
                            swap_claim.body.output_1.ephemeral_key,
                            swap_claim.body.output_2.ephemeral_key,
                        )
                        .context("a swap claim proof did not verify")?;

                    // TODO: any other stateless checks?

                    return Ok(());
                }
                _ => {}
            }
        }

        Ok(())
    }

    #[instrument(name = "dex", skip(state, tx))]
    async fn check_tx_stateful(state: Arc<State>, tx: Arc<Transaction>) -> Result<()> {
        // It's important to reject all LP actions for now, to prevent
        // inflation / minting bugs until we implement all required checks
        // (e.g., minting tokens by withdrawing reserves we don't check)
        for action in tx.transaction_body.actions.iter() {
            match action {
                Action::PositionOpen { .. }
                | Action::PositionClose { .. }
                | Action::PositionWithdraw { .. }
                | Action::PositionRewardClaim { .. } => {
                    return Err(anyhow::anyhow!("lp actions not supported yet"));
                }
                Action::Swap(_swap) => {
                    // TODO: are any other checks necessary?

                    return Ok(());
                }
                Action::SwapClaim(swap_claim) => {
                    // 1. Validate the epoch duration passed in the swap claim matches
                    // what we know.
                    let epoch_duration = state.get_epoch_duration().await?;
                    let provided_epoch_duration = swap_claim.body.epoch_duration;
                    if epoch_duration != provided_epoch_duration {
                        return Err(anyhow::anyhow!(
                            "provided epoch duration does not match chain epoch duration"
                        ));
                    }

                    // 2. The stateful check *must* validate that the clearing
                    // prices used in the proof are valid.
                    let provided_output_height = swap_claim.body.output_data.height;
                    let provided_trading_pair = swap_claim.body.output_data.trading_pair;
                    let output_data = state
                        .output_data(provided_output_height, provided_trading_pair)
                        .await?
                        // This check also ensures that the height for the swap is in the past, otherwise
                        // the output data would not be present in the JMT.
                        .ok_or_else(|| anyhow::anyhow!("output data not found"))?;

                    if output_data != swap_claim.body.output_data {
                        return Err(anyhow::anyhow!(
                            "provided output data does not match chain output data"
                        ));
                    }

                    return Ok(());
                }
                _ => {}
            }
        }
        Ok(())
    }

    #[instrument(name = "dex", skip(state, tx))]
    async fn execute_tx(state: &mut StateTransaction, tx: Arc<Transaction>) -> Result<()> {
        for action in tx.transaction_body.actions.iter() {
            match action {
                Action::PositionOpen { .. }
                | Action::PositionClose { .. }
                | Action::PositionWithdraw { .. }
                | Action::PositionRewardClaim { .. } => {}
                Action::Swap(swap) => {
                    // All swaps will be tallied for the block so the
                    // BatchSwapOutputData for the trading pair/block height can
                    // be set during `end_block`.
                    let mut swap_flow = state.swap_flow(&swap.body.trading_pair);

                    // Add the amount of each asset being swapped to the batch swap flow.
                    swap_flow.0 += MockFlowCiphertext::new(swap.body.delta_1_i.into());
                    swap_flow.1 += MockFlowCiphertext::new(swap.body.delta_2_i.into());

                    // Set the batch swap flow for the trading pair.
                    state.put_swap_flow(&swap.body.trading_pair, swap_flow);
                }
                Action::SwapClaim(_) => {
                    // Nothing to do here, note payloads and nullifiers processed in shielded pool
                }
                _ => {}
            }
        }

        Ok(())
    }

    #[instrument(name = "dex", skip(state, end_block))]
    async fn end_block(state: &mut StateTransaction, end_block: &abci::request::EndBlock) {
        // For each batch swap during the block, calculate clearing prices and set in the JMT.
        for (trading_pair, swap_flows) in state.swap_flows() {
            let (delta_1, delta_2) = (swap_flows.0.mock_decrypt(), swap_flows.1.mock_decrypt());

            tracing::debug!(?delta_1, ?delta_2, ?trading_pair);
            let (lambda_1, lambda_2, success) =
                match state.stub_cpmm_reserves(&trading_pair).await.unwrap() {
                    Some(reserves) => {
                        tracing::debug!(?reserves, "stub cpmm is present");
                        let mut amm = StubCpmm { reserves };
                        let (lambda_1, lambda_2) = amm.trade_netted((delta_1, delta_2));
                        tracing::debug!(?lambda_1, ?lambda_2, new_reserves = ?amm.reserves);
                        state.set_stub_cpmm_reserves(&trading_pair, amm.reserves);
                        (lambda_1, lambda_2, true)
                    }
                    None => (0, 0, false),
                };

            let output_data = BatchSwapOutputData {
                height: end_block.height.try_into().unwrap(),
                trading_pair,
                delta_1,
                delta_2,
                lambda_1,
                lambda_2,
                success,
            };
            tracing::debug!(?output_data);
            state.set_output_data(output_data);
        }
    }
}

/// Extension trait providing read access to dex data.
#[async_trait]
pub trait StateReadExt: StateRead {
    async fn output_data(
        &self,
        height: u64,
        trading_pair: TradingPair,
    ) -> Result<Option<BatchSwapOutputData>> {
        self.get(&state_key::output_data(height, trading_pair))
            .await
    }

    async fn stub_cpmm_reserves(&self, trading_pair: &TradingPair) -> Result<Option<Reserves>> {
        self.get(&state_key::stub_cpmm_reserves(trading_pair)).await
    }

    // Get the swap flow for the given trading pair accumulated in this block so far.
    fn swap_flow(&self, pair: &TradingPair) -> SwapFlow {
        self.swap_flows().get(pair).cloned().unwrap_or_default()
    }

    fn swap_flows(&self) -> BTreeMap<TradingPair, SwapFlow> {
        self.get_ephemeral::<BTreeMap<TradingPair, SwapFlow>>(state_key::swap_flows())
            .cloned()
            .unwrap_or_default()
    }
}

impl<T: StateRead> StateReadExt for T {}

/// Extension trait providing write access to dex data.
#[async_trait]
pub trait StateWriteExt: StateWrite + StateReadExt {
    fn set_output_data(&mut self, output_data: BatchSwapOutputData) {
        let height = output_data.height;
        let trading_pair = output_data.trading_pair;
        self.put(state_key::output_data(height, trading_pair), output_data);
    }

    fn set_stub_cpmm_reserves(&mut self, trading_pair: &TradingPair, reserves: Reserves) {
        self.put(state_key::stub_cpmm_reserves(trading_pair), reserves);
    }

    fn put_swap_flow(&mut self, trading_pair: &TradingPair, swap_flow: SwapFlow) {
        // TODO: replace with IM struct later
        let mut swap_flows = self.swap_flows();
        swap_flows.insert(*trading_pair, swap_flow);
        self.put_ephemeral(state_key::swap_flows().into(), swap_flows)
    }
}

impl<T: StateWrite> StateWriteExt for T {}
