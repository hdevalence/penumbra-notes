use std::convert::{TryFrom, TryInto};

use ark_ff::Zero;
use bytes::Bytes;

use decaf377::FieldExt;

use penumbra_proto::{transaction, Message, Protobuf};

use crate::{
    action::{error::ProtoError, Action},
    merkle,
    rdsa::{Binding, Signature, VerificationKey, VerificationKeyBytes},
    Fr,
};

mod builder;
pub use builder::Builder;

#[derive(Clone)]
pub struct TransactionBody {
    pub actions: Vec<Action>,
    pub merkle_root: merkle::Root,
    pub expiry_height: u32,
    pub chain_id: String,
    pub fee: Fee,
}

impl Into<Vec<u8>> for TransactionBody {
    fn into(self) -> Vec<u8> {
        let protobuf_serialized: transaction::TransactionBody = self.into();
        protobuf_serialized.encode_to_vec()
    }
}

impl Protobuf<transaction::TransactionBody> for TransactionBody {}

impl From<TransactionBody> for transaction::TransactionBody {
    fn from(msg: TransactionBody) -> Self {
        transaction::TransactionBody {
            actions: msg.actions.into_iter().map(|x| x.into()).collect(),
            anchor: Bytes::copy_from_slice(&msg.merkle_root.0.to_bytes()),
            expiry_height: msg.expiry_height,
            chain_id: msg.chain_id,
            fee: Some(msg.fee.into()),
        }
    }
}

impl TryFrom<transaction::TransactionBody> for TransactionBody {
    type Error = ProtoError;

    fn try_from(proto: transaction::TransactionBody) -> anyhow::Result<Self, Self::Error> {
        let mut actions = Vec::<Action>::new();
        for action in proto.actions {
            actions.push(
                action
                    .try_into()
                    .map_err(|_| ProtoError::TransactionBodyMalformed)?,
            );
        }

        let merkle_root = proto.anchor[..]
            .try_into()
            .map_err(|_| ProtoError::TransactionBodyMalformed)?;

        let expiry_height = proto
            .expiry_height
            .try_into()
            .map_err(|_| ProtoError::TransactionBodyMalformed)?;

        let chain_id = proto
            .chain_id
            .try_into()
            .map_err(|_| ProtoError::TransactionBodyMalformed)?;

        let fee: Fee = proto
            .fee
            .ok_or(ProtoError::TransactionBodyMalformed)?
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

#[derive(Clone, Debug)]
pub struct Fee(pub u64);

// temp: remove dead code when Transaction fields are read
#[allow(dead_code)]
pub struct Transaction {
    transaction_body: TransactionBody,
    binding_sig: Signature<Binding>,
}

impl Transaction {
    /// Start building a transaction relative to a given [`merkle::Root`].
    pub fn build_with_root(merkle_root: merkle::Root) -> Builder {
        Builder {
            actions: Vec::new(),
            fee: None,
            synthetic_blinding_factor: Fr::zero(),
            value_balance: decaf377::Element::default(),
            value_commitments: decaf377::Element::default(),
            merkle_root,
            expiry_height: None,
            chain_id: None,
        }
    }

    pub fn transaction_body(&self) -> TransactionBody {
        self.transaction_body.clone()
    }

    pub fn binding_sig(&self) -> Signature<Binding> {
        self.binding_sig
    }

    /// Verify the binding signature.
    pub fn verify_binding_sig(&self) -> bool {
        let mut value_commitments = decaf377::Element::default();
        for action in self.transaction_body.actions.clone() {
            match action {
                Action::Output(inner) => {
                    value_commitments -= inner.body.value_commitment.0;
                }
                Action::Spend(inner) => {
                    value_commitments += inner.body.value_commitment.0;
                }
            }
        }

        let binding_verification_key_bytes: VerificationKeyBytes<Binding> =
            value_commitments.compress().0.into();
        let binding_verification_key: VerificationKey<Binding> = binding_verification_key_bytes
            .try_into()
            .expect("verification key is valid");

        let transaction_body_serialized: Vec<u8> = self.transaction_body.clone().into();
        binding_verification_key
            .verify(&transaction_body_serialized, &self.binding_sig)
            .is_ok()
    }
}

impl Protobuf<transaction::Transaction> for Transaction {}

impl From<Transaction> for transaction::Transaction {
    fn from(msg: Transaction) -> Self {
        let sig_bytes: [u8; 64] = msg.binding_sig.into();
        transaction::Transaction {
            body: Some(msg.transaction_body.into()),
            binding_sig: Bytes::copy_from_slice(&sig_bytes),
        }
    }
}

impl TryFrom<transaction::Transaction> for Transaction {
    type Error = ProtoError;

    fn try_from(proto: transaction::Transaction) -> anyhow::Result<Self, Self::Error> {
        let transaction_body = proto
            .body
            .ok_or(ProtoError::TransactionMalformed)?
            .try_into()
            .map_err(|_| ProtoError::TransactionBodyMalformed)?;

        let sig_bytes: [u8; 64] = proto.binding_sig[..]
            .try_into()
            .map_err(|_| ProtoError::TransactionMalformed)?;

        Ok(Transaction {
            transaction_body,
            binding_sig: sig_bytes.into(),
        })
    }
}

impl Protobuf<transaction::Fee> for Fee {}

impl From<Fee> for transaction::Fee {
    fn from(fee: Fee) -> Self {
        transaction::Fee { amount: fee.0 }
    }
}

impl From<transaction::Fee> for Fee {
    fn from(proto: transaction::Fee) -> Self {
        Fee(proto.amount)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use rand_core::OsRng;

    use crate::keys::SpendKey;
    use crate::memo::MemoPlaintext;
    use crate::transaction::builder::Error;
    use crate::{note, Fq, Note, Value};

    #[test]
    fn test_transaction_single_output_fails_due_to_nonzero_value_balance() {
        let mut rng = OsRng;
        let sk_sender = SpendKey::generate(&mut rng);
        let fvk_sender = sk_sender.full_viewing_key();
        let ovk_sender = fvk_sender.outgoing();

        let sk_recipient = SpendKey::generate(&mut rng);
        let fvk_recipient = sk_recipient.full_viewing_key();
        let ivk_recipient = fvk_recipient.incoming();
        let (dest, _dtk_d) = ivk_recipient.payment_address(0u64.into());

        let merkle_root = merkle::Root(Fq::zero());
        let transaction = Transaction::build_with_root(merkle_root)
            .set_fee(20)
            .set_chain_id("Pen".to_string())
            .add_output(
                &mut rng,
                &dest,
                Value {
                    amount: 10,
                    asset_id: b"pen".as_ref().into(),
                },
                MemoPlaintext::default(),
                ovk_sender,
            )
            .finalize(&mut rng);

        assert!(transaction.is_err());
        assert_eq!(transaction.err(), Some(Error::NonZeroValueBalance));
    }

    #[test]
    fn test_transaction_succeeds_if_values_balance() {
        let mut rng = OsRng;
        let sk_sender = SpendKey::generate(&mut rng);
        let fvk_sender = sk_sender.full_viewing_key();
        let ovk_sender = fvk_sender.outgoing();

        let sk_recipient = SpendKey::generate(&mut rng);
        let fvk_recipient = sk_recipient.full_viewing_key();
        let ivk_recipient = fvk_recipient.incoming();
        let (dest, _dtk_d) = ivk_recipient.payment_address(0u64.into());

        let merkle_root = merkle::Root(Fq::zero());
        let mut merkle_siblings = Vec::new();
        for _i in 0..merkle::DEPTH {
            merkle_siblings.push(note::Commitment(Fq::zero()))
        }
        let dummy_merkle_path: merkle::Path = (merkle::DEPTH, merkle_siblings);

        let value_to_send = Value {
            amount: 10,
            asset_id: b"pen".as_ref().into(),
        };
        let dummy_note = Note::new(
            *dest.diversifier(),
            dest.transmission_key(),
            value_to_send,
            Fq::zero(),
        )
        .expect("transmission key is valid");

        let transaction = Transaction::build_with_root(merkle_root)
            .set_fee(20)
            .set_chain_id("Pen".to_string())
            .add_output(
                &mut rng,
                &dest,
                value_to_send,
                MemoPlaintext::default(),
                ovk_sender,
            )
            .add_spend(&mut rng, sk_sender, dummy_merkle_path, dummy_note, 0.into())
            .finalize(&mut rng);

        assert!(transaction.is_ok());
        assert_eq!(transaction.unwrap().transaction_body.expiry_height, 0);
    }
}
