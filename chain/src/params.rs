use num_rational::Ratio;
use penumbra_crypto::asset;
use penumbra_proto::{chain as pb, crypto as pbc, Protobuf};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
pub struct AssetInfo {
    pub asset_id: asset::Id,
    pub denom: asset::Denom,
    pub as_of_block_height: u64,
    pub total_supply: u64,
}

impl Protobuf<pb::AssetInfo> for AssetInfo {}

impl TryFrom<pb::AssetInfo> for AssetInfo {
    type Error = anyhow::Error;

    fn try_from(msg: pb::AssetInfo) -> Result<Self, Self::Error> {
        Ok(AssetInfo {
            asset_id: asset::Id::try_from(msg.asset_id.unwrap())?,
            denom: asset::Denom::try_from(msg.denom.unwrap())?,
            as_of_block_height: msg.as_of_block_height,
            total_supply: msg.total_supply,
        })
    }
}

impl From<AssetInfo> for pb::AssetInfo {
    fn from(ai: AssetInfo) -> Self {
        pb::AssetInfo {
            asset_id: Some(pbc::AssetId::from(ai.asset_id)),
            denom: Some(pbc::Denom::from(ai.denom)),
            as_of_block_height: ai.as_of_block_height,
            total_supply: ai.total_supply,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(try_from = "pb::ChainParameters", into = "pb::ChainParameters")]
pub struct ChainParameters {
    pub chain_id: String,
    pub epoch_duration: u64,

    pub unbonding_epochs: u64,
    /// The number of validators allowed in the consensus set (Active state).
    pub active_validator_limit: u64,
    /// The base reward rate, expressed in basis points of basis points
    pub base_reward_rate: u64,
    /// The penalty for slashing due to misbehavior, expressed in basis points.
    pub slashing_penalty_misbehavior_bps: u64,
    /// The penalty for slashing due to downtime, expressed in basis points.
    pub slashing_penalty_downtime_bps: u64,
    /// The number of blocks in the window to check for downtime.
    pub signed_blocks_window_len: u64,
    /// The maximum number of blocks in the window each validator can miss signing without slashing.
    pub missed_blocks_maximum: u64,

    /// Whether IBC (forming connections, processing IBC packets) is enabled.
    pub ibc_enabled: bool,
    /// Whether inbound ICS-20 transfers are enabled
    pub inbound_ics20_transfers_enabled: bool,
    /// Whether outbound ICS-20 transfers are enabled
    pub outbound_ics20_transfers_enabled: bool,

    /// The number of epochs during which a proposal is voted on.
    pub proposal_voting_epochs: u64,
    /// The deposit required to create a proposal.
    pub proposal_deposit_amount: u64,
    /// The quorum required for a proposal to be considered valid, as a fraction of the total stake
    /// weight of the network.
    pub proposal_valid_quorum: Ratio<u64>,
    /// The threshold for a proposal to pass voting, as a ratio of "yes" votes over "no" votes.
    pub proposal_pass_threshold: Ratio<u64>,
    /// The threshold for a proposal to be vetoed, regardless of whether the "yes" and "no" votes
    /// would have passed it, as a ratio of "no with veto" votes over all total votes.
    pub proposal_veto_threshold: Ratio<u64>,
}

impl Protobuf<pb::ChainParameters> for ChainParameters {}

impl TryFrom<pb::ChainParameters> for ChainParameters {
    type Error = anyhow::Error;

    fn try_from(msg: pb::ChainParameters) -> anyhow::Result<Self> {
        Ok(ChainParameters {
            chain_id: msg.chain_id,
            epoch_duration: msg.epoch_duration,
            unbonding_epochs: msg.unbonding_epochs,
            active_validator_limit: msg.active_validator_limit,
            slashing_penalty_downtime_bps: msg.slashing_penalty_downtime_bps,
            slashing_penalty_misbehavior_bps: msg.slashing_penalty_misbehavior_bps,
            base_reward_rate: msg.base_reward_rate,
            missed_blocks_maximum: msg.missed_blocks_maximum,
            signed_blocks_window_len: msg.signed_blocks_window_len,
            ibc_enabled: msg.ibc_enabled,
            inbound_ics20_transfers_enabled: msg.inbound_ics20_transfers_enabled,
            outbound_ics20_transfers_enabled: msg.outbound_ics20_transfers_enabled,
            proposal_voting_epochs: msg.proposal_voting_epochs,
            proposal_deposit_amount: msg.proposal_deposit_amount,
            proposal_valid_quorum: msg
                .proposal_valid_quorum
                .ok_or_else(|| anyhow::anyhow!("missing `proposal_valid_quorum`"))?
                .into(),
            proposal_pass_threshold: msg
                .proposal_pass_threshold
                .ok_or_else(|| anyhow::anyhow!("missing `proposal_pass_threshold`"))?
                .into(),
            proposal_veto_threshold: msg
                .proposal_veto_threshold
                .ok_or_else(|| anyhow::anyhow!("missing `proposal_veto_threshold`"))?
                .into(),
        })
    }
}

impl From<ChainParameters> for pb::ChainParameters {
    fn from(params: ChainParameters) -> Self {
        pb::ChainParameters {
            chain_id: params.chain_id,
            epoch_duration: params.epoch_duration,
            unbonding_epochs: params.unbonding_epochs,
            active_validator_limit: params.active_validator_limit,
            signed_blocks_window_len: params.signed_blocks_window_len,
            missed_blocks_maximum: params.missed_blocks_maximum,
            slashing_penalty_downtime_bps: params.slashing_penalty_downtime_bps,
            slashing_penalty_misbehavior_bps: params.slashing_penalty_misbehavior_bps,
            base_reward_rate: params.base_reward_rate,
            ibc_enabled: params.ibc_enabled,
            inbound_ics20_transfers_enabled: params.inbound_ics20_transfers_enabled,
            outbound_ics20_transfers_enabled: params.outbound_ics20_transfers_enabled,
            proposal_voting_epochs: params.proposal_voting_epochs,
            proposal_deposit_amount: params.proposal_deposit_amount,
            proposal_valid_quorum: Some(params.proposal_valid_quorum.into()),
            proposal_pass_threshold: Some(params.proposal_pass_threshold.into()),
            proposal_veto_threshold: Some(params.proposal_veto_threshold.into()),
        }
    }
}

// TODO: defaults are implemented here as well as in the
// `pd::main`
impl Default for ChainParameters {
    fn default() -> Self {
        Self {
            chain_id: String::new(),
            epoch_duration: 8640,
            unbonding_epochs: 30,
            active_validator_limit: 10,
            // copied from cosmos hub
            signed_blocks_window_len: 10000,
            missed_blocks_maximum: 9500,
            // 1000 basis points = 10%
            slashing_penalty_misbehavior_bps: 1000,
            // 1 basis point = 0.01%
            slashing_penalty_downtime_bps: 1,
            // 3bps -> 11% return over 365 epochs
            base_reward_rate: 3_0000,
            ibc_enabled: true,
            inbound_ics20_transfers_enabled: false,
            outbound_ics20_transfers_enabled: false,
            // governance
            proposal_voting_epochs: 2,
            proposal_deposit_amount: 10_000_000, // 10,000,000 upenumbra = 10 penumbra
            // governance parameters copied from cosmos hub
            proposal_valid_quorum: Ratio::new(2, 5),
            proposal_pass_threshold: Ratio::new(1, 2),
            proposal_veto_threshold: Ratio::new(1, 3),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(try_from = "pb::FmdParameters", into = "pb::FmdParameters")]
pub struct FmdParameters {
    /// Bits of precision.
    pub precision_bits: u8,
    /// The block height at which these parameters became effective.
    pub as_of_block_height: u64,
}

impl Protobuf<pb::FmdParameters> for FmdParameters {}

impl TryFrom<pb::FmdParameters> for FmdParameters {
    type Error = anyhow::Error;

    fn try_from(msg: pb::FmdParameters) -> Result<Self, Self::Error> {
        Ok(FmdParameters {
            precision_bits: msg.precision_bits.try_into()?,
            as_of_block_height: msg.as_of_block_height,
        })
    }
}

impl From<FmdParameters> for pb::FmdParameters {
    fn from(params: FmdParameters) -> Self {
        pb::FmdParameters {
            precision_bits: u32::from(params.precision_bits),
            as_of_block_height: params.as_of_block_height,
        }
    }
}

impl Default for FmdParameters {
    fn default() -> Self {
        Self {
            precision_bits: 0,
            as_of_block_height: 1,
        }
    }
}
