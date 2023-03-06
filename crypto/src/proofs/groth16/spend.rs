use std::str::FromStr;

use ark_r1cs_std::{
    prelude::{EqGadget, FieldVar},
    uint8::UInt8,
};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use decaf377::FieldExt;
use decaf377::{r1cs::FqVar, Bls12_377, Fq, Fr};

use ark_ff::ToConstraintField;
use ark_groth16::{Groth16, PreparedVerifyingKey, Proof, ProvingKey};
use ark_r1cs_std::prelude::AllocVar;
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef};
use ark_snark::SNARK;
use decaf377_rdsa::{SpendAuth, VerificationKey};
use penumbra_proto::{core::crypto::v1alpha1 as pb, DomainType};
use penumbra_tct as tct;
use rand::{CryptoRng, Rng};
use rand_core::OsRng;

use crate::proofs::groth16::{gadgets, ParameterSetup};
use crate::{
    balance,
    balance::commitment::BalanceCommitmentVar,
    keys::{
        AuthorizationKeyVar, IncomingViewingKeyVar, NullifierKey, NullifierKeyVar,
        RandomizedVerificationKey, SeedPhrase, SpendAuthRandomizerVar, SpendKey,
    },
    note,
    nullifier::NullifierVar,
    Note, Nullifier, Rseed, Value,
};

use super::GROTH16_PROOF_LENGTH_BYTES;

/// Groth16 proof for spending existing notes.
#[derive(Clone, Debug)]
pub struct SpendCircuit {
    // Witnesses
    /// Inclusion proof for the note commitment.
    state_commitment_proof: tct::Proof,
    /// The note being spent.
    note: Note,
    /// The blinding factor used for generating the value commitment.
    v_blinding: Fr,
    /// The randomizer used for generating the randomized spend auth key.
    spend_auth_randomizer: Fr,
    /// The spend authorization key.
    ak: VerificationKey<SpendAuth>,
    /// The nullifier deriving key.
    nk: NullifierKey,

    // Public inputs
    /// the merkle root of the state commitment tree.
    pub anchor: tct::Root,
    /// value commitment of the note to be spent.
    pub balance_commitment: balance::Commitment,
    /// nullifier of the note to be spent.
    pub nullifier: Nullifier,
    /// the randomized verification spend key.
    pub rk: VerificationKey<SpendAuth>,
}

impl ConstraintSynthesizer<Fq> for SpendCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fq>) -> ark_relations::r1cs::Result<()> {
        // Witnesses
        let note_var = note::NoteVar::new_witness(cs.clone(), || Ok(self.note.clone()))?;
        let claimed_note_commitment = note::NoteCommitmentVar::new_witness(cs.clone(), || {
            Ok(self.state_commitment_proof.commitment())
        })?;

        let position_var = tct::r1cs::PositionVar::new_witness(cs.clone(), || {
            Ok(self.state_commitment_proof.position())
        })?;
        let merkle_path_var =
            tct::r1cs::MerkleAuthPathVar::new(cs.clone(), self.state_commitment_proof)?;

        let v_blinding_arr: [u8; 32] = self.v_blinding.to_bytes();
        let v_blinding_vars = UInt8::new_witness_vec(cs.clone(), &v_blinding_arr)?;

        let spend_auth_randomizer_var =
            SpendAuthRandomizerVar::new_witness(cs.clone(), || Ok(self.spend_auth_randomizer))?;
        let ak_element_var: AuthorizationKeyVar =
            AuthorizationKeyVar::new_witness(cs.clone(), || Ok(self.ak))?;
        let nk_var = NullifierKeyVar::new_witness(cs.clone(), || Ok(self.nk))?;

        // Public inputs
        let anchor_var = FqVar::new_input(cs.clone(), || Ok(Fq::from(self.anchor)))?;
        let claimed_balance_commitment_var =
            BalanceCommitmentVar::new_input(cs.clone(), || Ok(self.balance_commitment))?;
        let claimed_nullifier_var = NullifierVar::new_input(cs.clone(), || Ok(self.nullifier))?;
        let rk_var = RandomizedVerificationKey::new_input(cs.clone(), || Ok(self.rk.clone()))?;

        // We short circuit to true if value released is 0. That means this is a _dummy_ spend.
        let is_dummy = note_var.amount().is_eq(&FqVar::zero())?;
        // We use a Boolean constraint to enforce the below constraints only if this is not a
        // dummy spend.
        let is_not_dummy = is_dummy.not();

        // Note commitment integrity.
        let note_commitment_var = note_var.commit()?;
        note_commitment_var.conditional_enforce_equal(&claimed_note_commitment, &is_not_dummy)?;

        // Nullifier integrity.
        let nullifier_var = nk_var.derive_nullifier(&position_var, &claimed_note_commitment)?;
        nullifier_var.conditional_enforce_equal(&claimed_nullifier_var, &is_not_dummy)?;

        // Merkle auth path verification against the provided anchor.
        merkle_path_var.verify(
            cs.clone(),
            &is_not_dummy,
            position_var.inner,
            anchor_var,
            claimed_note_commitment.inner(),
        )?;

        // Check integrity of randomized verification key.
        let computed_rk_var = ak_element_var.randomize(&spend_auth_randomizer_var)?;
        computed_rk_var.conditional_enforce_equal(&rk_var, &is_not_dummy)?;

        // Check integrity of diversified address.
        let ivk = IncomingViewingKeyVar::derive(&nk_var, &ak_element_var)?;
        let computed_transmission_key =
            ivk.diversified_public(&note_var.diversified_generator())?;
        computed_transmission_key
            .conditional_enforce_equal(&note_var.transmission_key(), &is_not_dummy)?;

        // Check integrity of balance commitment.
        let balance_commitment = note_var.value().commit(v_blinding_vars)?;
        balance_commitment
            .conditional_enforce_equal(&claimed_balance_commitment_var, &is_not_dummy)?;

        // Check elements were not identity.
        gadgets::element_not_identity(cs.clone(), &is_not_dummy, note_var.diversified_generator())?;
        gadgets::element_not_identity(cs, &is_not_dummy, ak_element_var.inner)?;
        Ok(())
    }
}

impl ParameterSetup for SpendCircuit {
    fn generate_test_parameters() -> (ProvingKey<Bls12_377>, PreparedVerifyingKey<Bls12_377>) {
        let seed_phrase = SeedPhrase::from_randomness([b'f'; 32]);
        let sk_sender = SpendKey::from_seed_phrase(seed_phrase, 0);
        let fvk_sender = sk_sender.full_viewing_key();
        let ivk_sender = fvk_sender.incoming();
        let (address, _dtk_d) = ivk_sender.payment_address(0u32.into());

        let spend_auth_randomizer = Fr::from(1);
        let rsk = sk_sender.spend_auth_key().randomize(&spend_auth_randomizer);
        let nk = *sk_sender.nullifier_key();
        let ak = sk_sender.spend_auth_key().into();
        let note = Note::from_parts(
            address,
            Value::from_str("1upenumbra").expect("valid value"),
            Rseed([1u8; 32]),
        )
        .expect("can make a note");
        let v_blinding = Fr::from(1);
        let rk: VerificationKey<SpendAuth> = rsk.into();
        let nullifier = Nullifier(Fq::from(1));
        let mut sct = tct::Tree::new();
        let note_commitment = note.commit();
        sct.insert(tct::Witness::Keep, note_commitment).unwrap();
        let anchor = sct.root();
        let state_commitment_proof = sct.witness(note_commitment).unwrap();

        let circuit = SpendCircuit {
            state_commitment_proof,
            note,
            v_blinding,
            spend_auth_randomizer,
            ak,
            nk,
            anchor,
            balance_commitment: balance::Commitment(decaf377::basepoint()),
            nullifier,
            rk,
        };
        let (pk, vk) = Groth16::circuit_specific_setup(circuit, &mut OsRng)
            .expect("can perform circuit specific setup");
        (pk, vk.into())
    }
}

#[derive(Clone, Debug)]
pub struct SpendProof(Proof<Bls12_377>);

impl SpendProof {
    #![allow(clippy::too_many_arguments)]
    pub fn prove<R: CryptoRng + Rng>(
        rng: &mut R,
        pk: &ProvingKey<Bls12_377>,
        state_commitment_proof: tct::Proof,
        note: Note,
        v_blinding: Fr,
        spend_auth_randomizer: Fr,
        ak: VerificationKey<SpendAuth>,
        nk: NullifierKey,
        anchor: tct::Root,
        balance_commitment: balance::Commitment,
        nullifier: Nullifier,
        rk: VerificationKey<SpendAuth>,
    ) -> anyhow::Result<Self> {
        let circuit = SpendCircuit {
            state_commitment_proof,
            note,
            v_blinding,
            spend_auth_randomizer,
            ak,
            nk,
            anchor,
            balance_commitment,
            nullifier,
            rk,
        };
        let proof = Groth16::prove(pk, circuit, rng).map_err(|err| anyhow::anyhow!(err))?;
        Ok(Self(proof))
    }

    /// Called to verify the proof using the provided public inputs.
    // Commented out, but this may be useful when debugging proof verification failures,
    // to check that the proof data and verification keys are consistent.
    //#[tracing::instrument(skip(self, vk), fields(self = ?base64::encode(&self.clone().encode_to_vec()), vk = ?vk.debug_id()))]
    #[tracing::instrument(skip(self, vk))]
    pub fn verify(
        &self,
        vk: &PreparedVerifyingKey<Bls12_377>,
        anchor: tct::Root,
        balance_commitment: balance::Commitment,
        nullifier: Nullifier,
        rk: VerificationKey<SpendAuth>,
    ) -> anyhow::Result<()> {
        let mut public_inputs = Vec::new();
        public_inputs.extend(Fq::from(anchor.0).to_field_elements().unwrap());
        public_inputs.extend(balance_commitment.0.to_field_elements().unwrap());
        public_inputs.extend(nullifier.0.to_field_elements().unwrap());
        let element_rk = decaf377::Encoding(rk.to_bytes())
            .vartime_decompress()
            .expect("expect only valid element points");
        public_inputs.extend(element_rk.to_field_elements().unwrap());

        tracing::trace!(?public_inputs);
        let start = std::time::Instant::now();
        let proof_result =
            Groth16::verify_with_processed_vk(&vk, public_inputs.as_slice(), &self.0)
                .map_err(|err| anyhow::anyhow!(err))?;
        tracing::debug!(?proof_result, elapsed = ?start.elapsed());
        proof_result
            .then_some(())
            .ok_or_else(|| anyhow::anyhow!("spend proof did not verify"))
    }
}

impl DomainType for SpendProof {
    type Proto = pb::ZkSpendProof;
}

impl From<SpendProof> for pb::ZkSpendProof {
    fn from(proof: SpendProof) -> Self {
        let mut proof_bytes = [0u8; GROTH16_PROOF_LENGTH_BYTES];
        Proof::serialize(&proof.0, &mut proof_bytes[..]).expect("can serialize Proof");
        pb::ZkSpendProof {
            inner: proof_bytes.to_vec(),
        }
    }
}

impl TryFrom<pb::ZkSpendProof> for SpendProof {
    type Error = anyhow::Error;

    fn try_from(proto: pb::ZkSpendProof) -> Result<Self, Self::Error> {
        Ok(SpendProof(
            Proof::deserialize(&proto.inner[..]).map_err(|e| anyhow::anyhow!(e))?,
        ))
    }
}
