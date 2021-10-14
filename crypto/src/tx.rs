use rand::seq::SliceRandom;
use rand_core::{CryptoRng, RngCore};

use crate::{
    addresses::PaymentAddress,
    asset,
    keys::{Diversifier, OutgoingViewingKey, SpendKey},
    memo::MemoPlaintext,
    merkle::proof::MerklePath,
    Note, Output, Spend, Value,
};

/// Used to construct a Penumbra transaction.
pub struct TransactionBuilder {
    // Notes we'll consume in this transaction.
    spends: Vec<Spend>,
    // Notes we'll create in this transaction.
    outputs: Vec<Output>,
    // Transaction fee.
    fee: u64,
    // Total value changed in this transaction.
    balance: i64,
    // Put chain_id and anchor in here too?
}

impl TransactionBuilder {
    pub fn new() -> Self {
        Self {
            spends: Vec::<Spend>::new(),
            outputs: Vec::<Output>::new(),
            fee: 0,
            balance: 0,
        }
    }

    /// Create a new `Spend` to spend an existing note.
    pub fn add_spend<R: RngCore + CryptoRng>(
        &mut self,
        rng: &mut R,
        diversifier: &Diversifier,
        spend_key: SpendKey,
        merkle_path: MerklePath,
        note: Note,
    ) {
        self.balance -= note.value.amount as i64;
        let spend = Spend::new(rng, diversifier, spend_key, merkle_path, note);
        self.spends.push(spend);
    }

    /// Create a new `Output` to create a new note.
    pub fn add_output<R: RngCore + CryptoRng>(
        &mut self,
        rng: &mut R,
        dest: &PaymentAddress,
        asset_id: asset::Id,
        amount: u64,
        memo: MemoPlaintext,
        ovk: &OutgoingViewingKey,
    ) {
        let value_to_send = Value { amount, asset_id };
        self.balance += value_to_send.amount as i64;
        let output = Output::new(rng, dest, value_to_send, memo, ovk);
        self.outputs.push(output);
    }

    /// Set the transaction fee in PEN.
    pub fn set_fee(&mut self, fee: u64) {
        self.balance -= fee as i64;
        self.fee = fee
    }

    // xx Uses rand::RngCore instead of RngCore
    pub fn finalize<R: CryptoRng + rand::RngCore + rand::seq::SliceRandom>(
        &mut self,
        rng: &mut R,
    ) -> TransactionBody {
        // Randomize outputs to minimize info leakage.
        self.outputs.shuffle(rng);
        self.spends.shuffle(rng);

        // Apply sig
        todo!();
    }
}

pub struct TransactionBody {}

impl TransactionBody {
    pub fn sign() -> Transaction {
        todo!()
    }
}

pub struct Transaction {}

#[cfg(test)]
mod tests {
    use super::*;

    use rand_core::OsRng;

    // Not really a test - just to exercise the code path for now
    #[test]
    fn test_transaction_create() {
        let mut rng = OsRng;
        let diversifier = Diversifier::generate(&mut rng);
        let sk_sender = SpendKey::generate(&mut rng);
        let fvk_sender = sk_sender.full_viewing_key();
        let ivk_sender = fvk_sender.outgoing();

        let sk_recipient = SpendKey::generate(&mut rng);
        let diversifier_recipient = Diversifier::generate(&mut rng);
        let fvk_recipient = sk_recipient.full_viewing_key();
        let ivk_recipient = fvk_recipient.incoming();
        let dest = PaymentAddress::new(ivk_recipient, diversifier_recipient);

        let mut builder = TransactionBuilder::new();
        builder.set_fee(20);

        let pen_trace = b"pen";
        let pen_id = asset::Id::from(&pen_trace[..]);
        let memo = MemoPlaintext::default();
        builder.add_output(&mut rng, &dest, pen_id, 10, memo, ivk_sender);
    }
}
