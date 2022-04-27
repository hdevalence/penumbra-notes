use std::convert::{TryFrom, TryInto};

use anyhow::Error;
use ark_ff::Zero;
use bytes::Bytes;
use decaf377::FieldExt;
use penumbra_crypto::{
    merkle,
    rdsa::{Binding, Signature, VerificationKey, VerificationKeyBytes},
    Fr, Nullifier, Value, STAKING_TOKEN_ASSET_ID,
};
use penumbra_proto::{ibc as pb_ibc, stake as pbs, transaction as pbt, Message, Protobuf};

use crate::{
    action::{output, Delegate, Undelegate},
    Action,
};

mod builder;
pub use builder::Builder;

#[derive(Clone, Debug)]
pub struct TransactionBody {
    pub actions: Vec<Action>,
    pub merkle_root: merkle::Root,
    pub expiry_height: u32,
    pub chain_id: String,
    pub fee: Fee,
}

impl TransactionBody {
    pub fn sighash(&self) -> [u8; 64] {
        use penumbra_proto::sighash::SigHashTransaction;

        let sighash_tx = SigHashTransaction::from(pbt::TransactionBody::from(self.clone()));
        let sighash_tx_bytes: Vec<u8> = sighash_tx.encode_to_vec();

        *blake2b_simd::Params::default()
            .personal(b"Penumbra_SigHash")
            .hash(&sighash_tx_bytes)
            .as_array()
    }
}

#[derive(Clone, Debug)]
pub struct Fee(pub u64);

#[derive(Clone, Debug)]
pub struct Transaction {
    pub transaction_body: TransactionBody,
    pub binding_sig: Signature<Binding>,
}

impl Transaction {
    /// Start building a transaction relative to a given [`merkle::Root`].
    pub fn build_with_root(merkle_root: merkle::Root) -> Builder {
        Builder {
            spends: Vec::new(),
            outputs: Vec::new(),
            delegations: Vec::new(),
            undelegations: Vec::new(),
            validator_definitions: Vec::new(),
            fee: None,
            synthetic_blinding_factor: Fr::zero(),
            value_balance: decaf377::Element::default(),
            value_commitments: decaf377::Element::default(),
            merkle_root,
            expiry_height: None,
            chain_id: None,
        }
    }

    pub fn actions(&self) -> impl Iterator<Item = &Action> {
        self.transaction_body.actions.iter()
    }

    pub fn delegations(&self) -> impl Iterator<Item = &Delegate> {
        self.actions().filter_map(|action| {
            if let Action::Delegate(d) = action {
                Some(d)
            } else {
                None
            }
        })
    }

    pub fn undelegations(&self) -> impl Iterator<Item = &Undelegate> {
        self.actions().filter_map(|action| {
            if let Action::Undelegate(d) = action {
                Some(d)
            } else {
                None
            }
        })
    }

    pub fn ibc_actions(&self) -> impl Iterator<Item = &pb_ibc::IbcAction> {
        self.actions().filter_map(|action| {
            if let Action::IBCAction(ibc_action) = action {
                Some(ibc_action)
            } else {
                None
            }
        })
    }

    pub fn validator_definitions(&self) -> impl Iterator<Item = &pbs::ValidatorDefinition> {
        self.actions().filter_map(|action| {
            if let Action::ValidatorDefinition(d) = action {
                Some(d)
            } else {
                None
            }
        })
    }

    pub fn output_bodies(&self) -> Vec<output::Body> {
        self.transaction_body
            .actions
            .iter()
            .filter_map(|action| {
                if let Action::Output(output) = action {
                    Some(output.body.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn spent_nullifiers(&self) -> Vec<Nullifier> {
        self.transaction_body
            .actions
            .iter()
            .filter_map(|action| {
                // Note: adding future actions that include nullifiers
                // will need to be matched here as well as Spends
                if let Action::Spend(spend) = action {
                    Some(spend.body.nullifier.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn transaction_body(&self) -> TransactionBody {
        self.transaction_body.clone()
    }

    pub fn binding_sig(&self) -> &Signature<Binding> {
        &self.binding_sig
    }

    pub fn id(&self) -> [u8; 32] {
        use sha2::{Digest, Sha256};

        let tx_bytes: Vec<u8> = self.clone().try_into().expect("can serialize transaction");
        let mut id_bytes = [0; 32];
        id_bytes[..].copy_from_slice(Sha256::digest(&tx_bytes).as_slice());

        id_bytes
    }

    /// Verify the binding signature.
    pub fn binding_verification_key(&self) -> VerificationKey<Binding> {
        let mut value_commitments = decaf377::Element::default();
        for action in &self.transaction_body.actions {
            value_commitments += action.value_commitment().0;
        }

        // Add fee into binding verification key computation.
        let fee_value = Value {
            amount: self.transaction_body.fee.0,
            asset_id: *STAKING_TOKEN_ASSET_ID,
        };
        let fee_v_blinding = Fr::zero();
        let fee_value_commitment = fee_value.commit(fee_v_blinding);
        value_commitments -= fee_value_commitment.0;

        let binding_verification_key_bytes: VerificationKeyBytes<Binding> =
            value_commitments.compress().0.into();

        binding_verification_key_bytes
            .try_into()
            .expect("verification key is valid")
    }
}

impl From<TransactionBody> for Vec<u8> {
    fn from(transaction_body: TransactionBody) -> Vec<u8> {
        let protobuf_serialized: pbt::TransactionBody = transaction_body.into();
        protobuf_serialized.encode_to_vec()
    }
}

impl Protobuf<pbt::TransactionBody> for TransactionBody {}

impl From<TransactionBody> for pbt::TransactionBody {
    fn from(msg: TransactionBody) -> Self {
        pbt::TransactionBody {
            actions: msg.actions.into_iter().map(|x| x.into()).collect(),
            anchor: Bytes::copy_from_slice(&msg.merkle_root.0.to_bytes()),
            expiry_height: msg.expiry_height,
            chain_id: msg.chain_id,
            fee: Some(msg.fee.into()),
        }
    }
}

impl TryFrom<pbt::TransactionBody> for TransactionBody {
    type Error = Error;

    fn try_from(proto: pbt::TransactionBody) -> anyhow::Result<Self, Self::Error> {
        let mut actions = Vec::<Action>::new();
        for action in proto.actions {
            actions.push(
                action
                    .try_into()
                    .map_err(|_| anyhow::anyhow!("transaction body malformed"))?,
            );
        }

        let merkle_root = proto.anchor[..]
            .try_into()
            .map_err(|_| anyhow::anyhow!("transaction body malformed"))?;

        let expiry_height = proto.expiry_height;

        let chain_id = proto.chain_id;

        let fee: Fee = proto
            .fee
            .ok_or(anyhow::anyhow!("transaction body malformed"))?
            .into();

        Ok(TransactionBody {
            actions,
            merkle_root,
            expiry_height,
            chain_id,
            fee,
        })
    }
}
impl Protobuf<pbt::Transaction> for Transaction {}

impl From<Transaction> for pbt::Transaction {
    fn from(msg: Transaction) -> Self {
        let sig_bytes: [u8; 64] = msg.binding_sig.into();
        pbt::Transaction {
            body: Some(msg.transaction_body.into()),
            binding_sig: Bytes::copy_from_slice(&sig_bytes),
        }
    }
}

impl From<&Transaction> for pbt::Transaction {
    fn from(msg: &Transaction) -> Self {
        msg.into()
    }
}

impl TryFrom<pbt::Transaction> for Transaction {
    type Error = Error;

    fn try_from(proto: pbt::Transaction) -> anyhow::Result<Self, Self::Error> {
        let transaction_body = proto
            .body
            .ok_or(anyhow::anyhow!("transaction malformed"))?
            .try_into()
            .map_err(|_| anyhow::anyhow!("transaction body malformed"))?;

        let sig_bytes: [u8; 64] = proto.binding_sig[..]
            .try_into()
            .map_err(|_| anyhow::anyhow!("transaction malformed"))?;

        Ok(Transaction {
            transaction_body,
            binding_sig: sig_bytes.into(),
        })
    }
}

impl TryFrom<&[u8]> for Transaction {
    type Error = Error;

    fn try_from(bytes: &[u8]) -> Result<Transaction, Self::Error> {
        let protobuf_serialized_proof = pbt::Transaction::decode(bytes)
            .map_err(|_| anyhow::anyhow!("transaction malformed"))?;
        protobuf_serialized_proof
            .try_into()
            .map_err(|_| anyhow::anyhow!("transaction malformed"))
    }
}

impl TryFrom<Vec<u8>> for Transaction {
    type Error = Error;

    fn try_from(bytes: Vec<u8>) -> Result<Transaction, Self::Error> {
        Self::try_from(&bytes[..])
    }
}

impl From<Transaction> for Vec<u8> {
    fn from(transaction: Transaction) -> Vec<u8> {
        let protobuf_serialized: pbt::Transaction = transaction.into();
        protobuf_serialized.encode_to_vec()
    }
}

impl From<&Transaction> for Vec<u8> {
    fn from(transaction: &Transaction) -> Vec<u8> {
        let protobuf_serialized: pbt::Transaction = transaction.into();
        protobuf_serialized.encode_to_vec()
    }
}

impl Protobuf<pbt::Fee> for Fee {}

impl From<Fee> for pbt::Fee {
    fn from(fee: Fee) -> Self {
        pbt::Fee { amount: fee.0 }
    }
}

impl From<pbt::Fee> for Fee {
    fn from(proto: pbt::Fee) -> Self {
        Fee(proto.amount)
    }
}

#[cfg(test)]
mod tests {
    use penumbra_crypto::{
        keys::{SeedPhrase, SpendKey, SpendSeed},
        memo::MemoPlaintext,
        Fq, Value,
    };
    use rand_core::OsRng;

    use super::*;
    use crate::Error;

    #[test]
    fn test_transaction_single_output_fails_due_to_nonzero_value_balance() {
        let mut rng = OsRng;
        let seed_phrase = SeedPhrase::generate(&mut rng);
        let spend_seed = SpendSeed::from_seed_phrase(seed_phrase, 0);
        let sk_sender = SpendKey::new(spend_seed);
        let fvk_sender = sk_sender.full_viewing_key();
        let ovk_sender = fvk_sender.outgoing();

        let seed_phrase = SeedPhrase::generate(&mut rng);
        let spend_seed = SpendSeed::from_seed_phrase(seed_phrase, 0);
        let sk_recipient = SpendKey::new(spend_seed);
        let fvk_recipient = sk_recipient.full_viewing_key();
        let ivk_recipient = fvk_recipient.incoming();
        let (dest, _dtk_d) = ivk_recipient.payment_address(0u64.into());

        let merkle_root = merkle::Root(Fq::zero());
        let transaction = Transaction::build_with_root(merkle_root)
            .set_fee(20)
            .set_chain_id("penumbra".to_string())
            .add_output(
                &mut rng,
                &dest,
                Value {
                    amount: 10,
                    asset_id: *STAKING_TOKEN_ASSET_ID,
                },
                MemoPlaintext::default(),
                ovk_sender,
            )
            .finalize(&mut rng);

        assert!(transaction.is_err());
        assert_eq!(transaction.err(), Some(Error::NonZeroValueBalance));
    }
}
