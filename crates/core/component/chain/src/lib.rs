#![deny(clippy::unwrap_used)]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]

pub use epoch::Epoch;

mod epoch;

/// The substore prefix used for storing historic CometBFT block data.
pub static COMETBFT_SUBSTORE_PREFIX: &'static str = "cometbft-data";

#[cfg(feature = "component")]
pub mod component;

pub mod genesis;
pub mod params;
pub mod state_key;
