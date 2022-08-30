//! Declarative transaction plans, used for transaction authorization and
//! creation.

use anyhow::Result;
use penumbra_crypto::{transaction::Fee, Address};
use penumbra_proto::{ibc as pb_ibc, stake as pb_stake, transaction as pb, Protobuf};
use rand::{CryptoRng, Rng};
use serde::{Deserialize, Serialize};

use crate::action::{Delegate, ProposalSubmit, Undelegate};

mod action;
mod auth;
mod build;
mod clue;

pub use action::{
    ActionPlan, DelegatorVotePlan, OutputPlan, ProposalWithdrawPlan, SpendPlan, SwapClaimPlan,
    SwapPlan, ValidatorVotePlan,
};
pub use clue::CluePlan;

/// A declaration of a planned [`Transaction`](crate::Transaction),
/// for use in transaction authorization and creation.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(try_from = "pb::TransactionPlan", into = "pb::TransactionPlan")]
pub struct TransactionPlan {
    /// A list of this transaction's actions.
    pub actions: Vec<ActionPlan>,
    pub expiry_height: u64,
    pub chain_id: String,
    pub fee: Fee,
    pub clue_plans: Vec<CluePlan>,
}

impl Default for TransactionPlan {
    fn default() -> Self {
        Self {
            actions: Default::default(),
            expiry_height: 0,
            chain_id: String::new(),
            fee: Fee(0),
            clue_plans: vec![],
        }
    }
}

impl TransactionPlan {
    pub fn spend_plans(&self) -> impl Iterator<Item = &SpendPlan> {
        self.actions.iter().filter_map(|action| {
            if let ActionPlan::Spend(s) = action {
                Some(s)
            } else {
                None
            }
        })
    }

    pub fn output_plans(&self) -> impl Iterator<Item = &OutputPlan> {
        self.actions.iter().filter_map(|action| {
            if let ActionPlan::Output(o) = action {
                Some(o)
            } else {
                None
            }
        })
    }

    pub fn clue_plans(&self) -> impl Iterator<Item = &CluePlan> {
        self.clue_plans.iter()
    }

    pub fn delegations(&self) -> impl Iterator<Item = &Delegate> {
        self.actions.iter().filter_map(|action| {
            if let ActionPlan::Delegate(d) = action {
                Some(d)
            } else {
                None
            }
        })
    }

    pub fn undelegations(&self) -> impl Iterator<Item = &Undelegate> {
        self.actions.iter().filter_map(|action| {
            if let ActionPlan::Undelegate(d) = action {
                Some(d)
            } else {
                None
            }
        })
    }

    pub fn ibc_actions(&self) -> impl Iterator<Item = &pb_ibc::IbcAction> {
        self.actions.iter().filter_map(|action| {
            if let ActionPlan::IBCAction(ibc_action) = action {
                Some(ibc_action)
            } else {
                None
            }
        })
    }

    pub fn validator_definitions(&self) -> impl Iterator<Item = &pb_stake::ValidatorDefinition> {
        self.actions.iter().filter_map(|action| {
            if let ActionPlan::ValidatorDefinition(d) = action {
                Some(d)
            } else {
                None
            }
        })
    }

    pub fn proposal_submits(&self) -> impl Iterator<Item = &ProposalSubmit> {
        self.actions.iter().filter_map(|action| {
            if let ActionPlan::ProposalSubmit(p) = action {
                Some(p)
            } else {
                None
            }
        })
    }

    pub fn proposal_withdraws(&self) -> impl Iterator<Item = &ProposalWithdrawPlan> {
        self.actions.iter().filter_map(|action| {
            if let ActionPlan::ProposalWithdraw(p) = action {
                Some(p)
            } else {
                None
            }
        })
    }

    pub fn delegator_vote_plans(&self) -> impl Iterator<Item = &DelegatorVotePlan> {
        self.actions.iter().filter_map(|action| {
            if let ActionPlan::DelegatorVote(v) = action {
                Some(v)
            } else {
                None
            }
        })
    }

    pub fn validator_votes(&self) -> impl Iterator<Item = &ValidatorVotePlan> {
        self.actions.iter().filter_map(|action| {
            if let ActionPlan::ValidatorVote(v) = action {
                Some(v)
            } else {
                None
            }
        })
    }

    pub fn swap_plans(&self) -> impl Iterator<Item = &SwapPlan> {
        self.actions.iter().filter_map(|action| {
            if let ActionPlan::Swap(v) = action {
                Some(v)
            } else {
                None
            }
        })
    }

    pub fn swap_claim_plans(&self) -> impl Iterator<Item = &SwapClaimPlan> {
        self.actions.iter().filter_map(|action| {
            if let ActionPlan::SwapClaim(v) = action {
                Some(v)
            } else {
                None
            }
        })
    }

    /// Convenience method to get all the destination addresses for each `OutputPlan`s.
    pub fn dest_addresses(&self) -> Vec<Address> {
        self.output_plans()
            .into_iter()
            .map(|plan| plan.dest_address)
            .collect()
    }

    /// Convenience method to get the number of `OutputPlan`s in this transaction.
    pub fn num_outputs(&self) -> usize {
        self.output_plans().into_iter().count()
    }

    /// Method to add `CluePlan`s to a `TransactionPlan`.
    pub fn add_all_clue_plans<R: CryptoRng + Rng>(&mut self, mut rng: R, precision_bits: usize) {
        // Add one clue per recipient.
        let mut clue_plans = vec![];
        for dest_address in self.dest_addresses() {
            clue_plans.push(CluePlan::new(&mut rng, dest_address, precision_bits));
        }

        // Now add dummy clues until we have one clue per output.
        let num_dummy_clues = self.num_outputs() - clue_plans.len();
        for _ in 0..num_dummy_clues {
            let dummy_address = Address::dummy(&mut rng);
            clue_plans.push(CluePlan::new(&mut rng, dummy_address, precision_bits));
        }

        self.clue_plans = clue_plans;
    }
}

impl Protobuf<pb::TransactionPlan> for TransactionPlan {}

impl From<TransactionPlan> for pb::TransactionPlan {
    fn from(msg: TransactionPlan) -> Self {
        Self {
            actions: msg.actions.into_iter().map(Into::into).collect(),
            expiry_height: msg.expiry_height,
            chain_id: msg.chain_id,
            fee: Some(msg.fee.into()),
            clue_plans: msg.clue_plans.into_iter().map(Into::into).collect(),
        }
    }
}

impl TryFrom<pb::TransactionPlan> for TransactionPlan {
    type Error = anyhow::Error;
    fn try_from(value: pb::TransactionPlan) -> Result<Self, Self::Error> {
        Ok(Self {
            actions: value
                .actions
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?,
            expiry_height: value.expiry_height,
            chain_id: value.chain_id,
            fee: value
                .fee
                .ok_or_else(|| anyhow::anyhow!("missing fee"))?
                .try_into()?,
            clue_plans: value
                .clue_plans
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?,
        })
    }
}
