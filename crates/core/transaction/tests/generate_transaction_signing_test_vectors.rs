use decaf377::{Fq, Fr};
use decaf377_rdsa::{SigningKey, SpendAuth, VerificationKey, VerificationKeyBytes};
use ed25519_consensus::SigningKey as Ed25519SigningKey;
use ibc_proto::ics23::CommitmentProof;
use ibc_types::core::{
    channel::{msgs::MsgRecvPacket, packet::Sequence, ChannelId, Packet, PortId},
    client::Height,
    commitment::MerkleProof,
};
use ibc_types::timestamp::Timestamp;
use penumbra_asset::asset::{Id, Metadata};
use penumbra_asset::{asset, Value, ValueView};
use penumbra_auction::auction::{
    dutch::{
        actions::{
            ActionDutchAuctionEnd, ActionDutchAuctionSchedule, ActionDutchAuctionWithdrawPlan,
        },
        DutchAuctionDescription,
    },
    AuctionId,
};
use penumbra_community_pool::{CommunityPoolDeposit, CommunityPoolOutput, CommunityPoolSpend};
use penumbra_dex::{
    lp::{
        plan::PositionWithdrawPlan,
        position::{Position, State as PositionState},
        Reserves, TradingFunction,
    },
    swap::{SwapPlaintext, SwapPlan},
    swap_claim::SwapClaimPlan,
    BatchSwapOutputData, PositionClose, PositionOpen, TradingPair,
};
use penumbra_fee::Fee;
use penumbra_governance::{
    proposal_state::{Outcome as ProposalOutcome, Withdrawn},
    DelegatorVotePlan, Proposal, ProposalDepositClaim, ProposalPayload, ProposalSubmit,
    ProposalWithdraw, ValidatorVote, ValidatorVoteBody, ValidatorVoteReason, Vote,
};
use penumbra_ibc::IbcRelay;
use penumbra_keys::keys::{Bip44Path, SeedPhrase, SpendKey};
use penumbra_keys::test_keys::SEED_PHRASE;
use penumbra_keys::{Address, AddressView, FullViewingKey};
use penumbra_num::Amount;
use penumbra_proto::DomainType;
use penumbra_sct::epoch::Epoch;
use penumbra_shielded_pool::{Ics20Withdrawal, Note, OutputPlan, Rseed, SpendPlan};
use penumbra_stake::{
    validator, validator::Definition, Delegate, FundingStreams, GovernanceKey, IdentityKey,
    Penalty, Undelegate, UndelegateClaimPlan,
};
use penumbra_transaction::{
    memo::MemoPlaintext, plan::MemoPlan, ActionPlan, TransactionParameters, TransactionPlan,
};
use proptest::prelude::*;
use proptest::strategy::ValueTree;
use proptest::test_runner::{Config, TestRunner};
use rand_core::OsRng;
use serde_json::json;
use std::collections::HashMap;
use std::io::Write;
use std::str::FromStr;
use std::{fs, fs::File, io::Read};
use tendermint;

fn amount_strategy() -> impl Strategy<Value = Amount> {
    let inner_uint_range = 0u128..1_000_000_000_000_000_000u128;
    inner_uint_range.prop_map(|uint| Amount::from_le_bytes(uint.to_le_bytes()))
}

fn asset_id_strategy() -> impl Strategy<Value = Id> {
    Just(*penumbra_asset::STAKING_TOKEN_ASSET_ID)
}

fn value_strategy() -> impl Strategy<Value = penumbra_asset::Value> {
    (asset_id_strategy(), amount_strategy())
        .prop_map(|(asset_id, amount)| penumbra_asset::Value { amount, asset_id })
}

fn controlled_address_strategy(seed_phrase: SeedPhrase) -> impl Strategy<Value = Address> {
    (0u32..100u32).prop_map(move |index| {
        let sk = SpendKey::from_seed_phrase_bip44(seed_phrase.clone(), &Bip44Path::new(0));
        sk.full_viewing_key().payment_address(index.into()).0
    })
}

fn uncontrolled_address_strategy() -> impl Strategy<Value = Address> {
    // normally we would use address::dummy, but this seems to not work properly
    // for some reason (invalid key errors on computing effecthash.)
    prop::strategy::LazyJust::new(|| {
        let seed_phrase = SeedPhrase::generate(&mut OsRng);
        let sk = SpendKey::from_seed_phrase_bip44(seed_phrase, &Bip44Path::new(0));
        sk.full_viewing_key().payment_address(0u32.into()).0
    })
}

fn address_strategy(seed_phrase: SeedPhrase) -> impl Strategy<Value = Address> {
    prop_oneof![
        // 50% chance to generate a controlled address with a random index
        controlled_address_strategy(seed_phrase),
        // 50% chance to generate a random address
        uncontrolled_address_strategy()
    ]
}

fn note_strategy(addr: Address) -> impl Strategy<Value = Note> {
    value_strategy().prop_map(move |value| Note::generate(&mut OsRng, &addr, value))
}

fn spend_plan_strategy(fvk: &FullViewingKey) -> impl Strategy<Value = SpendPlan> {
    let tct_strategy = any::<penumbra_tct::Position>();
    let note_strategy = note_strategy(fvk.incoming().payment_address(0u32.into()).0);

    (tct_strategy, note_strategy)
        .prop_map(|(tct_pos, note)| SpendPlan::new(&mut OsRng, note, tct_pos))
}

fn output_plan_strategy(seed_phrase: SeedPhrase) -> impl Strategy<Value = OutputPlan> {
    (value_strategy(), address_strategy(seed_phrase.clone()))
        .prop_map(|(value, address)| OutputPlan::new(&mut OsRng, value, address))
}

fn identity_key_strategy() -> impl Strategy<Value = IdentityKey> {
    let rand_bytes = prop::array::uniform32(any::<u8>());

    rand_bytes.prop_map(|vk_bytes| IdentityKey(VerificationKeyBytes::<SpendAuth>::from(vk_bytes)))
}

fn delegate_plan_strategy() -> impl Strategy<Value = Delegate> {
    let epoch_index_strategy = 0..10000u64;
    let unbonded_amount_strategy = amount_strategy();
    let delegation_amount_strategy = amount_strategy();

    (
        identity_key_strategy(),
        epoch_index_strategy,
        unbonded_amount_strategy,
        delegation_amount_strategy,
    )
        .prop_map(
            |(validator_identity, epoch_index, unbonded_amount, delegation_amount)| Delegate {
                validator_identity,
                epoch_index,
                unbonded_amount,
                delegation_amount,
            },
        )
}

fn undelegate_plan_strategy() -> impl Strategy<Value = Undelegate> {
    let epoch_index_strategy = 0..10000u64;
    let unbonded_amount_strategy = amount_strategy();
    let delegation_amount_strategy = amount_strategy();
    (
        identity_key_strategy(),
        epoch_index_strategy,
        unbonded_amount_strategy,
        delegation_amount_strategy,
    )
        .prop_map(
            |(validator_identity, epoch_index, unbonded_amount, delegation_amount)| Undelegate {
                validator_identity,
                from_epoch: Epoch {
                    index: epoch_index,
                    start_height: epoch_index,
                },
                unbonded_amount,
                delegation_amount,
            },
        )
}

fn undelegate_claim_plan_strategy() -> impl Strategy<Value = UndelegateClaimPlan> {
    let penalty_bps = 0..100u64;
    let unbonding_start_height_strategy = 1000..100000u64;
    (
        identity_key_strategy(),
        penalty_bps,
        amount_strategy(),
        unbonding_start_height_strategy,
    )
        .prop_map(
            |(validator_identity, penalty_bps, unbonding_amount, unbonding_start_height)| {
                UndelegateClaimPlan {
                    validator_identity,
                    penalty: Penalty::from_bps(penalty_bps),
                    unbonding_amount,
                    balance_blinding: Fr::rand(&mut OsRng),
                    proof_blinding_r: Fq::rand(&mut OsRng),
                    proof_blinding_s: Fq::rand(&mut OsRng),
                    unbonding_start_height,
                }
            },
        )
}

fn signing_key_strategy() -> impl Strategy<Value = SigningKey<SpendAuth>> {
    prop::strategy::LazyJust::new(|| SigningKey::<SpendAuth>::new(OsRng))
}

fn consensus_secret_key_strategy() -> impl Strategy<Value = Ed25519SigningKey> {
    prop::strategy::LazyJust::new(|| Ed25519SigningKey::new(OsRng))
}

fn validator_strategy() -> impl Strategy<Value = (validator::Validator, SigningKey<SpendAuth>)> {
    (signing_key_strategy(), consensus_secret_key_strategy()).prop_map(
        move |(new_validator_id_sk, new_validator_consensus_sk)| {
            let new_validator_id = IdentityKey(VerificationKey::from(&new_validator_id_sk).into());
            let new_validator_consensus = new_validator_consensus_sk.verification_key();
            (
                validator::Validator {
                    identity_key: new_validator_id.clone(),
                    consensus_key: tendermint::PublicKey::from_raw_ed25519(
                        &new_validator_consensus.to_bytes(),
                    )
                    .expect("consensus key is valid"),
                    governance_key: GovernanceKey(new_validator_id_sk.into()),
                    enabled: true,
                    sequence_number: 0,
                    name: "test validator".to_string(),
                    website: String::default(),
                    description: String::default(),
                    funding_streams: FundingStreams::default(),
                },
                new_validator_id_sk,
            )
        },
    )
}

fn validator_definition_strategy() -> impl Strategy<Value = Definition> {
    (validator_strategy()).prop_map(|(new_validator, new_validator_id_sk)| {
        let bytes = new_validator.encode_to_vec();
        let auth_sig = new_validator_id_sk.sign(OsRng, &bytes);
        Definition {
            validator: new_validator,
            auth_sig,
        }
    })
}

#[derive(Debug, Clone)]
enum SwapAmountType {
    ZeroFirst,
    ZeroSecond,
    BothNonZero,
}

fn swap_amount_type_strategy() -> impl Strategy<Value = SwapAmountType> {
    // We want test vectors for cases where the first asset is zero, the second asset is zero,
    // and both assets are non-zero. In the latter case, hardware custody backends should
    // refuse to sign the transaction.
    prop_oneof![
        Just(SwapAmountType::ZeroFirst),
        Just(SwapAmountType::ZeroSecond),
        Just(SwapAmountType::BothNonZero),
    ]
}

fn swap_plaintext_strategy(seed_phrase: SeedPhrase) -> impl Strategy<Value = SwapPlaintext> {
    (
        amount_strategy(),
        amount_strategy(),
        asset_id_strategy(),
        asset_id_strategy(),
        address_strategy(seed_phrase),
        swap_amount_type_strategy(),
    )
        .prop_map(
            |(delta_1_i, delta_2_i, asset_1, asset_2, claim_address, amount_type)| {
                let (delta_1_i, delta_2_i) = match amount_type {
                    SwapAmountType::ZeroFirst => (0u64.into(), delta_2_i),
                    SwapAmountType::ZeroSecond => (delta_1_i, 0u64.into()),
                    SwapAmountType::BothNonZero => (delta_1_i, delta_2_i),
                };
                let trading_pair = TradingPair::new(asset_1, asset_2);
                SwapPlaintext::new(
                    &mut OsRng,
                    trading_pair,
                    delta_1_i,
                    delta_2_i,
                    Fee::from_staking_token_amount(0u64.into()),
                    claim_address,
                )
            },
        )
}

fn swap_plan_strategy(seed_phrase: SeedPhrase) -> impl Strategy<Value = SwapPlan> {
    (swap_plaintext_strategy(seed_phrase)).prop_map(|swap_plaintext| SwapPlan {
        proof_blinding_r: Fq::rand(&mut OsRng),
        proof_blinding_s: Fq::rand(&mut OsRng),
        swap_plaintext,
        fee_blinding: Fr::rand(&mut OsRng),
    })
}

fn batch_swap_output_data_strategy() -> impl Strategy<Value = BatchSwapOutputData> {
    // Represents a filled swap
    let delta_1 = (4001..2000000000u128).prop_map(Amount::from);
    let delta_2 = (4001..2000000000u128).prop_map(Amount::from);

    let lambda_1 = (2..2000u64).prop_map(Amount::from);
    let lambda_2 = (2..2000u64).prop_map(Amount::from);

    let unfilled_1 = (2..2000u64).prop_map(Amount::from);
    let unfilled_2 = (2..2000u64).prop_map(Amount::from);

    (
        delta_1,
        delta_2,
        lambda_1,
        lambda_2,
        unfilled_1,
        unfilled_2,
        asset_id_strategy(),
        asset_id_strategy(),
    )
        .prop_map(
            |(
                delta_1,
                delta_2,
                lambda_1,
                lambda_2,
                unfilled_1,
                unfilled_2,
                asset_id_1,
                asset_id_2,
            )| BatchSwapOutputData {
                delta_1,
                delta_2,
                lambda_1,
                lambda_2,
                unfilled_1,
                unfilled_2,
                height: 0u64.into(),
                trading_pair: TradingPair::new(asset_id_1, asset_id_2),
                sct_position_prefix: Default::default(),
            },
        )
}

fn swap_claim_plan_strategy(seed_phrase: SeedPhrase) -> impl Strategy<Value = SwapClaimPlan> {
    (
        swap_plaintext_strategy(seed_phrase),
        batch_swap_output_data_strategy(),
    )
        .prop_map(|(swap_plaintext, output_data)| SwapClaimPlan {
            swap_plaintext,
            position: penumbra_tct::Position::from(0u64),
            output_data,
            epoch_duration: 1000u64,
            proof_blinding_r: Fq::rand(&mut OsRng),
            proof_blinding_s: Fq::rand(&mut OsRng),
        })
}

fn sequence_strategy() -> impl Strategy<Value = Sequence> {
    (4001..2000000000u64).prop_map(Sequence)
}

fn ibc_action_strategy(seed_phrase: SeedPhrase) -> impl Strategy<Value = IbcRelay> {
    (
        sequence_strategy(),
        0..1000000000u64,
        0..1000000000u64,
        address_strategy(seed_phrase.clone()),
    )
        .prop_map(|(sequence, revision_number, revision_height, src)| {
            IbcRelay::RecvPacket(MsgRecvPacket {
                packet: Packet {
                    sequence,
                    port_on_a: PortId::default(),
                    chan_on_a: ChannelId::default(),
                    port_on_b: PortId::default(),
                    chan_on_b: ChannelId::default(),
                    data: vec![0u8; 100],
                    timeout_height_on_b: ibc_types::core::channel::TimeoutHeight::At(
                        Height::new(revision_number, revision_height).expect("test value"),
                    ),
                    timeout_timestamp_on_b: Timestamp::now(),
                },
                // this can't be empty
                proof_commitment_on_a: MerkleProof {
                    proofs: vec![CommitmentProof::default()],
                },
                proof_height_on_a: Height::new(revision_number, revision_height)
                    .expect("test value"),
                signer: src.to_string(),
            })
        })
}

fn proposal_strategy() -> impl Strategy<Value = Proposal> {
    (
        prop::string::string_regex(r"[a-z]+-[0-9]+").unwrap(),
        prop::string::string_regex(r"[a-z]+-[0-9]+").unwrap(),
    )
        .prop_map(|(title, description)| Proposal {
            id: 0u64,
            title,
            description,
            payload: ProposalPayload::Signaling { commit: None },
        })
}

fn proposal_id_strategy() -> impl Strategy<Value = u64> {
    0u64..1000000000u64
}

fn proposal_submit_strategy() -> impl Strategy<Value = ProposalSubmit> {
    (proposal_strategy(), amount_strategy()).prop_map(|(proposal, deposit_amount)| ProposalSubmit {
        proposal,
        deposit_amount,
    })
}

fn proposal_withdraw_strategy() -> impl Strategy<Value = ProposalWithdraw> {
    (proposal_id_strategy()).prop_map(|proposal| ProposalWithdraw {
        proposal,
        reason: String::default(),
    })
}

fn vote_strategy() -> impl Strategy<Value = Vote> {
    prop_oneof![Just(Vote::Yes), Just(Vote::No), Just(Vote::Abstain),]
}

fn note_strategy_without_address() -> impl Strategy<Value = Note> {
    (
        uncontrolled_address_strategy(),
        value_strategy(),
        prop::array::uniform32(any::<u8>()),
    )
        .prop_map(|(address, value, rseed_bytes)| {
            Note::from_parts(address, value, Rseed(rseed_bytes))
                .expect("should be a valid test note")
        })
}

fn delegator_vote_strategy() -> impl Strategy<Value = DelegatorVotePlan> {
    (
        proposal_id_strategy(),
        vote_strategy(),
        amount_strategy(),
        note_strategy_without_address(),
    )
        .prop_map(
            |(proposal, vote, unbonded_amount, staked_note)| DelegatorVotePlan {
                proposal,
                vote,
                start_position: penumbra_tct::Position::from(0u64),
                staked_note,
                unbonded_amount,
                position: penumbra_tct::Position::from(0u64),
                randomizer: Fr::rand(&mut OsRng),
                proof_blinding_r: Fq::rand(&mut OsRng),
                proof_blinding_s: Fq::rand(&mut OsRng),
            },
        )
}

fn validator_vote_strategy() -> impl Strategy<Value = ValidatorVote> {
    (
        proposal_id_strategy(),
        vote_strategy(),
        identity_key_strategy(),
        signing_key_strategy(),
        prop::string::string_regex(r"[a-zA-Z0-9]+").unwrap(),
    )
        .prop_map(|(proposal, vote, identity_key, signing_key, reason)| {
            let governance_key = GovernanceKey(signing_key.into());
            let body = ValidatorVoteBody {
                proposal,
                vote,
                identity_key,
                governance_key,
                reason: ValidatorVoteReason(reason),
            };

            let bytes = body.encode_to_vec();
            let auth_sig = signing_key.sign(OsRng, &bytes);
            ValidatorVote { body, auth_sig }
        })
}

fn proposal_outcome_strategy() -> impl Strategy<Value = ProposalOutcome<()>> {
    prop_oneof![
        Just(ProposalOutcome::Passed),
        Just(ProposalOutcome::Failed {
            withdrawn: Withdrawn::No
        }),
        Just(ProposalOutcome::Slashed {
            withdrawn: Withdrawn::No
        }),
    ]
}

fn proposal_deposit_claim_strategy() -> impl Strategy<Value = ProposalDepositClaim> {
    (
        proposal_id_strategy(),
        amount_strategy(),
        proposal_outcome_strategy(),
    )
        .prop_map(|(proposal, deposit_amount, outcome)| ProposalDepositClaim {
            proposal,
            deposit_amount,
            outcome,
        })
}

fn position_state_strategy() -> impl Strategy<Value = PositionState> {
    prop_oneof![Just(PositionState::Opened), Just(PositionState::Closed)]
}

fn trading_function_strategy() -> impl Strategy<Value = TradingFunction> {
    (
        amount_strategy(),
        amount_strategy(),
        asset_id_strategy(),
        asset_id_strategy(),
    )
        .prop_map(|(p, q, asset_1, asset_2)| {
            let trading_pair = TradingPair::new(asset_1, asset_2);
            TradingFunction::new(trading_pair, 0u32, p, q)
        })
}

fn position_strategy() -> impl Strategy<Value = Position> {
    (
        position_state_strategy(),
        amount_strategy(),
        amount_strategy(),
        trading_function_strategy(),
    )
        .prop_map(|(state, r1, r2, phi)| Position {
            state,
            reserves: Reserves { r1, r2 },
            phi,
            nonce: [0u8; 32],
            close_on_fill: true,
        })
}

fn position_open_strategy() -> impl Strategy<Value = PositionOpen> {
    (position_strategy()).prop_map(|position| PositionOpen { position })
}

fn position_close_strategy() -> impl Strategy<Value = PositionClose> {
    (position_strategy()).prop_map(|position| PositionClose {
        position_id: position.id(),
    })
}

fn position_withdraw_strategy() -> impl Strategy<Value = PositionWithdrawPlan> {
    (position_strategy()).prop_map(|position| PositionWithdrawPlan {
        position_id: position.id(),
        reserves: position.reserves,
        rewards: vec![],
        pair: position.phi.pair,
        sequence: 1u64,
    })
}

fn community_pool_deposit_strategy() -> impl Strategy<Value = CommunityPoolDeposit> {
    (value_strategy()).prop_map(|value| CommunityPoolDeposit { value })
}

fn community_pool_spend_strategy() -> impl Strategy<Value = CommunityPoolSpend> {
    (value_strategy()).prop_map(|value| CommunityPoolSpend { value })
}

fn community_pool_output_strategy() -> impl Strategy<Value = CommunityPoolOutput> {
    (value_strategy(), uncontrolled_address_strategy())
        .prop_map(|(value, address)| CommunityPoolOutput { value, address })
}

fn denom_strategy() -> impl Strategy<Value = String> {
    prop::string::string_regex(r"[a-zA-Z0-9]+").unwrap()
}

fn ics20_withdrawal_strategy(seed_phrase: SeedPhrase) -> impl Strategy<Value = Ics20Withdrawal> {
    (
        amount_strategy(),
        address_strategy(seed_phrase.clone()),
        address_strategy(seed_phrase.clone()),
        denom_strategy(),
        0..1000000000u64,
        0..1000000000u64,
    )
        .prop_map(
            |(
                amount,
                destination_chain_address,
                return_address,
                denom,
                revision_number,
                revision_height,
            )| Ics20Withdrawal {
                amount,
                denom: Metadata::try_from(&denom[..]).expect("valid test denom"),
                destination_chain_address: destination_chain_address.to_string(),
                return_address,
                timeout_height: Height::new(revision_number, revision_height).expect("test value"),
                timeout_time: 0u64,
                source_channel: ChannelId::default(),
                use_compat_address: false,
            },
        )
}

fn auction_dutch_schedule_strategy() -> impl Strategy<Value = ActionDutchAuctionSchedule> {
    (
        value_strategy(),
        asset_id_strategy(),
        amount_strategy(),
        amount_strategy(),
        0..1000000000u64,
        0..1000000000u64,
        prop::array::uniform32(any::<u8>()),
    )
        .prop_map(
            |(input, output_id, max_output, min_output, start_height, step_count, nonce)| {
                ActionDutchAuctionSchedule {
                    description: DutchAuctionDescription {
                        input,
                        output_id,
                        max_output,
                        min_output,
                        start_height,
                        end_height: start_height + 1,
                        step_count,
                        nonce,
                    },
                }
            },
        )
}

fn auction_dutch_withdraw_plan_strategy() -> impl Strategy<Value = ActionDutchAuctionWithdrawPlan> {
    (
        prop::array::uniform32(any::<u8>()),
        0..1000000000u64,
        value_strategy(),
        value_strategy(),
    )
        .prop_map(|(auction_id_bytes, seq, reserves_input, reserves_output)| {
            ActionDutchAuctionWithdrawPlan {
                auction_id: AuctionId(auction_id_bytes),
                seq,
                reserves_input,
                reserves_output,
            }
        })
}

fn auction_dutch_end_strategy() -> impl Strategy<Value = ActionDutchAuctionEnd> {
    (prop::array::uniform32(any::<u8>())).prop_map(|auction_id_bytes| ActionDutchAuctionEnd {
        auction_id: AuctionId(auction_id_bytes),
    })
}

fn action_plan_strategy(
    fvk: &FullViewingKey,
    seed_phrase: SeedPhrase,
) -> impl Strategy<Value = ActionPlan> {
    prop_oneof![
        spend_plan_strategy(fvk).prop_map(ActionPlan::Spend),
        output_plan_strategy(seed_phrase.clone()).prop_map(ActionPlan::Output),
        delegate_plan_strategy().prop_map(ActionPlan::Delegate),
        undelegate_plan_strategy().prop_map(ActionPlan::Undelegate),
        undelegate_claim_plan_strategy().prop_map(ActionPlan::UndelegateClaim),
        validator_definition_strategy().prop_map(ActionPlan::ValidatorDefinition),
        swap_plan_strategy(seed_phrase.clone()).prop_map(ActionPlan::Swap),
        swap_claim_plan_strategy(seed_phrase.clone()).prop_map(ActionPlan::SwapClaim),
        proposal_submit_strategy().prop_map(ActionPlan::ProposalSubmit),
        proposal_withdraw_strategy().prop_map(ActionPlan::ProposalWithdraw),
        ibc_action_strategy(seed_phrase.clone()).prop_map(ActionPlan::IbcAction),
        delegator_vote_strategy().prop_map(ActionPlan::DelegatorVote),
        validator_vote_strategy().prop_map(ActionPlan::ValidatorVote),
        proposal_deposit_claim_strategy().prop_map(ActionPlan::ProposalDepositClaim),
        position_open_strategy().prop_map(ActionPlan::PositionOpen),
        position_close_strategy().prop_map(ActionPlan::PositionClose),
        position_withdraw_strategy().prop_map(ActionPlan::PositionWithdraw),
        community_pool_deposit_strategy().prop_map(ActionPlan::CommunityPoolDeposit),
        community_pool_spend_strategy().prop_map(ActionPlan::CommunityPoolSpend),
        community_pool_output_strategy().prop_map(ActionPlan::CommunityPoolOutput),
        ics20_withdrawal_strategy(seed_phrase.clone()).prop_map(ActionPlan::Ics20Withdrawal),
        auction_dutch_end_strategy().prop_map(ActionPlan::ActionDutchAuctionEnd),
        auction_dutch_withdraw_plan_strategy().prop_map(ActionPlan::ActionDutchAuctionWithdraw),
        auction_dutch_schedule_strategy().prop_map(ActionPlan::ActionDutchAuctionSchedule),
    ]
}

fn actions_vec_strategy(
    fvk: &FullViewingKey,
    seed_phrase: SeedPhrase,
) -> impl Strategy<Value = Vec<ActionPlan>> {
    prop::collection::vec(action_plan_strategy(fvk, seed_phrase), 2..5)
}

fn chain_id_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("penumbra-1".to_string()),
        "[a-z]+-[0-9]+".prop_map(|s| s.to_string()), // Random other chain IDs
    ]
}

fn expiry_height_strategy() -> impl Strategy<Value = u64> {
    prop_oneof![
        Just(0u64),       // Absent expiry height
        1u64..1000000u64, // Random non-zero expiry height
    ]
}

fn transaction_parameters_strategy() -> impl Strategy<Value = TransactionParameters> {
    let fee = value_strategy().prop_map(|fee_value| Fee(fee_value));

    (expiry_height_strategy(), chain_id_strategy(), fee).prop_map(
        |(expiry_height, chain_id, fee)| TransactionParameters {
            expiry_height,
            chain_id,
            fee,
        },
    )
}

fn memo_plaintext_strategy(seed_phrase: SeedPhrase) -> impl Strategy<Value = MemoPlaintext> {
    (address_strategy(seed_phrase), "[a-zA-Z0-9 ]{1,432}").prop_map(|(return_address, text)| {
        MemoPlaintext::new(return_address, text).expect("memo text should be valid")
    })
}

fn memo_plan_strategy(seed_phrase: SeedPhrase) -> impl Strategy<Value = MemoPlan> {
    memo_plaintext_strategy(seed_phrase).prop_map(|plaintext| MemoPlan::new(&mut OsRng, plaintext))
}

fn transaction_plan_strategy(
    fvk: &FullViewingKey,
    seed_phrase: SeedPhrase,
) -> impl Strategy<Value = TransactionPlan> {
    (
        actions_vec_strategy(fvk, seed_phrase.clone()),
        transaction_parameters_strategy(),
        prop_oneof![Just(None), memo_plan_strategy(seed_phrase).prop_map(Some),],
    )
        .prop_map(|(actions, params, memo)| TransactionPlan {
            actions,
            transaction_parameters: params,
            detection_data: None,
            memo,
        })
}

#[test]
#[ignore]
fn generate_transaction_signing_test_vectors() {
    // Run this to regenerate the `EffectHash` test vectors. Ignored by default.
    let mut runner = TestRunner::new(Config::default());
    let test_vectors_dir = "tests/signing_test_vectors";
    std::fs::create_dir_all(test_vectors_dir).expect("failed to create test vectors dir");

    for i in 0..100 {
        let seed_phrase = SeedPhrase::from_str(SEED_PHRASE).expect("test seed phrase is valid");
        let sk = SpendKey::from_seed_phrase_bip44(seed_phrase.clone(), &Bip44Path::new(0));
        let fvk = sk.full_viewing_key();
        let value_tree = transaction_plan_strategy(fvk, seed_phrase)
            .new_tree(&mut runner)
            .expect("Failed to create new tree");
        let transaction_plan = value_tree.current();

        let json_plan = serde_json::to_string_pretty(&transaction_plan)
            .expect("should be able to json tx plan");

        let transaction_plan_encoded = transaction_plan.encode_to_vec();
        let effect_hash_hex = hex::encode(
            transaction_plan
                .effect_hash(fvk)
                .expect("should be able to compute effect hash")
                .0,
        );

        let json_file_path = format!("{}/transaction_plan_{}.json", test_vectors_dir, i);
        let proto_file_path = format!("{}/transaction_plan_{}.proto", test_vectors_dir, i);
        let hash_file_path = format!("{}/effect_hash_{}.txt", test_vectors_dir, i);

        let mut json_file = File::create(&json_file_path).expect("Failed to create JSON file");
        json_file
            .write_all(json_plan.as_bytes())
            .expect("Failed to write JSON file");
        let mut proto_file =
            File::create(&proto_file_path).expect("Failed to create Protobuf file");
        proto_file
            .write_all(&transaction_plan_encoded)
            .expect("Failed to write Protobuf file");

        // Write effect hash
        let mut hash_file = File::create(&hash_file_path).expect("Failed to create hash file");
        hash_file
            .write_all(effect_hash_hex.as_bytes())
            .expect("Failed to write hash file");
    }
}

/// After the colon, there should be maximum 38 characters.
const MAX_VALUE_LENGTH: usize = 38;

/// Format a string to fit within display constraints by truncating if needed
fn format_for_display(label: &str, value: String) -> Vec<String> {
    let mut result = Vec::new();
    let mut total_chunks = 0;

    // First count total chunks needed
    for line in value.split('\n') {
        total_chunks += if line.len() <= MAX_VALUE_LENGTH {
            1
        } else {
            (line.len() + MAX_VALUE_LENGTH - 1) / MAX_VALUE_LENGTH
        };
    }

    // Now generate output with chunk numbers displayed if needed (i.e.
    // not to display [1/1] if there's only one chunk)
    let mut current_chunk = 1;
    for line in value.split('\n') {
        if line.len() <= MAX_VALUE_LENGTH {
            if total_chunks == 1 {
                result.push(format!("{} : {}", label, line));
            } else {
                result.push(format!(
                    "{} [{}/{}] : {}",
                    label, current_chunk, total_chunks, line
                ));
            }
            current_chunk += 1;
        } else {
            for chunk in line.as_bytes().chunks(MAX_VALUE_LENGTH) {
                let chunk_str = String::from_utf8_lossy(chunk);
                result.push(format!(
                    "{} [{}/{}] : {}",
                    label, current_chunk, total_chunks, chunk_str
                ));
                current_chunk += 1;
            }
        }
    }
    result
}

#[test]
fn effect_hash_test_vectors() {
    // This parses the transaction plan, computes the effect hash, and verifies that it
    // matches the expected effect hash.
    let test_vectors_dir = "tests/signing_test_vectors";
    let seed_phrase = SeedPhrase::from_str(SEED_PHRASE).expect("test seed phrase is valid");
    let sk = SpendKey::from_seed_phrase_bip44(seed_phrase, &Bip44Path::new(0));
    let fvk = sk.full_viewing_key();

    for i in 0..100 {
        let proto_file_path = format!("{}/transaction_plan_{}.proto", test_vectors_dir, i);
        let mut proto_file = File::open(&proto_file_path).expect("Failed to open Protobuf file");
        let mut transaction_plan_encoded = Vec::<u8>::new();
        proto_file
            .read_to_end(&mut transaction_plan_encoded)
            .expect("Failed to read Protobuf file");
        let transaction_plan = TransactionPlan::decode(&transaction_plan_encoded[..])
            .expect("should be able to decode transaction plan");
        let effect_hash_hex = hex::encode(
            transaction_plan
                .effect_hash(fvk)
                .expect("should be able to compute effect hash")
                .0,
        );

        let hash_file_path = format!("{}/effect_hash_{}.txt", test_vectors_dir, i);
        let expected_effect_hash = std::fs::read_to_string(&hash_file_path)
            .expect("should be able to read expected effect hash");
        assert_eq!(effect_hash_hex, expected_effect_hash);
    }
}

#[ignore]
#[test]
fn generate_hw_display_test_vectors() {
    let test_vectors_dir = "tests/signing_test_vectors";
    let mut test_vectors = Vec::new();

    let seed_phrase = SeedPhrase::from_str(SEED_PHRASE).expect("test seed phrase is valid");
    let sk = SpendKey::from_seed_phrase_bip44(seed_phrase, &Bip44Path::new(0));
    let fvk = sk.full_viewing_key();

    for i in 0..100 {
        let proto_file_path = format!("{}/transaction_plan_{}.proto", test_vectors_dir, i);
        let transaction_plan_encoded =
            fs::read(&proto_file_path).expect("Failed to read Protobuf file");

        let transaction_plan = TransactionPlan::decode(&transaction_plan_encoded[..])
            .expect("should be able to decode transaction plan");

        let display_vector = json!({
            "index": i,
            "blob": hex::encode(&transaction_plan_encoded),
            "output": generate_normal_output(&transaction_plan, &fvk),
            "output_expert": generate_expert_output(&transaction_plan, &fvk),
        });

        test_vectors.push(display_vector);
    }

    // Write the test vectors to a JSON file
    let output_path = format!("{}/hw_display_vectors.json", test_vectors_dir);
    fs::write(
        output_path,
        serde_json::to_string_pretty(&test_vectors).unwrap(),
    )
    .expect("Failed to write display test vectors");
}

fn address_display(address: &Address, fvk: &FullViewingKey) -> String {
    // Use the existing AddressView to render the address data.
    let address_view = fvk.view_address(address.clone());

    match address_view {
        // The address is not controlled by the user’s account.
        // In this case it should be rendered using the Canonical Short Form.
        AddressView::Opaque { address } => address.display_short_form(),
        // The address is controlled by the user’s account.
        // In this case it should be rendered as “Main Account” or “Sub-account #N”,
        // depending on the account number.
        AddressView::Decoded {
            address: _,
            index,
            wallet_id: _,
        } => {
            if index.account == 0 {
                "Main Account".to_string()
            } else {
                format!("Sub-account #{}", index.account)
            }
        }
    }
}

fn value_display(
    value: &Value,
    chain_id: &str,
    base_denoms: &HashMap<asset::Id, String>,
) -> String {
    let amount = value.amount.value();
    let asset_id = value.asset_id;
    let cache = asset::Cache::with_known_assets();
    let value_view = value.view_with_cache(&cache);

    match value_view {
        ValueView::KnownAssetId {
            amount, metadata, ..
        } => {
            if chain_id == "penumbra-1" {
                // 1: Check if we're on penumbra-1 and if the asset is in our known registry
                let unit = metadata.default_unit();
                return format!("{} {}", unit.format_value(amount), unit);
            } else if base_denoms.get(&asset_id).is_some() {
                // 2: The asset is in the provided base denominations
                let unit = metadata.default_unit();
                return format!("{} {}", unit.format_value(amount), unit);
            } else {
                // 3: Fallback to bech32 asset ID
                return format!("{} {}", amount, asset_id.to_string());
            };
        }
        ValueView::UnknownAssetId { .. } => {
            // 3: Fallback to bech32 asset ID
            return format!("{} {}", amount, asset_id.to_string());
        }
    }
}

fn generate_normal_output(plan: &TransactionPlan, fvk: &FullViewingKey) -> Vec<String> {
    let mut output = Vec::new();
    let mut index = 0;
    // TODO: populate this
    let base_denoms = HashMap::new();
    let ivk = fvk.incoming();

    // Add chain ID
    if !plan.transaction_parameters.chain_id.is_empty() {
        for line in format_for_display("Chain ID", plan.transaction_parameters.chain_id.clone()) {
            output.push(format!("{} | {}", index, line));
        }
        index += 1;
    }

    // Add expiry height if nonzero
    if plan.transaction_parameters.expiry_height != 0 {
        for line in format_for_display(
            "Expiry Height",
            plan.transaction_parameters.expiry_height.to_string(),
        ) {
            output.push(format!("{} | {}", index, line));
        }
        index += 1;
    }

    // Add fee
    for line in format_for_display(
        "Fee",
        value_display(
            &plan.transaction_parameters.fee.0,
            &plan.transaction_parameters.chain_id,
            &base_denoms,
        ),
    ) {
        output.push(format!("{} | {}", index, line));
    }
    index += 1;

    for action in &plan.actions {
        match action {
            ActionPlan::Spend(spend) => {
                // Format the value
                let value_display = value_display(
                    &spend.note.value(),
                    &plan.transaction_parameters.chain_id,
                    &base_denoms,
                );

                // Format the address
                let address_display = address_display(&spend.note.address(), fvk);

                // Combine into "Spend {value} from {address}"
                let spend_display = format!("Spend {} from {}", value_display, address_display);

                for line in format_for_display("Action", spend_display) {
                    output.push(format!("{} | {}", index, line));
                }
                index += 1;
            }
            ActionPlan::Output(output_action) => {
                // Format the value
                let value_display = value_display(
                    &output_action.value,
                    &plan.transaction_parameters.chain_id,
                    &base_denoms,
                );

                // Format the address
                let address_display = address_display(&output_action.dest_address, fvk);

                // Combine into "Output {value} to {address}"
                let output_display = format!("Output {} to {}", value_display, address_display);

                for line in format_for_display("Action", output_display) {
                    output.push(format!("{} | {}", index, line));
                }
                index += 1;
            }
            ActionPlan::Ics20Withdrawal(withdrawal) => {
                let channel_display = format!("{}", withdrawal.source_channel);

                // Format and display the value
                let value = Value {
                    amount: withdrawal.amount,
                    asset_id: withdrawal.denom.id(),
                };
                let value_display =
                    value_display(&value, &plan.transaction_parameters.chain_id, &base_denoms);

                // Display destination address
                let destination_display = format!("{}", withdrawal.destination_chain_address);

                // Verify return address is controlled by user, bail if not
                let mut error_display = "".to_string();
                if !ivk.views_address(&withdrawal.return_address) {
                    error_display = format!("PANIC [X/X] : LEDGER SHOULD REFUSE TO SIGN (return address in Ics20Withdrawal not controlled by user)");
                }

                let ics20_display;
                if error_display != "" {
                    ics20_display = format!(
                        "ICS20Withdrawal\nChannel {}\nAmount {}\nTo {}\n{}",
                        channel_display, value_display, destination_display, error_display
                    );
                } else {
                    ics20_display = format!(
                        "ICS20Withdrawal\nChannel {}\nAmount {}\nTo {}",
                        channel_display, value_display, destination_display
                    );
                }
                for line in format_for_display("Action", ics20_display) {
                    output.push(format!("{} | {}", index, line));
                }

                // TODO: After UIP-5, add ICS-20 memo display here
                index += 1;
            }
            ActionPlan::Swap(swap) => {
                // Verify claim address is controlled by user
                let mut error_display = "".to_string();
                if !ivk.views_address(&swap.swap_plaintext.claim_address) {
                    error_display = format!(
                        "PANIC [X/X] : LEDGER SHOULD REFUSE TO SIGN (claim address in Swap not controlled by user)",
                    );
                }

                // Determine input and output assets based on which delta is nonzero
                let (input_value, output_asset) = if swap.swap_plaintext.delta_1_i != Amount::zero()
                    && swap.swap_plaintext.delta_2_i == Amount::zero()
                {
                    // Asset 1 is input, Asset 2 is output
                    let input = Value {
                        amount: swap.swap_plaintext.delta_1_i,
                        asset_id: swap.swap_plaintext.trading_pair.asset_1(),
                    };
                    (input, swap.swap_plaintext.trading_pair.asset_2())
                } else if swap.swap_plaintext.delta_2_i != Amount::zero()
                    && swap.swap_plaintext.delta_1_i == Amount::zero()
                {
                    // Asset 2 is input, Asset 1 is output
                    let input = Value {
                        amount: swap.swap_plaintext.delta_2_i,
                        asset_id: swap.swap_plaintext.trading_pair.asset_2(),
                    };
                    (input, swap.swap_plaintext.trading_pair.asset_1())
                } else {
                    // Invalid swap: exactly one delta must be nonzero
                    if error_display == "" {
                        error_display = format!(
                            "PANIC [X/X] : LEDGER SHOULD REFUSE TO SIGN (invalid swap: one delta must be nonzero)",
                        );
                    } else {
                        // Add to existing error
                        error_display = format!(
                            "{} (invalid swap: one delta must be nonzero)",
                            error_display
                        );
                    }

                    // Arbitrary choice of asset 2 as input
                    let input = Value {
                        amount: swap.swap_plaintext.delta_2_i,
                        asset_id: swap.swap_plaintext.trading_pair.asset_2(),
                    };
                    (input, swap.swap_plaintext.trading_pair.asset_1())
                };

                // Display input value
                let input_display = value_display(
                    &input_value,
                    &plan.transaction_parameters.chain_id,
                    &base_denoms,
                );

                // Display output asset by creating a zero-value and extracting just the denomination
                let output_value = Value {
                    amount: Amount::from(0u64),
                    asset_id: output_asset,
                };
                let value_view = value_display(
                    &output_value,
                    &plan.transaction_parameters.chain_id,
                    &base_denoms,
                );
                // Skip the "0 " prefix to get just the denomination
                let output_asset_display = value_view
                    .clone()
                    .split_once(' ')
                    .map_or(value_view, |(_amount, denom)| denom.to_string());

                // Display claim fee
                let claim_fee_display = value_display(
                    &swap.swap_plaintext.claim_fee.0,
                    &plan.transaction_parameters.chain_id,
                    &base_denoms,
                );

                let swap_display;
                if error_display != "" {
                    swap_display = format!(
                        "Swap\nInput {}\nOutput Asset {}\nClaim Fee {}\n{}",
                        input_display, output_asset_display, claim_fee_display, error_display
                    );
                } else {
                    swap_display = format!(
                        "Swap\nInput {}\nOutput Asset {}\nClaim Fee {}",
                        input_display, output_asset_display, claim_fee_display
                    );
                }

                for line in format_for_display("Action", swap_display) {
                    output.push(format!("{} | {}", index, line));
                }

                index += 1;
            }
            ActionPlan::Delegate(delegate) => {
                // Format the unbonded amount (input)
                let input_value = Value {
                    amount: delegate.unbonded_amount,
                    asset_id: *penumbra_asset::STAKING_TOKEN_ASSET_ID,
                };
                let input_display = value_display(
                    &input_value,
                    &plan.transaction_parameters.chain_id,
                    &base_denoms,
                );

                // Format validator identity
                let validator_display = format!("{}", delegate.validator_identity);

                let delegate_display = format!(
                    "Delegate\nTo {}\nInput {}",
                    validator_display, input_display
                );

                for line in format_for_display("Action", delegate_display) {
                    output.push(format!("{} | {}", index, line));
                }
                index += 1;
            }
            ActionPlan::Undelegate(undelegate) => {
                // Format the delegation amount (input)
                let input_value = undelegate.delegation_value();
                let input_display = value_display(
                    &input_value,
                    &plan.transaction_parameters.chain_id,
                    &base_denoms,
                );

                // Format the unbonding amount (output)
                let output_value = undelegate.unbonded_value();
                let output_display = value_display(
                    &output_value,
                    &plan.transaction_parameters.chain_id,
                    &base_denoms,
                );

                // Format validator identity
                let validator_display = format!("{}", undelegate.validator_identity);

                let undelegate_display = format!(
                    "Undelegate\nFrom {}\nInput {}\nOutput {}",
                    validator_display, input_display, output_display
                );

                for line in format_for_display("Action", undelegate_display) {
                    output.push(format!("{} | {}", index, line));
                }
                index += 1;
            }
            ActionPlan::UndelegateClaim(claim) => {
                // Format the unbonding tokens value
                let value = Value {
                    amount: claim.unbonding_amount,
                    asset_id: claim.unbonding_id(),
                };
                let value_display =
                    value_display(&value, &plan.transaction_parameters.chain_id, &base_denoms);

                let claim_display = format!("UndelegateClaim\nValue {}", value_display,);

                for line in format_for_display("Action", claim_display) {
                    output.push(format!("{} | {}", index, line));
                }
                index += 1;
            }
            ActionPlan::DelegatorVote(vote) => {
                // Format the voting power as a value
                let power_value = Value {
                    amount: vote.unbonded_amount,
                    asset_id: *penumbra_asset::STAKING_TOKEN_ASSET_ID,
                };
                let power_display = value_display(
                    &power_value,
                    &plan.transaction_parameters.chain_id,
                    &base_denoms,
                );

                // Convert vote to Yes/No/Abstain string
                let vote_choice = match vote.vote {
                    penumbra_governance::Vote::Yes => "Yes",
                    penumbra_governance::Vote::No => "No",
                    penumbra_governance::Vote::Abstain => "Abstain",
                };

                let vote_display = format!(
                    "DelegatorVote on Proposal {}\nVote {}\nVoting Power: {}",
                    vote.proposal, vote_choice, power_display
                );

                for line in format_for_display("Action", vote_display) {
                    output.push(format!("{} | {}", index, line));
                }
                index += 1;
            }
            ActionPlan::PositionOpen(position_open) => {
                // Format the first reserve amount
                let reserves_1 = Value {
                    amount: position_open.position.reserves.r1,
                    asset_id: position_open.position.phi.pair.asset_1(),
                };
                let reserves_1_display = value_display(
                    &reserves_1,
                    &plan.transaction_parameters.chain_id,
                    &base_denoms,
                );

                // Format the second reserve amount
                let reserves_2 = Value {
                    amount: position_open.position.reserves.r2,
                    asset_id: position_open.position.phi.pair.asset_2(),
                };
                let reserves_2_display = value_display(
                    &reserves_2,
                    &plan.transaction_parameters.chain_id,
                    &base_denoms,
                );

                // Build display string conditionally including close_on_fill
                let position_display = if position_open.position.close_on_fill {
                    format!(
                "PositionOpen\nReserves 1: {}\nReserves 2: {}\nTrading Function p: {}\nTrading Function q: {}\nFee: {}\nClose on fill: true",
                        reserves_1_display,
                        reserves_2_display,
                        position_open.position.phi.component.p,
                        position_open.position.phi.component.q,
                        position_open.position.phi.component.fee,
            )
                } else {
                    format!(
                        "PositionOpen\nReserves 1: {}\nReserves 2: {}\nTrading Function p: {}\nTrading Function q: {}\nFee: {}",
                        reserves_1_display,
                        reserves_2_display,
                        position_open.position.phi.component.p,
                        position_open.position.phi.component.q,
                        position_open.position.phi.component.fee,
                    )
                };

                for line in format_for_display("Action", position_display) {
                    output.push(format!("{} | {}", index, line));
                }
                index += 1;
            }
            ActionPlan::PositionClose(position_close) => {
                let position_display =
                    format!("PositionClose\nPosition ID {}", position_close.position_id);

                for line in format_for_display("Action", position_display) {
                    output.push(format!("{} | {}", index, line));
                }
                index += 1;
            }
            ActionPlan::PositionWithdraw(position_withdraw) => {
                let position_display = format!(
                    "PositionWithdraw\nPosition ID {}\nSequence number {}",
                    position_withdraw.position_id, position_withdraw.sequence
                );

                for line in format_for_display("Action", position_display) {
                    output.push(format!("{} | {}", index, line));
                }
                index += 1;
            }
            ActionPlan::ActionDutchAuctionSchedule(auction) => {
                // Format the selling amount
                let selling = Value {
                    amount: auction.description.input.amount,
                    asset_id: auction.description.input.asset_id,
                };
                let selling_display = value_display(
                    &selling,
                    &plan.transaction_parameters.chain_id,
                    &base_denoms,
                );

                // Format the "for" asset (just the asset ID since it's the target)
                let for_asset = Value {
                    amount: 0u64.into(), // Amount not relevant for display
                    asset_id: auction.description.output_id,
                };
                let for_asset_display = value_display(
                    &for_asset,
                    &plan.transaction_parameters.chain_id,
                    &base_denoms,
                )
                .split_whitespace()
                .last()
                .unwrap_or("unknown")
                .to_string();

                // Format starting price
                let start_price = format!(
                    "{} {} for {} {}",
                    auction.description.max_output,
                    for_asset_display,
                    auction.description.input.amount,
                    selling_display
                        .split_whitespace()
                        .last()
                        .unwrap_or("unknown")
                );

                // Format ending price
                let end_price = format!(
                    "{} {} for {} {}",
                    auction.description.min_output,
                    for_asset_display,
                    auction.description.input.amount,
                    selling_display
                        .split_whitespace()
                        .last()
                        .unwrap_or("unknown")
                );

                let auction_display: String = format!(
                    "DutchAuctionSchedule\nSelling: {}\nFor: {}\nStarting price: {}\nEnding price: {}\nStart block height: {}\nEnd block height: {}\nSteps: {}",
                    selling_display,
                    for_asset_display,
                    start_price,
                    end_price,
                    auction.description.start_height,
                    auction.description.end_height,
                    auction.description.step_count,
                );

                for line in format_for_display("Action", auction_display) {
                    output.push(format!("{} | {}", index, line));
                }
                index += 1;
            }
            ActionPlan::ActionDutchAuctionEnd(auction_end) => {
                let auction_display =
                    format!("DutchAuctionEnd\nAuction ID: {}", auction_end.auction_id);

                for line in format_for_display("Action", auction_display) {
                    output.push(format!("{} | {}", index, line));
                }
                index += 1;
            }
            ActionPlan::ActionDutchAuctionWithdraw(withdraw) => {
                // Format the unsold amount
                let unsold_display = value_display(
                    &withdraw.reserves_input,
                    &plan.transaction_parameters.chain_id,
                    &base_denoms,
                );

                // Format the proceeds amount
                let proceeds_display = value_display(
                    &withdraw.reserves_output,
                    &plan.transaction_parameters.chain_id,
                    &base_denoms,
                );

                let withdraw_display = format!(
                    "DutchAuctionWithdraw\nAuction ID: {}\nUnsold: {}\nProceeds: {}\nSequence number: {}",
                    withdraw.auction_id,
                    unsold_display,
                    proceeds_display,
                    withdraw.seq
                );

                for line in format_for_display("Action", withdraw_display) {
                    output.push(format!("{} | {}", index, line));
                }
                index += 1;
            }
            _ => {
                // TODO: populate this
            }
        }
    }

    // Add memo if present
    if let Some(memo) = &plan.memo {
        // Display sender address
        for line in format_for_display(
            "Sender Address",
            address_display(&memo.plaintext.return_address(), &fvk),
        ) {
            output.push(format!("{} | {}", index, line));
        }

        // Display memo text
        for line in format_for_display("Memo Text", memo.plaintext.text().to_string()) {
            output.push(format!("{} | {}", index, line));
        }
    }
    // TODO: If adding more stuff here increment the `index`

    output
}

fn generate_expert_output(plan: &TransactionPlan, fvk: &FullViewingKey) -> Vec<String> {
    // For now, expert mode shows the same output
    // We can customize this later if needed
    generate_normal_output(plan, fvk)
}
