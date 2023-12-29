use decaf377_fmd::Clue;
use penumbra_chain::EffectHash;
use penumbra_dao::{DaoDeposit, DaoOutput, DaoSpend};
use penumbra_dex::{
    lp::action::{PositionClose, PositionOpen, PositionRewardClaim, PositionWithdraw},
    swap, swap_claim,
};
use penumbra_fee::Fee;
use penumbra_governance::{
    DelegatorVote, DelegatorVoteBody, Proposal, ProposalDepositClaim, ProposalSubmit,
    ProposalWithdraw, ValidatorVote, ValidatorVoteBody, Vote,
};
use penumbra_ibc::IbcRelay;
use penumbra_keys::{FullViewingKey, PayloadKey};
use penumbra_proto::{
    core::component::dex::v1alpha1 as pbd, core::component::fee::v1alpha1 as pbf,
    core::component::governance::v1alpha1 as pbg, core::component::ibc::v1alpha1 as pbi,
    core::component::shielded_pool::v1alpha1 as pb_sp, core::component::stake::v1alpha1 as pbs,
    core::transaction::v1alpha1 as pbt, crypto::decaf377_fmd::v1alpha1 as pb_fmd, Message, Name,
};
use penumbra_shielded_pool::{output, spend, Ics20Withdrawal};
use penumbra_stake::{validator, Delegate, Undelegate, UndelegateClaimBody};

use crate::{
    memo::MemoCiphertext, plan::TransactionPlan, Action, DetectionData, Transaction,
    TransactionBody, TransactionParameters,
};

// Note: temporarily duplicate of chain/EffectingData
pub trait EffectingData {
    fn effect_hash(&self) -> EffectHash;
}

impl<'a, T: penumbra_chain::EffectingData> EffectingData for crate::Compat<'a, T> {
    fn effect_hash(&self) -> EffectHash {
        self.0.effect_hash()
    }
}

impl Transaction {
    pub fn effect_hash(&self) -> EffectHash {
        self.transaction_body.effect_hash()
    }
}

impl TransactionBody {
    pub fn expiry_height(&self) -> u64 {
        self.transaction_parameters.expiry_height
    }

    pub fn chain_id(&self) -> &str {
        &self.transaction_parameters.chain_id
    }

    pub fn effect_hash(&self) -> EffectHash {
        let mut state = create_personalized_state(&pbt::TransactionBody::type_url());

        // Hash the fixed data of the transaction body.
        state.update(self.transaction_parameters.effect_hash().as_bytes());
        if self.memo.is_some() {
            let memo_ciphertext = self.memo.clone();
            state.update(
                memo_ciphertext
                    .expect("memo is some")
                    .effect_hash()
                    .as_bytes(),
            );
        }
        if self.detection_data.is_some() {
            let detection_data = self.detection_data.clone();
            state.update(
                detection_data
                    .expect("detection data is some")
                    .effect_hash()
                    .as_bytes(),
            );
        }

        // Hash the number of actions, then each action.
        let num_actions = self.actions.len() as u32;
        state.update(&num_actions.to_le_bytes());
        for action in &self.actions {
            state.update(action.effect_hash().as_bytes());
        }

        EffectHash(state.finalize().as_array().clone())
    }
}

impl TransactionPlan {
    /// Computes the [`EffectHash`] for the [`Transaction`] described by this
    /// [`TransactionPlan`].
    ///
    /// This method does not require constructing the entire [`Transaction`],
    /// but it does require the associated [`FullViewingKey`] to derive
    /// effecting data that will be fed into the [`EffectHash`].
    pub fn effect_hash(&self, fvk: &FullViewingKey) -> EffectHash {
        // This implementation is identical to the one above, except that we
        // don't need to actually construct the entire `TransactionBody` with
        // complete `Action`s, we just need to construct the bodies of the
        // actions the transaction will have when constructed.

        let mut state = create_personalized_state(&pbt::TransactionBody::type_url());

        // Hash the fixed data of the transaction body.
        state.update(self.transaction_parameters.effect_hash().as_bytes());

        // Hash the memo and save the memo key for use with outputs later.
        let mut memo_key: Option<PayloadKey> = None;
        if self.memo_data.is_some() {
            let memo_plan = self
                .memo_data
                .clone()
                .expect("memo_plan must be present in TransactionPlan");
            let memo_ciphertext = memo_plan.memo().expect("can compute ciphertext");
            state.update(memo_ciphertext.effect_hash().as_bytes());
            memo_key = Some(memo_plan.key);
        }

        // Hash the detection data.
        if !self.detection_data.clue_plans.is_empty() {
            let detection_data = self.detection_data.detection_data();
            state.update(detection_data.effect_hash().as_bytes());
        }

        let num_actions = self.actions.len() as u32;
        state.update(&num_actions.to_le_bytes());

        // If the memo_key is None, then there is no memo, and we populate the memo key
        // field with a dummy key.
        let dummy_payload_key: PayloadKey = [0u8; 32].into();

        // Hash the effecting data of each action, in the order it appears in the plan,
        // which will be the order it appears in the transaction.
        for action_plan in &self.actions {
            state.update(
                action_plan
                    .effect_hash(fvk, memo_key.as_ref().unwrap_or(&dummy_payload_key))
                    .as_bytes(),
            );
        }

        EffectHash(state.finalize().as_array().clone())
    }
}

impl EffectingData for Action {
    fn effect_hash(&self) -> EffectHash {
        match self {
            Action::Output(output) => output.body.effect_hash(),
            Action::Spend(spend) => spend.body.effect_hash(),
            Action::Delegate(delegate) => delegate.effect_hash(),
            Action::Undelegate(undelegate) => undelegate.effect_hash(),
            Action::UndelegateClaim(claim) => claim.body.effect_hash(),
            Action::ProposalSubmit(submit) => submit.effect_hash(),
            Action::ProposalWithdraw(withdraw) => withdraw.effect_hash(),
            Action::ProposalDepositClaim(claim) => claim.effect_hash(),
            Action::DelegatorVote(vote) => vote.effect_hash(),
            Action::ValidatorVote(vote) => vote.effect_hash(),
            Action::SwapClaim(swap_claim) => swap_claim.body.effect_hash(),
            Action::Swap(swap) => swap.body.effect_hash(),
            Action::ValidatorDefinition(defn) => defn.effect_hash(),
            Action::IbcRelay(payload) => payload.effect_hash(),
            Action::PositionOpen(p) => p.effect_hash(),
            Action::PositionClose(p) => p.effect_hash(),
            Action::PositionWithdraw(p) => p.effect_hash(),
            Action::PositionRewardClaim(p) => p.effect_hash(),
            Action::Ics20Withdrawal(w) => w.effect_hash(),
            Action::DaoSpend(d) => d.effect_hash(),
            Action::DaoOutput(d) => d.effect_hash(),
            Action::DaoDeposit(d) => d.effect_hash(),
        }
    }
}

/// A helper function to hash the data of a proto-encoded message, using
/// the variable-length `TypeUrl` of the corresponding domain type as a
/// personalization string.
fn hash_proto_effecting_data<M: Message>(personalization: &str, message: &M) -> EffectHash {
    let mut state = create_personalized_state(personalization);
    state.update(&message.encode_to_vec());

    EffectHash(*state.finalize().as_array())
}

/// A helper function to create a BLAKE2b `State` instance given a variable-length personalization string.
fn create_personalized_state(personalization: &str) -> blake2b_simd::State {
    let mut state = blake2b_simd::State::new();

    // The `TypeUrl` provided as a personalization string is variable length,
    // so we first include the length in bytes as a fixed-length prefix.
    let length = personalization.len() as u64;
    state.update(&length.to_le_bytes());
    state.update(personalization.as_bytes());

    state
}

impl EffectingData for validator::Definition {
    fn effect_hash(&self) -> EffectHash {
        let effecting_data: pbs::ValidatorDefinition = self.clone().into();
        hash_proto_effecting_data(&pbs::ValidatorDefinition::type_url(), &effecting_data)
    }
}

impl EffectingData for IbcRelay {
    fn effect_hash(&self) -> EffectHash {
        let effecting_data: pbi::IbcRelay = self.clone().into();
        hash_proto_effecting_data(&pbi::IbcRelay::type_url(), &effecting_data)
    }
}

impl EffectingData for Ics20Withdrawal {
    fn effect_hash(&self) -> EffectHash {
        let effecting_data: pbi::Ics20Withdrawal = self.clone().into();
        hash_proto_effecting_data(&pbi::Ics20Withdrawal::type_url(), &effecting_data)
    }
}

impl EffectingData for output::Body {
    fn effect_hash(&self) -> EffectHash {
        // The effecting data is in the body of the output, so we can
        // just use hash the proto-encoding of the body.
        let body: pb_sp::OutputBody = self.clone().into();
        hash_proto_effecting_data(&pb_sp::OutputBody::type_url(), &body)
    }
}

impl EffectingData for spend::Body {
    fn effect_hash(&self) -> EffectHash {
        // The effecting data is in the body of the spend, so we can
        // just use hash the proto-encoding of the body.
        let body: pb_sp::SpendBody = self.clone().into();
        hash_proto_effecting_data(&pb_sp::SpendBody::type_url(), &body)
    }
}

impl EffectingData for DaoDeposit {
    fn effect_hash(&self) -> EffectHash {
        let effecting_data: pbg::DaoDeposit = self.clone().into();
        hash_proto_effecting_data(&pbg::DaoDeposit::type_url(), &effecting_data)
    }
}

impl EffectingData for DaoSpend {
    fn effect_hash(&self) -> EffectHash {
        let effecting_data: pbg::DaoSpend = self.clone().into();
        hash_proto_effecting_data(&pbg::DaoSpend::type_url(), &effecting_data)
    }
}

impl EffectingData for DaoOutput {
    fn effect_hash(&self) -> EffectHash {
        let effecting_data: pbg::DaoOutput = self.clone().into();
        hash_proto_effecting_data(&pbg::DaoOutput::type_url(), &effecting_data)
    }
}

impl EffectingData for swap::Body {
    fn effect_hash(&self) -> EffectHash {
        // The effecting data is in the body of the swap, so we can
        // just use hash the proto-encoding of the body.
        let effecting_data: pbd::SwapBody = self.clone().into();
        hash_proto_effecting_data(&pbd::SwapBody::type_url(), &effecting_data)
    }
}

impl EffectingData for swap_claim::Body {
    fn effect_hash(&self) -> EffectHash {
        // The effecting data is in the body of the swap claim, so we can
        // just use hash the proto-encoding of the body.
        let effecting_data: pbd::SwapClaimBody = self.clone().into();
        hash_proto_effecting_data(&pbd::SwapClaimBody::type_url(), &effecting_data)
    }
}

impl EffectingData for Delegate {
    fn effect_hash(&self) -> EffectHash {
        // For delegations, the entire action is considered effecting data.
        let effecting_data: pbs::Delegate = self.clone().into();
        hash_proto_effecting_data(&pbs::Delegate::type_url(), &effecting_data)
    }
}

impl EffectingData for Undelegate {
    fn effect_hash(&self) -> EffectHash {
        // For undelegations, the entire action is considered effecting data.
        let effecting_data: pbs::Undelegate = self.clone().into();
        hash_proto_effecting_data(&pbs::Undelegate::type_url(), &effecting_data)
    }
}

impl EffectingData for UndelegateClaimBody {
    fn effect_hash(&self) -> EffectHash {
        // The effecting data is in the body of the undelegate claim, so we can
        // just use hash the proto-encoding of the body.
        let effecting_data: pbs::UndelegateClaimBody = self.clone().into();
        hash_proto_effecting_data(&pbs::UndelegateClaimBody::type_url(), &effecting_data)
    }
}

impl EffectingData for Proposal {
    fn effect_hash(&self) -> EffectHash {
        let effecting_data: pbg::Proposal = self.clone().into();
        hash_proto_effecting_data(&pbg::Proposal::type_url(), &effecting_data)
    }
}

impl EffectingData for ProposalSubmit {
    fn effect_hash(&self) -> EffectHash {
        let effecting_data: pbg::ProposalSubmit = self.clone().into();
        hash_proto_effecting_data(&pbg::ProposalSubmit::type_url(), &effecting_data)
    }
}

impl EffectingData for ProposalWithdraw {
    fn effect_hash(&self) -> EffectHash {
        let effecting_data: pbg::ProposalWithdraw = self.clone().into();
        hash_proto_effecting_data(&pbg::ProposalWithdraw::type_url(), &effecting_data)
    }
}

impl EffectingData for ValidatorVote {
    fn effect_hash(&self) -> EffectHash {
        self.body.effect_hash()
    }
}

impl EffectingData for DelegatorVote {
    fn effect_hash(&self) -> EffectHash {
        self.body.effect_hash()
    }
}

impl EffectingData for Vote {
    fn effect_hash(&self) -> EffectHash {
        let effecting_data: pbg::Vote = self.clone().into();
        hash_proto_effecting_data(&pbg::Vote::type_url(), &effecting_data)
    }
}

impl EffectingData for ValidatorVoteBody {
    fn effect_hash(&self) -> EffectHash {
        let effecting_data: pbg::ValidatorVoteBody = self.clone().into();
        hash_proto_effecting_data(&pbg::ValidatorVoteBody::type_url(), &effecting_data)
    }
}

impl EffectingData for DelegatorVoteBody {
    fn effect_hash(&self) -> EffectHash {
        let effecting_data: pbg::DelegatorVoteBody = self.clone().into();
        hash_proto_effecting_data(&pbg::DelegatorVoteBody::type_url(), &effecting_data)
    }
}

impl EffectingData for ProposalDepositClaim {
    fn effect_hash(&self) -> EffectHash {
        let effecting_data: pbg::ProposalDepositClaim = self.clone().into();
        hash_proto_effecting_data(&pbg::ProposalDepositClaim::type_url(), &effecting_data)
    }
}

impl EffectingData for PositionOpen {
    fn effect_hash(&self) -> EffectHash {
        // The position open action consists only of the position, which
        // we consider effecting data.
        let effecting_data: pbd::PositionOpen = self.clone().into();
        hash_proto_effecting_data(&pbd::PositionOpen::type_url(), &effecting_data)
    }
}

impl EffectingData for PositionClose {
    fn effect_hash(&self) -> EffectHash {
        let effecting_data: pbd::PositionClose = self.clone().into();
        hash_proto_effecting_data(&pbd::PositionClose::type_url(), &effecting_data)
    }
}

impl EffectingData for PositionWithdraw {
    fn effect_hash(&self) -> EffectHash {
        let effecting_data: pbd::PositionWithdraw = self.clone().into();
        hash_proto_effecting_data(&pbd::PositionWithdraw::type_url(), &effecting_data)
    }
}

impl EffectingData for PositionRewardClaim {
    fn effect_hash(&self) -> EffectHash {
        let effecting_data: pbd::PositionRewardClaim = self.clone().into();
        hash_proto_effecting_data(&pbd::PositionRewardClaim::type_url(), &effecting_data)
    }
}

impl EffectingData for DetectionData {
    fn effect_hash(&self) -> EffectHash {
        let effecting_data: pbt::DetectionData = self.clone().into();
        hash_proto_effecting_data(&pbt::DetectionData::type_url(), &effecting_data)
    }
}

impl EffectingData for Clue {
    fn effect_hash(&self) -> EffectHash {
        let data: pb_fmd::Clue = self.clone().into();
        hash_proto_effecting_data(&pb_fmd::Clue::type_url(), &data)
    }
}

impl EffectingData for TransactionParameters {
    fn effect_hash(&self) -> EffectHash {
        let params: pbt::TransactionParameters = self.clone().into();
        hash_proto_effecting_data(&pbt::TransactionParameters::type_url(), &params)
    }
}

impl EffectingData for Fee {
    fn effect_hash(&self) -> EffectHash {
        let proto_encoded_fee: pbf::Fee = self.clone().into();
        hash_proto_effecting_data(&pbf::Fee::type_url(), &proto_encoded_fee)
    }
}

impl EffectingData for MemoCiphertext {
    fn effect_hash(&self) -> EffectHash {
        let proto_encoded_memo: pbt::MemoCiphertext = self.clone().into();
        hash_proto_effecting_data(&pbt::MemoCiphertext::type_url(), &proto_encoded_memo)
    }
}

#[cfg(test)]
mod tests {
    use penumbra_asset::{asset, Value, STAKING_TOKEN_ASSET_ID};
    use penumbra_dex::{swap::SwapPlaintext, swap::SwapPlan, TradingPair};
    use penumbra_fee::Fee;
    use penumbra_keys::{
        keys::{Bip44Path, SeedPhrase, SpendKey},
        Address,
    };
    use penumbra_shielded_pool::Note;
    use penumbra_shielded_pool::{OutputPlan, SpendPlan};
    use penumbra_tct as tct;
    use rand_core::OsRng;

    use crate::{
        memo::MemoPlaintext,
        plan::{CluePlan, DetectionDataPlan, MemoPlan, TransactionPlan},
        TransactionParameters, WitnessData,
    };

    /// This isn't an exhaustive test, but we don't currently have a
    /// great way to generate actions for randomized testing.
    ///
    /// All we hope to check here is that, for a basic transaction plan,
    /// we compute the same auth hash for the plan and for the transaction.
    #[test]
    fn plan_effect_hash_matches_transaction_effect_hash() {
        let rng = OsRng;
        let seed_phrase = SeedPhrase::generate(rng);
        let sk = SpendKey::from_seed_phrase_bip44(seed_phrase, &Bip44Path::new(0));
        let fvk = sk.full_viewing_key();
        let (addr, _dtk) = fvk.incoming().payment_address(0u32.into());

        let mut sct = tct::Tree::new();

        let note0 = Note::generate(
            &mut OsRng,
            &addr,
            Value {
                amount: 10000u64.into(),
                asset_id: *STAKING_TOKEN_ASSET_ID,
            },
        );
        let note1 = Note::generate(
            &mut OsRng,
            &addr,
            Value {
                amount: 20000u64.into(),
                asset_id: *STAKING_TOKEN_ASSET_ID,
            },
        );

        sct.insert(tct::Witness::Keep, note0.commit()).unwrap();
        sct.insert(tct::Witness::Keep, note1.commit()).unwrap();

        let trading_pair = TradingPair::new(
            asset::Cache::with_known_assets()
                .get_unit("nala")
                .unwrap()
                .id(),
            asset::Cache::with_known_assets()
                .get_unit("upenumbra")
                .unwrap()
                .id(),
        );

        let swap_plaintext = SwapPlaintext::new(
            &mut OsRng,
            trading_pair,
            100000u64.into(),
            1u64.into(),
            Fee(Value {
                amount: 3u64.into(),
                asset_id: asset::Cache::with_known_assets()
                    .get_unit("upenumbra")
                    .unwrap()
                    .id(),
            }),
            addr,
        );

        let mut rng = OsRng;

        let memo_plaintext = MemoPlaintext {
            return_address: Address::dummy(&mut rng),
            text: "".to_string(),
        };
        let plan = TransactionPlan {
            // Put outputs first to check that the auth hash
            // computation is not affected by plan ordering.
            actions: vec![
                OutputPlan::new(
                    &mut OsRng,
                    Value {
                        amount: 30000u64.into(),
                        asset_id: *STAKING_TOKEN_ASSET_ID,
                    },
                    addr.clone(),
                )
                .into(),
                SpendPlan::new(&mut OsRng, note0, 0u64.into()).into(),
                SpendPlan::new(&mut OsRng, note1, 1u64.into()).into(),
                SwapPlan::new(&mut OsRng, swap_plaintext).into(),
            ],
            transaction_parameters: TransactionParameters {
                expiry_height: 0,
                fee: Fee::default(),
                chain_id: "penumbra-test".to_string(),
            },
            detection_data: DetectionDataPlan {
                clue_plans: vec![CluePlan::new(&mut OsRng, addr, 1)],
            },
            memo_data: Some(MemoPlan::new(&mut OsRng, memo_plaintext.clone()).unwrap()),
        };

        println!("{}", serde_json::to_string_pretty(&plan).unwrap());

        let plan_effect_hash = plan.effect_hash(fvk);

        let auth_data = plan.authorize(rng, &sk);
        let witness_data = WitnessData {
            anchor: sct.root(),
            state_commitment_proofs: plan
                .spend_plans()
                .map(|spend: &SpendPlan| {
                    (
                        spend.note.commit(),
                        sct.witness(spend.note.commit()).unwrap(),
                    )
                })
                .collect(),
        };
        let transaction = plan.build(fvk, &witness_data, &auth_data).unwrap();

        let transaction_effect_hash = transaction.effect_hash();

        assert_eq!(plan_effect_hash, transaction_effect_hash);

        let decrypted_memo = transaction.decrypt_memo(fvk).expect("can decrypt memo");
        assert_eq!(decrypted_memo, memo_plaintext);

        // TODO: fix this and move into its own test?
        // // Also check the concurrent build results in the same effect hash.
        // let rt = Runtime::new().unwrap();
        // let transaction = rt
        //     .block_on(async move {
        //         plan.build_concurrent(&mut OsRng, fvk, auth_data, witness_data)
        //             .await
        //     })
        //     .expect("can build");
        // assert_eq!(plan_effect_hash, transaction.effect_hash());
    }
}
