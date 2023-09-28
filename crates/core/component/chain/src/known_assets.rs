use penumbra_asset::asset::{self, DenomMetadata};
use penumbra_proto::{penumbra::core::component::chain::v1alpha1 as pb, DomainType, TypeUrl};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(try_from = "pb::KnownAssets", into = "pb::KnownAssets")]
pub struct KnownAssets(pub Vec<DenomMetadata>);

impl TypeUrl for KnownAssets {
    const TYPE_URL: &'static str = "/penumbra.core.chain.v1alpha1.KnownAssets";
}

impl DomainType for KnownAssets {
    type Proto = pb::KnownAssets;
}

impl TryFrom<pb::KnownAssets> for KnownAssets {
    type Error = anyhow::Error;
    fn try_from(known_assets: pb::KnownAssets) -> anyhow::Result<Self> {
        Ok(KnownAssets(
            known_assets
                .assets
                .into_iter()
                .map(|asset| asset.try_into())
                .collect::<anyhow::Result<Vec<DenomMetadata>>>()?,
        ))
    }
}

impl From<KnownAssets> for pb::KnownAssets {
    fn from(known_assets: KnownAssets) -> Self {
        Self {
            assets: known_assets
                .0
                .into_iter()
                .map(|asset| asset.into())
                .collect(),
        }
    }
}

impl From<KnownAssets> for asset::Cache {
    fn from(assets: KnownAssets) -> Self {
        Self::from_iter(assets.0)
    }
}
