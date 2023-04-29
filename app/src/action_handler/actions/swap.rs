use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use penumbra_chain::sync::StatePayload;
use penumbra_crypto::MockFlowCiphertext;
use penumbra_proof_params::SWAP_PROOF_VERIFICATION_KEY;
use penumbra_shielded_pool::component::NoteManager;
use penumbra_storage::{StateRead, StateWrite};
use penumbra_transaction::{action::Swap, IsAction};

use crate::{
    action_handler::ActionHandler,
    stubdex::{StateReadExt as _, StateWriteExt as _},
};

#[async_trait]
impl ActionHandler for Swap {
    type CheckStatelessContext = ();
    async fn check_stateless(&self, _context: ()) -> Result<()> {
        // Check that the trading pair is distinct.
        if self.body.trading_pair.asset_1() == self.body.trading_pair.asset_2() {
            return Err(anyhow::anyhow!("Trading pair must be distinct"));
        }

        self.proof.verify(
            &SWAP_PROOF_VERIFICATION_KEY,
            self.balance_commitment(),
            self.body.payload.commitment,
            self.body.fee_commitment,
        )?;

        Ok(())
    }

    async fn check_stateful<S: StateRead + 'static>(&self, _state: Arc<S>) -> Result<()> {
        Ok(())
    }

    async fn execute<S: StateWrite>(&self, mut state: S) -> Result<()> {
        let swap = self;

        // All swaps will be tallied for the block so the
        // BatchSwapOutputData for the trading pair/block height can
        // be set during `end_block`.
        let mut swap_flow = state.swap_flow(&swap.body.trading_pair);

        // Add the amount of each asset being swapped to the batch swap flow.
        swap_flow.0 += MockFlowCiphertext::new(swap.body.delta_1_i);
        swap_flow.1 += MockFlowCiphertext::new(swap.body.delta_2_i);

        // Set the batch swap flow for the trading pair.
        state.put_swap_flow(&swap.body.trading_pair, swap_flow);

        // Record the swap commitment in the state.
        let source = state.object_get("source").unwrap_or_default();
        state
            .add_state_payload(StatePayload::Swap {
                source,
                swap: self.body.payload.clone(),
            })
            .await;

        Ok(())
    }
}
