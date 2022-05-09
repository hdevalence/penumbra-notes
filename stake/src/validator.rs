//! Penumbra validators and related structures.

use penumbra_proto::{stake as pb, Protobuf};
use serde::{Deserialize, Serialize};

use crate::{FundingStream, FundingStreams, IdentityKey};

mod definition;
mod info;
mod list;
mod state;
mod status;
mod unbonding;

pub use definition::Definition;
pub use info::Info;
pub use list::List;
pub use state::State;
pub use status::Status;
pub use unbonding::Status as UnbondingStatus;

/// Describes a Penumbra validator's configuration data.
///
/// This data is unauthenticated; the
/// [`ValidatorDefinition`](crate::action::ValidatorDefiniition) action includes
/// a signature over the transaction with the validator's identity key.
#[derive(Debug, PartialEq, Eq, Clone, Serialize, Deserialize)]
#[serde(try_from = "pb::Validator", into = "pb::Validator")]
pub struct Validator {
    /// The validator's identity verification key.
    pub identity_key: IdentityKey,

    /// The validator's consensus key, used by Tendermint for signing blocks and
    /// other consensus operations.
    pub consensus_key: tendermint::PublicKey,

    /// The validator's (human-readable) name.
    pub name: String,

    /// The validator's website URL.
    pub website: String,

    /// The validator's description.
    pub description: String,

    /// The destinations for the validator's staking reward. The commission is implicitly defined
    /// by the configuration of funding_streams, the sum of FundingStream.rate_bps.
    ///
    // NOTE: unclaimed rewards are tracked by inserting reward notes for the last epoch into the
    // NCT at the beginning of each epoch
    pub funding_streams: FundingStreams,

    /// The sequence number determines which validator data takes priority, and
    /// prevents replay attacks.  The chain only accepts new
    /// [`ValidatorDefinition`]s with increasing sequence numbers, preventing a
    /// third party from replaying previously valid but stale configuration data
    /// as an update.
    pub sequence_number: u32,
}

impl Protobuf<pb::Validator> for Validator {}

impl From<Validator> for pb::Validator {
    fn from(v: Validator) -> Self {
        pb::Validator {
            identity_key: Some(v.identity_key.into()),
            consensus_key: v.consensus_key.to_bytes(),
            name: v.name,
            website: v.website,
            description: v.description,
            funding_streams: v.funding_streams.into_iter().map(Into::into).collect(),
            sequence_number: v.sequence_number,
        }
    }
}

impl TryFrom<pb::Validator> for Validator {
    type Error = anyhow::Error;
    fn try_from(v: pb::Validator) -> Result<Self, Self::Error> {
        Ok(Validator {
            identity_key: v
                .identity_key
                .ok_or_else(|| anyhow::anyhow!("missing identity key"))?
                .try_into()?,
            consensus_key: tendermint::PublicKey::from_raw_ed25519(&v.consensus_key)
                .ok_or_else(|| anyhow::anyhow!("invalid ed25519 consensus pubkey"))?,
            name: v.name,
            website: v.website,
            description: v.description,
            funding_streams: v
                .funding_streams
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<Vec<FundingStream>, _>>()?
                .try_into()?,
            sequence_number: v.sequence_number,
        })
    }
}
