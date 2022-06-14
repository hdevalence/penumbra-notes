use anyhow::Result;
use jmt::KeyHash;
use penumbra_proto::Protobuf;

use crate::App;

#[derive(Debug, clap::Subcommand)]
pub enum QueryCmd {
    /// Queries an arbitrary key.
    Key {
        /// The key to query.
        key: String,
    },
    /// Queries shielded pool data.
    #[clap(subcommand)]
    ShieldedPool(ShieldedPool),
}

#[derive(Debug, clap::Subcommand)]
pub enum ShieldedPool {
    /// Queries the note commitment tree anchor for a given height.
    Anchor {
        /// The height to query.
        height: u64,
    },
    /// Queries the scheduled notes and nullifiers to unquarantine at a given epoch.
    QuarantinedToApply {
        /// The epoch to query.
        epoch: u64,
    },
}

impl QueryCmd {
    pub async fn exec(&self, app: &mut App) -> Result<()> {
        let mut client = app.specific_client().await?;

        let key_hash = self.key_hash();

        let req = if let QueryCmd::Key { key } = self {
            penumbra_proto::client::specific::KeyValueRequest {
                key: key.as_bytes().to_vec(),
                ..Default::default()
            }
        } else {
            penumbra_proto::client::specific::KeyValueRequest {
                key_hash: key_hash.0.to_vec(),
                ..Default::default()
            }
        };
        tracing::debug!(?req);

        let rsp = client.key_value(req).await?.into_inner();

        self.display_value(&rsp.value)?;
        Ok(())
    }

    fn key_hash(&self) -> KeyHash {
        match self {
            QueryCmd::Key { key } => key.as_bytes().into(),
            QueryCmd::ShieldedPool(sp) => sp.key_hash(),
        }
    }

    fn display_value(&self, bytes: &[u8]) -> Result<()> {
        match self {
            QueryCmd::Key { .. } => {
                println!("{}", hex::encode(bytes));
            }
            QueryCmd::ShieldedPool(sp) => sp.display_value(bytes)?,
        }

        Ok(())
    }
}

impl ShieldedPool {
    fn key_hash(&self) -> KeyHash {
        use penumbra_component::shielded_pool::state_key;
        match self {
            ShieldedPool::Anchor { height } => state_key::anchor_by_height(height),
            ShieldedPool::QuarantinedToApply { epoch } => state_key::quarantined_to_apply(*epoch),
        }
    }

    fn display_value(&self, bytes: &[u8]) -> Result<()> {
        match self {
            ShieldedPool::Anchor { .. } => {
                let anchor = penumbra_tct::Root::decode(bytes)?;
                println!("{:#?}", anchor);
            }
            ShieldedPool::QuarantinedToApply { .. } => {
                let notes = penumbra_component::shielded_pool::Quarantined::decode(bytes)?;
                println!("{:#?}", notes);
            }
        }
        Ok(())
    }
}
