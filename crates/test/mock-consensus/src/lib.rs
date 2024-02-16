//! `penumbra-mock-consensus` is a library for testing consensus-driven applications.
//!
//! See [`TestNode`] for more information.
//
//  see penumbra-zone/penumbra#3588.

pub mod builder;

mod abci;
mod block;

/// A test node.
///
/// Construct a new test node by calling [`TestNode::builder()`]. Use [`TestNode::block()`] to
/// build a new [`Block`].
///
/// This contains a consensus service `C`, which should be a [`tower::Service`] implementor that
/// accepts [`ConsensusRequest`][0_37::abci::ConsensusRequest]s, and returns
/// [`ConsensusResponse`][0_37::abci::ConsensusResponse]s. For `tower-abci` users, this should
/// correspond with the `ConsensusService` parameter of the `Server` type.
pub struct TestNode<C> {
    consensus: C,
    last_app_hash: Vec<u8>,
    height: tendermint::block::Height,
}

impl<C> TestNode<C> {
    /// Returns the last app_hash value, as a slice of bytes.
    pub fn last_app_hash(&self) -> &[u8] {
        &self.last_app_hash
    }

    /// Returns the last app_hash value, as a hexadecimal string.
    pub fn last_app_hash_hex(&self) -> String {
        // Use upper-case hexadecimal integers, include leading zeroes.
        // - https://doc.rust-lang.org/std/fmt/#formatting-traits
        format!("{:02X?}", self.last_app_hash)
    }
}
