use std::{
    collections::BTreeMap,
    fmt::{self, Debug, Formatter},
    mem,
};

use anyhow::{ensure, Result};
use penumbra_sct::epoch::Epoch;
use rand::{CryptoRng, RngCore};
use rand_core::OsRng;
use tracing::instrument;

use crate::{SpendableNoteRecord, ViewClient};
use anyhow::anyhow;
use penumbra_asset::{asset, Balance, Value};
use penumbra_auction::auction::dutch::actions::ActionDutchAuctionWithdrawPlan;
use penumbra_auction::auction::dutch::DutchAuctionDescription;
use penumbra_auction::auction::{
    dutch::actions::{ActionDutchAuctionEnd, ActionDutchAuctionSchedule},
    AuctionId,
};
use penumbra_community_pool::CommunityPoolDeposit;
use penumbra_dex::{
    lp::action::{PositionClose, PositionOpen},
    lp::plan::PositionWithdrawPlan,
    lp::position::{self, Position},
    lp::Reserves,
    swap::SwapPlaintext,
    swap::SwapPlan,
    swap_claim::SwapClaimPlan,
    TradingPair,
};
use penumbra_fee::{Fee, FeeTier, Gas, GasPrices};
use penumbra_governance::{
    proposal_state, DelegatorVotePlan, Proposal, ProposalDepositClaim, ProposalSubmit,
    ProposalWithdraw, ValidatorVote, Vote,
};
use penumbra_ibc::IbcRelay;
use penumbra_keys::{keys::AddressIndex, Address};
use penumbra_num::Amount;
use penumbra_proto::view::v1::{NotesForVotingRequest, NotesRequest};
use penumbra_shielded_pool::{Ics20Withdrawal, Note, OutputPlan, SpendPlan};
use penumbra_stake::{rate::RateData, validator, IdentityKey, UndelegateClaimPlan};
use penumbra_tct as tct;
use penumbra_transaction::{
    gas::GasCost,
    memo::MemoPlaintext,
    plan::{ActionPlan, MemoPlan, TransactionPlan},
    TransactionParameters,
};

/// A planner for a [`TransactionPlan`] that can fill in the required spends and change outputs upon
/// finalization to make a transaction balance.
pub struct Planner<R: RngCore + CryptoRng> {
    rng: R,
    /// The transaction plan to materialize.
    plan: TransactionPlan,
    // A list of the user-specified outputs.
    actions: Vec<ActionPlan>,
    // These are tracked separately for convenience when adjusting change.
    change_outputs: BTreeMap<asset::Id, OutputPlan>,
    /// The fee tier to apply to this transaction.
    fee_tier: FeeTier,
    /// The set of prices used for gas estimation.
    gas_prices: GasPrices,
    /// The set of IBC actions to include in the transaction.
    ibc_actions: Vec<IbcRelay>,
}

impl<R: RngCore + CryptoRng> Debug for Planner<R> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Planner").field("plan", &self.plan).finish()
    }
}

impl<R: RngCore + CryptoRng> Planner<R> {
    /// Create a new planner.
    pub fn new(rng: R) -> Self {
        Self {
            rng,
            plan: TransactionPlan::default(),
            ibc_actions: Vec::new(),
            gas_prices: GasPrices::zero(),
            fee_tier: FeeTier::default(),
            actions: Vec::new(),
            change_outputs: BTreeMap::default(),
        }
    }

    /// Set the current gas prices for fee prediction.
    #[instrument(skip(self))]
    pub fn set_gas_prices(&mut self, gas_prices: GasPrices) -> &mut Self {
        self.gas_prices = gas_prices;
        self
    }

    /// Set the fee tier.
    #[instrument(skip(self))]
    pub fn set_fee_tier(&mut self, fee_tier: FeeTier) -> &mut Self {
        self.fee_tier = fee_tier;
        self
    }

    /// Set the expiry height for the transaction plan.
    #[instrument(skip(self))]
    pub fn expiry_height(&mut self, expiry_height: u64) -> &mut Self {
        self.plan.transaction_parameters.expiry_height = expiry_height;
        self
    }

    /// Set a memo for this transaction plan.
    ///
    /// Errors if the memo is too long.
    #[instrument(skip(self))]
    pub fn memo(&mut self, memo: MemoPlaintext) -> anyhow::Result<&mut Self> {
        self.plan.memo = Some(MemoPlan::new(&mut self.rng, memo)?);
        Ok(self)
    }

    /// Add a fee to the transaction plan.
    ///
    /// This function should be called once.
    #[instrument(skip(self))]
    pub fn fee(&mut self, fee: Fee) -> &mut Self {
        self.plan.transaction_parameters.fee = fee;
        self
    }

    /// Spend a specific positioned note in the transaction.
    #[instrument(skip(self))]
    pub fn spend(&mut self, note: Note, position: tct::Position) -> &mut Self {
        let spend = SpendPlan::new(&mut self.rng, note, position).into();
        self.action(spend);
        self
    }

    /// Add an output note from this transaction.
    ///
    /// Any unused output value will be redirected back to the originating address as change notes.
    #[instrument(skip(self))]
    pub fn output(&mut self, value: Value, address: Address) -> &mut Self {
        let output = OutputPlan::new(&mut self.rng, value, address).into();
        self.action(output);
        self
    }

    /// Open a liquidity position in the order book.
    #[instrument(skip(self))]
    pub fn position_open(&mut self, position: Position) -> &mut Self {
        self.action(ActionPlan::PositionOpen(PositionOpen { position }));
        self
    }

    /// Close a liquidity position in the order book.
    #[instrument(skip(self))]
    pub fn position_close(&mut self, position_id: position::Id) -> &mut Self {
        self.action(ActionPlan::PositionClose(PositionClose { position_id }));
        self
    }

    /// Withdraw a liquidity position in the order book.
    ///
    /// Note: Currently this only supports an initial withdrawal from Closed, with no rewards.
    #[instrument(skip(self))]
    pub fn position_withdraw(
        &mut self,
        position_id: position::Id,
        reserves: Reserves,
        pair: TradingPair,
    ) -> &mut Self {
        self.action(ActionPlan::PositionWithdraw(PositionWithdrawPlan {
            reserves,
            position_id,
            pair,
            sequence: 0,
            rewards: Vec::new(),
        }));
        self
    }

    /// Schedule a Dutch auction.
    #[instrument(skip(self))]
    pub fn dutch_auction_schedule(
        &mut self,
        input: Value,
        output_id: asset::Id,
        max_output: Amount,
        min_output: Amount,
        start_height: u64,
        end_height: u64,
        step_count: u64,
        nonce: [u8; 32],
    ) -> &mut Self {
        self.action(ActionPlan::ActionDutchAuctionSchedule(
            ActionDutchAuctionSchedule {
                description: DutchAuctionDescription {
                    input,
                    output_id,
                    max_output,
                    min_output,
                    start_height,
                    end_height,
                    step_count,
                    nonce,
                },
            },
        ))
    }

    /// Ends a Dutch auction.
    #[instrument(skip(self))]
    pub fn dutch_auction_end(&mut self, auction_id: AuctionId) -> &mut Self {
        self.action(ActionPlan::ActionDutchAuctionEnd(ActionDutchAuctionEnd {
            auction_id,
        }))
    }
    /// Withdraws the reserves of the Dutch auction.
    #[instrument(skip(self))]
    pub fn dutch_auction_withdraw(
        &mut self,
        auction_id: AuctionId,
        seq: u64,
        reserves_input: Value,
        reserves_output: Value,
    ) -> &mut Self {
        self.action(ActionPlan::ActionDutchAuctionWithdraw(
            ActionDutchAuctionWithdrawPlan {
                auction_id,
                seq,
                reserves_input,
                reserves_output,
            },
        ))
    }

    /// Perform a swap based on input notes in the transaction.
    #[instrument(skip(self))]
    pub fn swap(
        &mut self,
        input_value: Value,
        into_asset: asset::Id,
        swap_claim_fee: Fee,
        claim_address: Address,
    ) -> Result<&mut Self> {
        // Determine the canonical order for the assets being swapped.
        // This will determine whether the input amount is assigned to delta_1 or delta_2.
        let trading_pair = TradingPair::new(input_value.asset_id, into_asset);

        // If `trading_pair.asset_1` is the input asset, then `delta_1` is the input amount,
        // and `delta_2` is 0.
        //
        // Otherwise, `delta_1` is 0, and `delta_2` is the input amount.
        let (delta_1, delta_2) = if trading_pair.asset_1() == input_value.asset_id {
            (input_value.amount, 0u64.into())
        } else {
            (0u64.into(), input_value.amount)
        };

        // If there is no input, then there is no swap.
        if delta_1 == Amount::zero() && delta_2 == Amount::zero() {
            anyhow::bail!("No input value for swap");
        }

        // Create the `SwapPlaintext` representing the swap to be performed:
        let swap_plaintext = SwapPlaintext::new(
            &mut self.rng,
            trading_pair,
            delta_1,
            delta_2,
            swap_claim_fee,
            claim_address,
        );

        let swap = SwapPlan::new(&mut self.rng, swap_plaintext).into();
        self.action(swap);

        Ok(self)
    }

    /// Perform a swap claim based on an input swap NFT with a pre-paid fee.
    #[instrument(skip(self))]
    pub fn swap_claim(&mut self, plan: SwapClaimPlan) -> &mut Self {
        self.action(plan.into());
        self
    }

    /// Add a delegation to this transaction.
    ///
    /// If you don't specify spends or outputs as well, they will be filled in automatically.
    #[instrument(skip(self))]
    pub fn delegate(
        &mut self,
        epoch: Epoch,
        unbonded_amount: Amount,
        rate_data: RateData,
    ) -> &mut Self {
        let delegation = rate_data.build_delegate(epoch, unbonded_amount).into();
        self.action(delegation);
        self
    }

    /// Add an undelegation to this transaction.
    #[instrument(skip(self))]
    pub fn undelegate(
        &mut self,
        epoch: Epoch,
        delegation_amount: Amount,
        rate_data: RateData,
    ) -> &mut Self {
        let undelegation = rate_data.build_undelegate(epoch, delegation_amount).into();
        self.action(undelegation);
        self
    }

    /// Add an undelegate claim to this transaction.
    #[instrument(skip(self))]
    pub fn undelegate_claim(&mut self, claim_plan: UndelegateClaimPlan) -> &mut Self {
        self.action(ActionPlan::UndelegateClaim(claim_plan));
        self
    }

    /// Upload a validator definition in this transaction.
    #[instrument(skip(self))]
    pub fn validator_definition(&mut self, new_validator: validator::Definition) -> &mut Self {
        self.action(ActionPlan::ValidatorDefinition(new_validator));
        self
    }

    /// Submit a new governance proposal in this transaction.
    #[instrument(skip(self))]
    pub fn proposal_submit(&mut self, proposal: Proposal, deposit_amount: Amount) -> &mut Self {
        self.action(ActionPlan::ProposalSubmit(ProposalSubmit {
            proposal,
            deposit_amount,
        }));
        self
    }

    /// Withdraw a governance proposal in this transaction.
    #[instrument(skip(self))]
    pub fn proposal_withdraw(&mut self, proposal: u64, reason: String) -> &mut Self {
        self.action(ActionPlan::ProposalWithdraw(ProposalWithdraw {
            proposal,
            reason,
        }));
        self
    }

    /// Claim a governance proposal deposit in this transaction.
    #[instrument(skip(self))]
    pub fn proposal_deposit_claim(
        &mut self,
        proposal: u64,
        deposit_amount: Amount,
        outcome: proposal_state::Outcome<()>,
    ) -> &mut Self {
        self.action(ActionPlan::ProposalDepositClaim(ProposalDepositClaim {
            proposal,
            deposit_amount,
            outcome,
        }));
        self
    }

    /// Deposit a value into the Community Pool.
    #[instrument(skip(self))]
    pub fn community_pool_deposit(&mut self, value: Value) -> &mut Self {
        self.action(ActionPlan::CommunityPoolDeposit(CommunityPoolDeposit {
            value,
        }));
        self
    }

    /// Cast a validator vote in this transaction.
    #[instrument(skip(self))]
    pub fn validator_vote(&mut self, vote: ValidatorVote) -> &mut Self {
        self.action(ActionPlan::ValidatorVote(vote));
        self
    }

    /// Perform an ICS-20 withdrawal
    #[instrument(skip(self))]
    pub fn ics20_withdrawal(&mut self, withdrawal: Ics20Withdrawal) -> &mut Self {
        self.action(ActionPlan::Ics20Withdrawal(withdrawal));
        self
    }

    /// Perform an IBC action
    #[instrument(skip(self))]
    pub fn ibc_action(&mut self, ibc_action: IbcRelay) -> &mut Self {
        self.action(ActionPlan::IbcAction(ibc_action));
        self
    }

    /// Vote with all possible vote weight on a given proposal.
    #[instrument(skip_all)]
    pub async fn delegator_vote<V: ViewClient>(
        // TODO this sucks, why isn't there a bundle of proposal data to use for voting
        // how is that not the thing returned by the rpc? why do we have to query a bunch of shit
        // independently and stitch it together?
        &mut self,
        view: &mut V,
        source: AddressIndex,
        proposal: u64,
        vote: Vote,
        start_block_height: u64,
        start_position: tct::Position,
        start_rate_data: BTreeMap<IdentityKey, RateData>,
    ) -> Result<&mut Self, anyhow::Error> {
        let voting_notes = view
            .notes_for_voting(NotesForVotingRequest {
                votable_at_height: start_block_height,
                address_index: Some(source.into()),
            })
            .await?;

        anyhow::ensure!(
            !voting_notes.is_empty(),
            "no notes were found for voting on proposal {}",
            proposal
        );

        // 1. Create a DelegatorVotePlan for each votable note.
        for (record, ik) in &voting_notes {
            let validator_start_rate_data = start_rate_data
                .get(&ik)
                .ok_or_else(|| anyhow!("missing rate data for votable note delegated to {}", ik))?;

            let voting_power_at_vote_start =
                validator_start_rate_data.unbonded_amount(record.note.amount());

            // 1. Create a DelegatorVotePlan that votes with this note on the proposal.
            let plan = DelegatorVotePlan::new(
                &mut self.rng,
                proposal,
                start_position,
                vote,
                record.note.clone(),
                record.position,
                voting_power_at_vote_start,
            );
            self.delegator_vote_precise(plan);
        }

        // 2. Here, we could sweep any spendable notes with delegation tokens to
        // a new output to try to unlink them from a future vote.  In practice
        // this is meaningless because we don't have flow encryption, so
        // delegator votes reveal the precise amount, and this amount will
        // likely be unique to the delegator and enough to link their votes.
        // Also, because we're in a single transaction, the pattern of
        // delegations will also be revealed (vs creating distinct transactions
        // for each validator).
        //
        // So instead, we do nothing.

        Ok(self)
    }

    /// Vote with a specific positioned note in the transaction.
    ///
    /// If you don't use this method to specify votes, they will be filled in automatically from the
    /// implied voting intent by [`vote`](Planner::vote) when the plan is
    /// [`finish`](Planner::finish)ed.
    #[instrument(skip(self, plan))]
    pub fn delegator_vote_precise(&mut self, plan: DelegatorVotePlan) -> &mut Self {
        self.push(plan.into());

        self
    }

    fn balance(&self) -> Balance {
        let mut balance = Balance::zero();
        for action in &self.actions {
            balance += action.balance();
        }
        for action in self.change_outputs.values() {
            balance += action.balance();
        }
        balance
    }

    fn push(&mut self, action: ActionPlan) {
        self.actions.push(action);
    }

    /// Estimate the gas cost for the transaction, based on the actions in the plan,
    /// and the change outputs.
    ///
    /// This does not include the gas cost for the tx bytes itself, so the gas estimate always
    /// *undershoots*. We typically add them separately, deducting from the change outputs.
    fn gas_estimate(&self) -> Gas {
        let mut gas = Gas::zero();
        for action in &self.actions {
            gas += action.gas_cost();
        }
        for action in self.change_outputs.values() {
            gas += ActionPlan::from(action.clone()).gas_cost();
        }

        gas
    }

    /// Estimate the fee for each action and output in the transaction, scaled by a fee tier.
    fn fee_estimate(&self, gas_prices: &GasPrices, fee_tier: &FeeTier) -> Fee {
        let base_fee = Fee::from_staking_token_amount(gas_prices.fee(&self.gas_estimate()));
        let fee = base_fee.apply_tier(*fee_tier);

        fee
    }

    /// Return a total balance for the transaction, deducting fees for each action and change notes.
    fn balance_with_fee_estimate(&self, gas_prices: &GasPrices, fee_tier: &FeeTier) -> Balance {
        self.balance() - self.fee_estimate(gas_prices, fee_tier).0
    }

    /// Actualize the change outputs for the transaction, based on the current balance.
    fn refresh_change(&mut self, change_address: Address) {
        self.change_outputs = BTreeMap::new();
        // For each "provided" balance component, create a change note.
        for value in self.balance().provided() {
            self.change_outputs.insert(
                value.asset_id,
                OutputPlan::new(&mut OsRng, value, change_address.clone()),
            );
        }
    }

    /// Deduct the fee from the change outputs, if possible.
    fn adjust_change_for_fee(&mut self, fee: Fee) {
        self.change_outputs.entry(fee.0.asset_id).and_modify(|e| {
            e.value.amount = e.value.amount.saturating_sub(&fee.0.amount);
        });
    }

    /// Prioritize notes to spend to release value of a specific transaction.
    ///
    /// Various logic is possible for note selection. Currently, this method
    /// prioritizes notes sent to a one-time address, then notes with the largest
    /// value:
    ///
    /// - Prioritizing notes sent to one-time addresses optimizes for a future in
    /// which we implement DAGSync keyed by fuzzy message detection (which will not
    /// be able to detect notes sent to one-time addresses). Spending these notes
    /// immediately converts them into change notes, sent to the default address for
    /// the users' account, which are detectable.
    ///
    /// - Prioritizing notes with the largest value optimizes for gas used by the
    /// transaction.
    ///
    /// We may want to make note prioritization configurable in the future. For
    /// instance, a user might prefer a note prioritization strategy that harvested
    /// capital losses when possible, using cost basis information retained by the
    /// view server.
    fn prioritize_and_filter_spendable_notes(
        records: Vec<SpendableNoteRecord>,
    ) -> Vec<SpendableNoteRecord> {
        let mut filtered = records
            .into_iter()
            .filter(|record| record.note.amount() > Amount::zero())
            .collect::<Vec<_>>();

        filtered.sort_by(|a, b| {
            // Sort by whether the note was sent to an ephemeral address...
            match (
                a.address_index.is_ephemeral(),
                b.address_index.is_ephemeral(),
            ) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                // ... then by largest amount.
                _ => b.note.amount().cmp(&a.note.amount()),
            }
        });

        filtered
    }

    fn action(&mut self, action: ActionPlan) -> &mut Self {
        // Add the action to the plan
        self.plan.actions.push(action);
        self
    }

    // Collect and tally all the surplus value balance from `SwapClaim` actions.
    fn swap_claim_surplus(&self) -> Fee {
        let total = self
            .actions
            .iter()
            .filter(|action| matches!(action, ActionPlan::SwapClaim(_)))
            .map(|action| match action {
                ActionPlan::SwapClaim(claim) => {
                    // Multi-asset fees require changing this logic so that we tally `Balance` for fees,
                    // and mint notes opportunistically. Without changing this logic, the transaction won't
                    // balance if it has prepaid fees that are not staking tokens.
                    claim.swap_plaintext.claim_fee.amount()
                }
                _ => Amount::zero(),
            })
            .sum();

        Fee::from_staking_token_amount(total)
    }

    /// Add spends and change outputs as required to balance the transaction, using the view service
    /// provided to supply the notes and other information.
    ///
    /// Clears the contents of the planner, which can be re-used.
    pub async fn plan<V: ViewClient>(
        &mut self,
        view: &mut V,
        source: AddressIndex,
    ) -> anyhow::Result<TransactionPlan> {
        // Gather all the information needed from the view service
        let app_params = view.app_params().await?;
        let chain_id = app_params.chain_id.clone();
        let fmd_params = view.fmd_parameters().await?;

        // Caller has already processed all the user-supplied intents into complete action plans.
        self.actions = self.plan.actions.clone();

        // Change address represents the sender's address.
        let change_address = view.address_by_index(source).await?.clone();

        // It's possible that adding spends could increase the gas, increasing the fee
        // amount, and so on, so we add spends iteratively.
        let mut notes_by_asset_id = BTreeMap::new();

        for required in self
            .balance_with_fee_estimate(&self.gas_prices, &self.fee_tier)
            .required()
        {
            // Find all the notes of this asset in the source account.
            let records: Vec<SpendableNoteRecord> = view
                .notes(NotesRequest {
                    include_spent: false,
                    asset_id: Some(required.asset_id.into()),
                    address_index: Some(source.into()),
                    amount_to_spend: None,
                })
                .await?;
            notes_by_asset_id.insert(
                required.asset_id,
                Self::prioritize_and_filter_spendable_notes(records),
            );
        }

        let mut iterations = 0usize;

        while let Some(required) = self
            .balance_with_fee_estimate(&self.gas_prices, &self.fee_tier)
            .required()
            .next()
        {
            // Spend a single note towards the required balance, if possible.
            let Some(note) = notes_by_asset_id
                .get_mut(&required.asset_id)
                .expect("we already queried")
                .pop()
            else {
                return Err(anyhow!(
                    "ran out of notes to spend while planning transaction, need {} of asset {}",
                    required.amount,
                    required.asset_id,
                )
                .into());
            };
            self.actions
                .push(SpendPlan::new(&mut OsRng, note.note, note.position).into());

            // Recompute the change outputs, without accounting for fees.
            self.refresh_change(change_address.clone());
            // Now re-estimate the fee of the updated transaction and adjust the change if possible.
            let fee = self.fee_estimate(&self.gas_prices, &self.fee_tier);
            self.adjust_change_for_fee(fee);

            iterations = iterations + 1;
            if iterations > 100 {
                return Err(anyhow!("failed to plan transaction after 100 iterations").into());
            }
        }

        let fee = self.fee_estimate(&self.gas_prices, &self.fee_tier);

        // At this point, we should have a fully balanced transaction, unless:
        // - We lack enough notes to cover the required balance
        // - We have surplus value that we need to shed (or capture)
        //
        // The latter can happen with swap claims, for example, since they are equipped
        // with a pre-paid fee. If we detect a surplus, we have to decide what to do with it.
        // One option would be to a new note with the surplus value, but that could potentially
        // increase the fee, ahead of the prepaid surplus available. This thing is a proper
        // state machine, and since I want to go bed, we'll just release it into the transaction
        // fee directly.
        let swap_claim_surplus = self.swap_claim_surplus();

        tracing::debug!(?swap_claim_surplus, "detected swap claim surplus value");

        let total_fee =
            Fee::from_staking_token_amount(fee.0.amount.max(swap_claim_surplus.0.amount));
        let expiry_height = self.plan.transaction_parameters.expiry_height;

        let mut plan = TransactionPlan {
            actions: self
                .actions
                .clone()
                .into_iter()
                .chain(self.change_outputs.clone().into_values().map(Into::into))
                .collect(),
            transaction_parameters: TransactionParameters {
                expiry_height,
                chain_id,
                fee: total_fee,
            },
            detection_data: None,
            memo: None,
        };

        if let Some(memo_plan) = self.plan.memo.clone() {
            plan.memo = Some(MemoPlan::new(&mut OsRng, memo_plan.plaintext)?);
        } else if plan.output_plans().next().is_some() {
            // If a memo was not provided, but is required (because we have outputs),
            // auto-create one with the change address.
            plan.memo = Some(MemoPlan::new(
                &mut OsRng,
                MemoPlaintext::new(change_address, String::new())?,
            )?);
        }
        plan.populate_detection_data(&mut OsRng, fmd_params.precision_bits.into());
        self.plan = plan;

        tracing::info!("finished balancing transaction");
        /* Wrap-up planning, display stats, reset state */
        // We add some fail-fast checks to give callers a helpful error message if the transaction
        // does not balance.
        ensure!(
            !self.plan.actions.is_empty(),
            "the transaction contains no actions"
        );

        let final_balance = self.balance() - total_fee.0;
        ensure!(
            final_balance.is_zero(),
            "the transaction is not balanced: {:?}",
            final_balance
        );

        // Reset the internal state
        self.ibc_actions = Vec::new();
        self.gas_prices = GasPrices::zero();
        self.change_outputs = BTreeMap::new();
        self.actions = Vec::new();
        let plan = mem::take(&mut self.plan);

        Ok(plan)
    }
}
