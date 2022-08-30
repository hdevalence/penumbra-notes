use std::{
    fmt::{self, Debug, Formatter},
    mem,
};

use penumbra_component::stake::{rate::RateData, validator};
use penumbra_crypto::{
    keys::AddressIndex,
    memo::MemoPlaintext,
    rdsa::{SpendAuth, VerificationKey},
    Address, DelegationToken, FieldExt, Fr, FullViewingKey, Note, Value, STAKING_TOKEN_ASSET_ID,
};
use penumbra_proto::view::NotesRequest;
use penumbra_tct as tct;
use penumbra_transaction::{
    action::{Proposal, ProposalSubmit, ProposalWithdrawBody, ValidatorVote},
    plan::{ActionPlan, OutputPlan, ProposalWithdrawPlan, SpendPlan, TransactionPlan},
};
use penumbra_view::ViewClient;
use rand::{CryptoRng, RngCore};
use tracing::instrument;

pub use super::balance::Balance;

/// A planner for a [`TransactionPlan`] that can fill in the required spends and change outputs upon
/// finalization to make a transaction balance.
pub struct Planner<R: RngCore + CryptoRng> {
    rng: R,
    balance: Balance,
    plan: TransactionPlan,
    proposal_submits: Vec<Proposal>,
    proposal_withdraws: Vec<(Address, ProposalWithdrawBody)>,
    // IMPORTANT: if you add more fields here, make sure to clear them when the planner is finished
}

impl<R: RngCore + CryptoRng> Debug for Planner<R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Builder")
            .field("balance", &self.balance)
            .field("plan", &self.plan)
            .finish()
    }
}

impl<R: RngCore + CryptoRng> Planner<R> {
    /// Create a new planner.
    pub fn new(rng: R) -> Self {
        Self {
            rng,
            balance: Balance::default(),
            plan: TransactionPlan::default(),
            proposal_submits: Vec::new(),
            proposal_withdraws: Vec::new(),
        }
    }

    /// Get the current transaction balance of the planner.
    pub fn balance(&self) -> &Balance {
        &self.balance
    }

    /// Set the expiry height for the transaction plan.
    #[instrument(skip(self))]
    pub fn expiry_height(&mut self, expiry_height: u64) -> &mut Self {
        self.plan.expiry_height = expiry_height;
        self
    }

    /// Add a fee to the transaction plan.
    ///
    /// Calling this function more than once will add to the fee, not replace it.
    #[instrument(skip(self))]
    pub fn fee(&mut self, fee: u64) -> &mut Self {
        self.balance.require(Value {
            amount: fee,
            asset_id: *STAKING_TOKEN_ASSET_ID,
        });
        self.plan.fee.0 += fee;
        self
    }

    /// Spend a specific positioned note in the transaction.
    ///
    /// If you don't use this method to specify spends, they will be filled in automatically from
    /// the view service when the plan is [`finish`](Builder::finish)ed.
    #[instrument(skip(self))]
    pub fn spend(&mut self, note: Note, position: tct::Position) -> &mut Self {
        let spend = SpendPlan::new(&mut self.rng, note, position).into();
        self.action(spend);
        self
    }

    /// Add an output note from this transaction.
    ///
    /// Any unused output value will be redirected back to the originating address as change notes
    /// when the plan is [`finish`](Builder::finish)ed.
    #[instrument(skip(self, memo))]
    pub fn output(&mut self, value: Value, address: Address, memo: MemoPlaintext) -> &mut Self {
        let output = OutputPlan::new(&mut self.rng, value, address, memo).into();
        self.action(output);
        self
    }

    /// Add a delegation to this transaction.
    ///
    /// If you don't specify spends or outputs as well, they will be filled in automatically.
    #[instrument(skip(self))]
    pub fn delegate(&mut self, unbonded_amount: u64, rate_data: RateData) -> &mut Self {
        let delegation = rate_data.build_delegate(unbonded_amount).into();
        self.action(delegation);
        self
    }

    /// Add an undelegation to this transaction.
    ///
    /// Undelegations have special rules to prevent you from accidentally locking up funds while the
    /// transaction is unbonding: any transaction containing an undelegation must contain exactly
    /// one undelegation, must spend only delegation tokens matching the validator from which the
    /// undelegation is being performed, and must output only staking tokens. This means that it
    /// must be an "exact change" transaction with no other actions.
    ///
    /// In order to ensure that the transaction is an "exact change" transaction, you should
    /// probably explicitly add the precisely correct spends to the transaction, after having
    /// generated those exact notes by splitting notes in a previous transaction, if necessary.
    ///
    /// The conditions imposed by the consensus rules are more permissive, but the planner will
    /// protect you from shooting yourself in the foot by throwing an error, should the built
    /// transaction fail these conditions.
    #[instrument(skip(self))]
    pub fn undelegate(&mut self, delegation_amount: u64, rate_data: RateData) -> &mut Self {
        let undelegation = rate_data.build_undelegate(delegation_amount).into();
        self.action(undelegation);
        self
    }

    /// Upload a validator definition in this transaction.
    #[instrument(skip(self))]
    pub fn validator_definition(&mut self, new_validator: validator::Definition) -> &mut Self {
        self.action(ActionPlan::ValidatorDefinition(new_validator.into()));
        self
    }

    /// Submit a new governance proposal in this transaction.
    #[instrument(skip(self))]
    pub fn proposal_submit(&mut self, proposal: Proposal) -> &mut Self {
        self.proposal_submits.push(proposal);
        self
    }

    /// Withdraw a governance proposal in this transaction.
    #[instrument(skip(self))]
    pub fn proposal_withdraw(
        &mut self,
        proposal_id: u64,
        deposit_refund_address: Address,
        reason: String,
    ) -> &mut Self {
        self.proposal_withdraws.push((
            deposit_refund_address,
            ProposalWithdrawBody {
                proposal: proposal_id,
                reason,
            },
        ));
        self
    }

    /// Cast a validator vote in this transaction.
    #[instrument(skip(self))]
    pub fn validator_vote(&mut self, vote: ValidatorVote) -> &mut Self {
        self.action(ActionPlan::ValidatorVote(vote));
        self
    }

    fn action(&mut self, action: ActionPlan) -> &mut Self {
        use ActionPlan::*;

        // Track this action's contribution to the value balance of the transaction: this must match
        // the actual contribution to the value commitment, but this isn't checked, so make sure
        // that when you're adding a new action, you correctly match this up to the calculation of
        // the value commitment for the transaction, or else the planner will submit transactions
        // that are not balanced!
        match &action {
            Spend(spend) => self.balance.provide(spend.note.value()),
            Output(output) => self.balance.require(output.value),
            Delegate(delegate) => {
                self.balance.require(Value {
                    amount: delegate.unbonded_amount,
                    asset_id: *STAKING_TOKEN_ASSET_ID,
                });
                self.balance.provide(Value {
                    amount: delegate.delegation_amount,
                    asset_id: DelegationToken::new(delegate.validator_identity).id(),
                })
            }
            Undelegate(undelegate) => {
                self.balance.provide(Value {
                    amount: undelegate.unbonded_amount,
                    asset_id: *STAKING_TOKEN_ASSET_ID,
                });
                self.balance.require(Value {
                    amount: undelegate.delegation_amount,
                    asset_id: DelegationToken::new(undelegate.validator_identity).id(),
                })
            }
            ProposalSubmit(proposal_submit) => {
                self.balance.require(Value {
                    amount: proposal_submit.deposit_amount,
                    asset_id: *STAKING_TOKEN_ASSET_ID,
                });
            }
            PositionOpen(_) => todo!(),
            PositionClose(_) => todo!(),
            PositionWithdraw(_) => todo!(),
            PositionRewardClaim(_) => todo!(),
            Swap(_) => todo!(),
            SwapClaim(_) => todo!(),
            IBCAction(_) => todo!(),
            ValidatorDefinition(_) | ProposalWithdraw(_) | DelegatorVote(_) | ValidatorVote(_) => {
                // No contribution to the value balance of the transaction
            }
        };

        // Add the action to the plan
        self.plan.actions.push(action);
        self
    }

    /// Add spends and change outputs as required to balance the transaction, using the view service
    /// provided to supply the notes and other information.
    ///
    /// Clears the contents of the planner, which can be re-used.
    #[instrument(skip(self, view, fvk))]
    pub async fn plan<V: ViewClient>(
        &mut self,
        view: &mut V,
        fvk: &FullViewingKey,
        source: Option<AddressIndex>,
    ) -> anyhow::Result<TransactionPlan> {
        tracing::debug!(plan = ?self.plan, balance = ?self.balance, "finalizing transaction");

        // Fill in the chain id based on the view service
        let chain_params = view.chain_params().await?;
        self.plan.chain_id = chain_params.chain_id;

        // Proposals aren't actually turned into action plans until now, because we need the view
        // service to fill in the details. Now we have the chain parameters and the FVK, so we can
        // automatically fill in the rest of the action plan without asking the user for anything:
        for proposal in mem::take(&mut self.proposal_submits) {
            let (deposit_refund_address, withdraw_proposal_key) =
                self.proposal_address_and_withdraw_key(fvk);

            self.action(
                ProposalSubmit {
                    proposal,
                    deposit_amount: chain_params.proposal_deposit_amount,
                    deposit_refund_address,
                    withdraw_proposal_key,
                }
                .into(),
            );
        }

        // Similarly, proposal withdrawals need the FVK to convert the address into the original
        // randomizer, so we delay adding it to the transaction plan until now
        for (address, body) in mem::take(&mut self.proposal_withdraws) {
            let randomizer = self.proposal_withdraw_randomizer(fvk, &address);
            self.action(ProposalWithdrawPlan { body, randomizer }.into());
        }

        // Get all notes required to fulfill needed spends
        let mut spends = Vec::new();
        for Value { amount, asset_id } in self.balance.required() {
            spends.extend(
                view.notes(NotesRequest {
                    account_id: Some(fvk.hash().into()),
                    asset_id: Some(asset_id.into()),
                    address_index: source.map(Into::into),
                    amount_to_spend: amount,
                    include_spent: false,
                })
                .await?,
            );
        }

        // Add the required spends to the planner
        for record in spends {
            self.spend(record.note, record.position);
        }

        // For any remaining provided balance, make a single change note for each
        let self_address = fvk
            .incoming()
            .payment_address(source.unwrap_or(AddressIndex::Numeric(0)))
            .0;

        for value in self.balance.provided().collect::<Vec<_>>() {
            self.output(value, self_address, MemoPlaintext::default());
        }

        // TODO: add dummy change outputs in the staking token denomination (this means they'll pass
        // the undelegate rules check)

        // Ensure that the transaction won't cause excessive quarantining
        self.check_undelegate_rules()?;

        // Add clue plans for `Output`s.
        let fmd_params = view.fmd_parameters().await?;
        let precision_bits = fmd_params.precision_bits;
        self.plan
            .add_all_clue_plans(&mut self.rng, precision_bits.into());

        // Now the transaction should be fully balanced, unless we didn't have enough to spend
        if !self.balance.is_zero() {
            anyhow::bail!(
                "balance is non-zero after attempting to balance transaction: {:?}",
                self.balance
            );
        }

        tracing::debug!(plan = ?self.plan, "finished balancing transaction");

        // Clear the planner and pull out the plan to return
        self.balance = Balance::new();
        let plan = mem::take(&mut self.plan);

        Ok(plan)
    }

    /// Undelegations should have a very particular form to avoid excessive quarantining: all
    /// their spends should be of the delegation token being undelegated, and all their outputs
    /// should be of the staking token, and they should contain no other actions.
    fn check_undelegate_rules(&self) -> anyhow::Result<()> {
        match self
            .plan
            .actions
            .iter()
            .filter_map(|action| {
                if let ActionPlan::Undelegate(undelegate) = action {
                    Some(undelegate)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .as_slice()
        {
            [] => {
                // No undelegations
            }
            [undelegate] => {
                let delegation_asset_id = DelegationToken::new(undelegate.validator_identity).id();
                for action in self.plan.actions.iter() {
                    match action {
                        ActionPlan::Spend(spend) => {
                            if spend.note.value().asset_id != delegation_asset_id {
                                return Err(anyhow::anyhow!(
                                    "undelegation transaction must spend only delegation tokens"
                                ));
                            }
                        }
                        ActionPlan::Output(output) => {
                            if output.value.asset_id != *STAKING_TOKEN_ASSET_ID {
                                return Err(anyhow::anyhow!(
                                    "undelegation transaction must output only staking tokens"
                                ));
                            }
                        }
                        ActionPlan::Undelegate(_) => {
                            // There's only one undelegate action, so this is the one we already
                            // know about, so we don't have to do anything with it
                        }
                        _ => {
                            return Err(anyhow::anyhow!(
                                "undelegation transaction must not contain extraneous actions"
                            ))
                        }
                    }
                }
            }
            _ => {
                return Err(anyhow::anyhow!(
                    "undelegation transaction must not contain multiple undelegations"
                ))
            }
        }

        Ok(())
    }

    /// Get a random address/withdraw key pair for proposals.
    fn proposal_address_and_withdraw_key(
        &mut self,
        fvk: &FullViewingKey,
    ) -> (Address, VerificationKey<SpendAuth>) {
        // The deposit refund address should be an ephemeral address
        let deposit_refund_address = fvk.incoming().ephemeral_address(&mut self.rng).0;

        // The proposal withdraw verification key is the spend auth verification key randomized by the
        // deposit refund address's address index
        let withdraw_proposal_key = {
            // Use the fvk to get the original address index of the diversifier
            let deposit_refund_address_index = fvk
                .incoming()
                .index_for_diversifier(deposit_refund_address.diversifier());

            // Convert this to a vector
            let mut deposit_refund_address_index_bytes =
                deposit_refund_address_index.to_bytes().to_vec();

            // Pad it with zeros to be 32 bytes long (the size expected by a randomizer)
            deposit_refund_address_index_bytes.extend([0; 16]);

            // Convert it back to exactly 32 bytes
            let deposit_refund_address_index_bytes = deposit_refund_address_index_bytes
                .try_into()
                .expect("exactly 32 bytes");

            // Get the scalar `Fr` element derived from these bytes
            let withdraw_proposal_key_randomizer =
                Fr::from_bytes(deposit_refund_address_index_bytes)
                    .expect("bytes are within range for `Fr`");

            // Randomize the spend verification key for the fvk using this randomizer
            fvk.spend_verification_key()
                .randomize(&withdraw_proposal_key_randomizer)
        };

        (deposit_refund_address, withdraw_proposal_key)
    }

    /// Get the randomizer from an address using the FVK.
    fn proposal_withdraw_randomizer(&self, fvk: &FullViewingKey, address: &Address) -> Fr {
        // Use the fvk to get the original address index of the diversifier
        let deposit_refund_address_index =
            fvk.incoming().index_for_diversifier(address.diversifier());

        // Convert this to a vector
        let mut deposit_refund_address_index_bytes =
            deposit_refund_address_index.to_bytes().to_vec();
        // Pad it with zeros to be 32 bytes long (the size expected by a randomizer)
        deposit_refund_address_index_bytes.extend([0; 16]);
        // Convert it back to exactly 32 bytes
        let deposit_refund_address_index_bytes = deposit_refund_address_index_bytes
            .try_into()
            .expect("exactly 32 bytes");

        // Get the scalar `Fr` element derived from these bytes
        Fr::from_bytes(deposit_refund_address_index_bytes).expect("bytes are within range for `Fr`")
    }
}
