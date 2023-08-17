use anyhow::{Context, Error};
use serde::{Deserialize, Serialize};
use std::convert::{TryFrom, TryInto};

use penumbra_asset::{Balance, Value};
use penumbra_proto::{core::governance::v1alpha1 as pb, DomainType, TypeUrl};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(try_from = "pb::DaoDeposit", into = "pb::DaoDeposit")]
pub struct DaoDeposit {
    pub value: Value,
}

impl DaoDeposit {
    pub fn balance(&self) -> Balance {
        // Deposits into the DAO require value
        -Balance::from(self.value)
    }
}

impl TypeUrl for DaoDeposit {
    const TYPE_URL: &'static str = "/penumbra.core.governance.v1alpha1.DaoDeposit";
}

impl DomainType for DaoDeposit {
    type Proto = pb::DaoDeposit;
}

impl From<DaoDeposit> for pb::DaoDeposit {
    fn from(msg: DaoDeposit) -> Self {
        pb::DaoDeposit {
            value: Some(msg.value.into()),
        }
    }
}

impl TryFrom<pb::DaoDeposit> for DaoDeposit {
    type Error = Error;

    fn try_from(proto: pb::DaoDeposit) -> anyhow::Result<Self, Self::Error> {
        let value = proto
            .value
            .ok_or_else(|| anyhow::anyhow!("missing value"))?
            .try_into()
            .context("malformed value")?;

        Ok(DaoDeposit { value })
    }
}
