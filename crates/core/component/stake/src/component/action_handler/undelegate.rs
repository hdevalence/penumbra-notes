use anyhow::{ensure, Result};
use async_trait::async_trait;
use cnidarium::StateWrite;
use penumbra_shielded_pool::component::SupplyWrite;

use crate::{
    component::action_handler::ActionHandler,
    component::{validator_handler::ValidatorDataRead, StateWriteExt as _},
    event, Undelegate,
};

#[async_trait]
impl ActionHandler for Undelegate {
    type CheckStatelessContext = ();
    async fn check_stateless(&self, _context: ()) -> Result<()> {
        Ok(())
    }

    async fn check_and_execute<S: StateWrite>(&self, mut state: S) -> Result<()> {
        // These checks all formerly happened in the `check_historical` method,
        // if profiling shows that they cause a bottleneck we could (CAREFULLY)
        // move some of them back.

        let u = self;
        let rate_data = state
            .get_validator_rate(&u.validator_identity)
            .await?
            .ok_or_else(|| {
                anyhow::anyhow!("unknown validator identity {}", u.validator_identity)
            })?;

        // Check whether the start epoch is correct first, to give a more helpful
        // error message if it's wrong.
        if u.start_epoch_index != rate_data.epoch_index {
            anyhow::bail!(
                "undelegation was prepared for next epoch {} but the next epoch is {}",
                u.start_epoch_index,
                rate_data.epoch_index
            );
        }

        // For undelegations, we enforce correct computation (with rounding)
        // of the *unbonded amount based on the delegation amount*, because
        // users (should be) starting with the amount of delegation tokens they
        // wish to undelegate, and computing the amount of unbonded stake
        // they receive.
        //
        // The direction of the computation matters because the computation
        // involves rounding, so while both
        //
        // (unbonded amount, rates) -> delegation amount
        // (delegation amount, rates) -> unbonded amount
        //
        // should give approximately the same results, they may not give
        // exactly the same results.
        let expected_unbonded_amount = rate_data.unbonded_amount(u.delegation_amount);

        ensure!(
            u.unbonded_amount == expected_unbonded_amount,
            "undelegation amount {} does not match expected amount {}",
            u.unbonded_amount,
            expected_unbonded_amount,
        );

        // (end of former check_historical impl)

        tracing::debug!(?self, "queuing undelegation for next epoch");
        state.push_undelegation(self.clone());
        // Register the undelegation's denom, so clients can look it up later.
        state
            .register_denom(&self.unbonding_token().denom())
            .await?;
        // TODO: should we be tracking changes to token supply here or in end_epoch?
        state.record(event::undelegate(self));

        Ok(())
    }
}
