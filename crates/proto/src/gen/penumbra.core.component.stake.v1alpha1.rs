/// A Penumbra ZK undelegate claim proof.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ZkUndelegateClaimProof {
    #[prost(bytes = "vec", tag = "1")]
    pub inner: ::prost::alloc::vec::Vec<u8>,
}
/// Describes a validator's configuration data.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Validator {
    /// The validator's identity verification key.
    #[prost(message, optional, tag = "1")]
    pub identity_key: ::core::option::Option<
        super::super::super::keys::v1alpha1::IdentityKey,
    >,
    /// The validator's consensus pubkey for use in Tendermint (Ed25519).
    #[prost(bytes = "vec", tag = "2")]
    pub consensus_key: ::prost::alloc::vec::Vec<u8>,
    /// The validator's (human-readable) name.
    #[prost(string, tag = "3")]
    pub name: ::prost::alloc::string::String,
    /// The validator's website.
    #[prost(string, tag = "4")]
    pub website: ::prost::alloc::string::String,
    /// The validator's description.
    #[prost(string, tag = "5")]
    pub description: ::prost::alloc::string::String,
    /// Whether the validator is enabled or not.
    ///
    /// Disabled validators cannot be delegated to, and immediately begin unbonding.
    #[prost(bool, tag = "8")]
    pub enabled: bool,
    /// A list of funding streams describing the validator's commission.
    #[prost(message, repeated, tag = "6")]
    pub funding_streams: ::prost::alloc::vec::Vec<FundingStream>,
    /// The sequence number determines which validator data takes priority, and
    /// prevents replay attacks.  The chain only accepts new validator definitions
    /// with increasing sequence numbers.
    #[prost(uint32, tag = "7")]
    pub sequence_number: u32,
    /// The validator's governance key.
    #[prost(message, optional, tag = "9")]
    pub governance_key: ::core::option::Option<
        super::super::super::keys::v1alpha1::GovernanceKey,
    >,
}
/// For storing the list of keys of known validators.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ValidatorList {
    #[prost(message, repeated, tag = "1")]
    pub validator_keys: ::prost::alloc::vec::Vec<
        super::super::super::keys::v1alpha1::IdentityKey,
    >,
}
/// A portion of a validator's commission.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct FundingStream {
    /// The recipient of the funding stream.
    #[prost(oneof = "funding_stream::Recipient", tags = "1, 2")]
    pub recipient: ::core::option::Option<funding_stream::Recipient>,
}
/// Nested message and enum types in `FundingStream`.
pub mod funding_stream {
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct ToAddress {
        /// The destination address for the funding stream.
        #[prost(string, tag = "1")]
        pub address: ::prost::alloc::string::String,
        /// The portion of the staking reward for the entire delegation pool
        /// allocated to this funding stream, specified in basis points.
        #[prost(uint32, tag = "2")]
        pub rate_bps: u32,
    }
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Message)]
    pub struct ToDao {
        /// The portion of the staking reward for the entire delegation pool
        /// allocated to this funding stream, specified in basis points.
        #[prost(uint32, tag = "2")]
        pub rate_bps: u32,
    }
    /// The recipient of the funding stream.
    #[allow(clippy::derive_partial_eq_without_eq)]
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Recipient {
        #[prost(message, tag = "1")]
        ToAddress(ToAddress),
        #[prost(message, tag = "2")]
        ToDao(ToDao),
    }
}
/// Describes the reward and exchange rates and voting power for a validator in some epoch.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RateData {
    #[prost(message, optional, tag = "1")]
    pub identity_key: ::core::option::Option<
        super::super::super::keys::v1alpha1::IdentityKey,
    >,
    #[prost(uint64, tag = "2")]
    pub epoch_index: u64,
    #[prost(uint64, tag = "4")]
    pub validator_reward_rate: u64,
    #[prost(uint64, tag = "5")]
    pub validator_exchange_rate: u64,
}
/// Describes the base reward and exchange rates in some epoch.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BaseRateData {
    #[prost(uint64, tag = "1")]
    pub epoch_index: u64,
    #[prost(uint64, tag = "2")]
    pub base_reward_rate: u64,
    #[prost(uint64, tag = "3")]
    pub base_exchange_rate: u64,
}
/// Describes the current state of a validator on-chain
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ValidatorStatus {
    #[prost(message, optional, tag = "1")]
    pub identity_key: ::core::option::Option<
        super::super::super::keys::v1alpha1::IdentityKey,
    >,
    #[prost(message, optional, tag = "2")]
    pub state: ::core::option::Option<ValidatorState>,
    #[prost(uint64, tag = "3")]
    pub voting_power: u64,
    #[prost(message, optional, tag = "4")]
    pub bonding_state: ::core::option::Option<BondingState>,
}
/// Describes the unbonding state of a validator's stake pool.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BondingState {
    #[prost(enumeration = "bonding_state::BondingStateEnum", tag = "1")]
    pub state: i32,
    #[prost(uint64, tag = "2")]
    pub unbonding_epoch: u64,
}
/// Nested message and enum types in `BondingState`.
pub mod bonding_state {
    #[derive(
        Clone,
        Copy,
        Debug,
        PartialEq,
        Eq,
        Hash,
        PartialOrd,
        Ord,
        ::prost::Enumeration
    )]
    #[repr(i32)]
    pub enum BondingStateEnum {
        Unspecified = 0,
        Bonded = 1,
        Unbonding = 2,
        Unbonded = 3,
    }
    impl BondingStateEnum {
        /// String value of the enum field names used in the ProtoBuf definition.
        ///
        /// The values are not transformed in any way and thus are considered stable
        /// (if the ProtoBuf definition does not change) and safe for programmatic use.
        pub fn as_str_name(&self) -> &'static str {
            match self {
                BondingStateEnum::Unspecified => "BONDING_STATE_ENUM_UNSPECIFIED",
                BondingStateEnum::Bonded => "BONDING_STATE_ENUM_BONDED",
                BondingStateEnum::Unbonding => "BONDING_STATE_ENUM_UNBONDING",
                BondingStateEnum::Unbonded => "BONDING_STATE_ENUM_UNBONDED",
            }
        }
        /// Creates an enum from field names used in the ProtoBuf definition.
        pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
            match value {
                "BONDING_STATE_ENUM_UNSPECIFIED" => Some(Self::Unspecified),
                "BONDING_STATE_ENUM_BONDED" => Some(Self::Bonded),
                "BONDING_STATE_ENUM_UNBONDING" => Some(Self::Unbonding),
                "BONDING_STATE_ENUM_UNBONDED" => Some(Self::Unbonded),
                _ => None,
            }
        }
    }
}
/// Describes the state of a validator
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ValidatorState {
    #[prost(enumeration = "validator_state::ValidatorStateEnum", tag = "1")]
    pub state: i32,
}
/// Nested message and enum types in `ValidatorState`.
pub mod validator_state {
    #[derive(
        Clone,
        Copy,
        Debug,
        PartialEq,
        Eq,
        Hash,
        PartialOrd,
        Ord,
        ::prost::Enumeration
    )]
    #[repr(i32)]
    pub enum ValidatorStateEnum {
        Unspecified = 0,
        Inactive = 1,
        Active = 2,
        Jailed = 3,
        Tombstoned = 4,
        Disabled = 5,
    }
    impl ValidatorStateEnum {
        /// String value of the enum field names used in the ProtoBuf definition.
        ///
        /// The values are not transformed in any way and thus are considered stable
        /// (if the ProtoBuf definition does not change) and safe for programmatic use.
        pub fn as_str_name(&self) -> &'static str {
            match self {
                ValidatorStateEnum::Unspecified => "VALIDATOR_STATE_ENUM_UNSPECIFIED",
                ValidatorStateEnum::Inactive => "VALIDATOR_STATE_ENUM_INACTIVE",
                ValidatorStateEnum::Active => "VALIDATOR_STATE_ENUM_ACTIVE",
                ValidatorStateEnum::Jailed => "VALIDATOR_STATE_ENUM_JAILED",
                ValidatorStateEnum::Tombstoned => "VALIDATOR_STATE_ENUM_TOMBSTONED",
                ValidatorStateEnum::Disabled => "VALIDATOR_STATE_ENUM_DISABLED",
            }
        }
        /// Creates an enum from field names used in the ProtoBuf definition.
        pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
            match value {
                "VALIDATOR_STATE_ENUM_UNSPECIFIED" => Some(Self::Unspecified),
                "VALIDATOR_STATE_ENUM_INACTIVE" => Some(Self::Inactive),
                "VALIDATOR_STATE_ENUM_ACTIVE" => Some(Self::Active),
                "VALIDATOR_STATE_ENUM_JAILED" => Some(Self::Jailed),
                "VALIDATOR_STATE_ENUM_TOMBSTONED" => Some(Self::Tombstoned),
                "VALIDATOR_STATE_ENUM_DISABLED" => Some(Self::Disabled),
                _ => None,
            }
        }
    }
}
/// Combines all validator info into a single packet.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ValidatorInfo {
    #[prost(message, optional, tag = "1")]
    pub validator: ::core::option::Option<Validator>,
    #[prost(message, optional, tag = "2")]
    pub status: ::core::option::Option<ValidatorStatus>,
    #[prost(message, optional, tag = "3")]
    pub rate_data: ::core::option::Option<RateData>,
}
/// A transaction action (re)defining a validator.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ValidatorDefinition {
    /// The configuration data for the validator.
    #[prost(message, optional, tag = "1")]
    pub validator: ::core::option::Option<Validator>,
    /// A signature by the validator's identity key over the validator data.
    #[prost(bytes = "vec", tag = "2")]
    pub auth_sig: ::prost::alloc::vec::Vec<u8>,
}
/// A transaction action adding stake to a validator's delegation pool.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Delegate {
    /// The identity key of the validator to delegate to.
    #[prost(message, optional, tag = "1")]
    pub validator_identity: ::core::option::Option<
        super::super::super::keys::v1alpha1::IdentityKey,
    >,
    /// The index of the epoch in which this delegation was performed.
    /// The delegation takes effect in the next epoch.
    #[prost(uint64, tag = "2")]
    pub epoch_index: u64,
    /// The delegation amount, in units of unbonded stake.
    /// TODO: use flow aggregation to hide this, replacing it with bytes amount_ciphertext;
    #[prost(message, optional, tag = "3")]
    pub unbonded_amount: ::core::option::Option<
        super::super::super::num::v1alpha1::Amount,
    >,
    /// The amount of delegation tokens produced by this action.
    ///
    /// This is implied by the validator's exchange rate in the specified epoch
    /// (and should be checked in transaction validation!), but including it allows
    /// stateless verification that the transaction is internally consistent.
    #[prost(message, optional, tag = "4")]
    pub delegation_amount: ::core::option::Option<
        super::super::super::num::v1alpha1::Amount,
    >,
}
/// A transaction action withdrawing stake from a validator's delegation pool.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Undelegate {
    /// The identity key of the validator to undelegate from.
    #[prost(message, optional, tag = "1")]
    pub validator_identity: ::core::option::Option<
        super::super::super::keys::v1alpha1::IdentityKey,
    >,
    /// The index of the epoch in which this undelegation was performed.
    #[prost(uint64, tag = "2")]
    pub start_epoch_index: u64,
    /// The amount to undelegate, in units of unbonding tokens.
    #[prost(message, optional, tag = "3")]
    pub unbonded_amount: ::core::option::Option<
        super::super::super::num::v1alpha1::Amount,
    >,
    /// The amount of delegation tokens consumed by this action.
    ///
    /// This is implied by the validator's exchange rate in the specified epoch
    /// (and should be checked in transaction validation!), but including it allows
    /// stateless verification that the transaction is internally consistent.
    #[prost(message, optional, tag = "4")]
    pub delegation_amount: ::core::option::Option<
        super::super::super::num::v1alpha1::Amount,
    >,
}
/// A transaction action finishing an undelegation, converting (slashable)
/// "unbonding tokens" to (unslashable) staking tokens.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UndelegateClaim {
    #[prost(message, optional, tag = "1")]
    pub body: ::core::option::Option<UndelegateClaimBody>,
    #[prost(bytes = "vec", tag = "2")]
    pub proof: ::prost::alloc::vec::Vec<u8>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UndelegateClaimBody {
    /// The identity key of the validator to finish undelegating from.
    #[prost(message, optional, tag = "1")]
    pub validator_identity: ::core::option::Option<
        super::super::super::keys::v1alpha1::IdentityKey,
    >,
    /// The epoch in which unbonding began, used to verify the penalty.
    #[prost(uint64, tag = "2")]
    pub start_epoch_index: u64,
    /// The penalty applied to undelegation, in bps^2 (10e-8).
    /// In the happy path (no slashing), this is 0.
    #[prost(message, optional, tag = "3")]
    pub penalty: ::core::option::Option<Penalty>,
    /// The action's contribution to the transaction's value balance.
    #[prost(message, optional, tag = "4")]
    pub balance_commitment: ::core::option::Option<
        super::super::super::asset::v1alpha1::BalanceCommitment,
    >,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UndelegateClaimPlan {
    /// The identity key of the validator to finish undelegating from.
    #[prost(message, optional, tag = "1")]
    pub validator_identity: ::core::option::Option<
        super::super::super::keys::v1alpha1::IdentityKey,
    >,
    /// The epoch in which unbonding began, used to verify the penalty.
    #[prost(uint64, tag = "2")]
    pub start_epoch_index: u64,
    /// The penalty applied to undelegation, in bps^2 (10e-8).
    /// In the happy path (no slashing), this is 0.
    #[prost(message, optional, tag = "4")]
    pub penalty: ::core::option::Option<Penalty>,
    /// The amount of unbonding tokens to claim.
    /// This is a bare number because its denom is determined by the preceding data.
    #[prost(message, optional, tag = "5")]
    pub unbonding_amount: ::core::option::Option<
        super::super::super::num::v1alpha1::Amount,
    >,
    /// The blinding factor to use for the balance commitment.
    #[prost(bytes = "vec", tag = "6")]
    pub balance_blinding: ::prost::alloc::vec::Vec<u8>,
    /// The first blinding factor to use for the ZK undelegate claim proof.
    #[prost(bytes = "vec", tag = "7")]
    pub proof_blinding_r: ::prost::alloc::vec::Vec<u8>,
    /// The second blinding factor to use for the ZK undelegate claim proof.
    #[prost(bytes = "vec", tag = "8")]
    pub proof_blinding_s: ::prost::alloc::vec::Vec<u8>,
}
/// A list of pending delegations and undelegations.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct DelegationChanges {
    #[prost(message, repeated, tag = "1")]
    pub delegations: ::prost::alloc::vec::Vec<Delegate>,
    #[prost(message, repeated, tag = "2")]
    pub undelegations: ::prost::alloc::vec::Vec<Undelegate>,
}
/// Track's a validator's uptime.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Uptime {
    #[prost(uint64, tag = "1")]
    pub as_of_block_height: u64,
    #[prost(uint32, tag = "2")]
    pub window_len: u32,
    #[prost(bytes = "vec", tag = "3")]
    pub bitvec: ::prost::alloc::vec::Vec<u8>,
}
/// Tracks our view of Tendermint's view of the validator set, so we can keep it
/// from getting confused.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CurrentConsensusKeys {
    #[prost(message, repeated, tag = "1")]
    pub consensus_keys: ::prost::alloc::vec::Vec<
        super::super::super::keys::v1alpha1::ConsensusKey,
    >,
}
/// Tracks slashing penalties applied to a validator in some epoch.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Penalty {
    #[prost(uint64, tag = "1")]
    pub inner: u64,
}
