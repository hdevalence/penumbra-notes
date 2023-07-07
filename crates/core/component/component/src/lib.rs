//! Core trait definitions for components of the Penumbra application.
//!
//! This crate defines two traits for use by "component crates":
//!
//! - [`Component`], which defines the _internally driven_ behavior of a
//! component, triggered at the beginning and end of blocks and at the ends of
//! epochs;
//! - [`ActionHandler`], which defines the _externally driven_ behavior of a
//! component, triggered by actions in blockchain transactions.
//!
//! Component crates should be structured as follows:
//!
//! - Definitions of any transaction actions related to the component, and their
//!   corresponding plans and views;
//! - a `crate::component` module, feature-gated by the `component` feature,
//!   with the `Component` implementation and `ActionHandler` implementations for
//!   any locally-defined actions, and any other code touching the chain state
//!   inside;
//! - a `crate::state_key` module defining the component's state keys (which are
//!   a public API, like the rest of the chain state);
//! - a `crate::event` module defining any events emitted by the component;
//!
//! The `penumbra_transaction` crate depends on all of the component crates
//! without the `component` feature, so it can assemble all of the actions for
//! each component into a top-level transaction type.  This means all async code
//! should be confined to the `component` module, so that the transaction
//! definitions don't depend on `tokio`, `mio`, etc and can be used without
//! problems in wasm or other non-async contexts.
//!
//! The `penumbra_app` crate depends on all of the component crates with the
//! `component` feature enabled, so it can assemble all of the `ActionHandler`
//! implementations into a top-level `ActionHandler` implementation and call
//! each component's `Component` implementation at the appropriate times.

mod action_handler;
mod component;
mod effect_hash;
mod transaction;

pub use action_handler::ActionHandler;
pub use component::Component;
pub use effect_hash::{EffectHash, EffectingData};
pub use transaction::TransactionContext;
