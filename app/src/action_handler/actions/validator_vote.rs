use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use penumbra_chain::StateReadExt as _;
use penumbra_proto::DomainType;
use penumbra_storage::{StateRead, StateWrite};
use penumbra_transaction::{
    action::{ValidatorVote, ValidatorVoteBody},
    proposal, Transaction,
};

use crate::{
    action_handler::ActionHandler,
    governance::{StateReadExt, StateWriteExt},
};

#[async_trait]
impl ActionHandler for ValidatorVote {
    async fn check_stateless(&self, _context: Arc<Transaction>) -> Result<()> {
        let ValidatorVote { body, auth_sig } = self;

        // Check the signature using the GOVERNANCE KEY:
        let body_bytes = body.encode_to_vec();
        body.governance_key
            .0
            .verify(&body_bytes, auth_sig)
            .context("validator vote signature failed to verify")?;

        // This is stateless verification, so we still need to check that the proposal being voted
        // on exists, and that this validator hasn't voted on it already.

        Ok(())
    }

    async fn check_stateful<S: StateRead + 'static>(&self, state: Arc<S>) -> Result<()> {
        let ValidatorVote {
            body:
                ValidatorVoteBody {
                    proposal,
                    vote: _, // All votes are valid, so we don't need to do anything with this
                    identity_key,
                    governance_key,
                },
            auth_sig: _, // We already checked this in stateless verification
        } = self;

        state.check_proposal_votable(*proposal).await?;
        state
            .check_validator_active_at_proposal_start(*proposal, identity_key)
            .await?;
        state
            .check_validator_has_not_voted(*proposal, identity_key)
            .await?;
        state
            .check_governance_key_matches_validator(identity_key, governance_key)
            .await?;

        Ok(())
    }

    async fn execute<S: StateWrite>(&self, mut state: S) -> Result<()> {
        let ValidatorVote {
            auth_sig: _,
            body:
                ValidatorVoteBody {
                    proposal,
                    vote,
                    identity_key,
                    governance_key: _, // This is only used for checks so that stateless verification can be done on the signature
                },
        } = self;

        tracing::debug!(proposal = %proposal, "cast validator vote");
        state.cast_validator_vote(*proposal, *identity_key, *vote);

        // If the proposal was an emergency proposal, enact it immediately if it passes the special
        // emergency proposal threshold.
        state.enact_proposals_if_emergency(Some(*proposal)).await?;

        Ok(())
    }
}
