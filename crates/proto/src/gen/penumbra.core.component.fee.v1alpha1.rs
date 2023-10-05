/// Specifies fees paid by a transaction.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Fee {
    /// The amount of the token used to pay fees.
    #[prost(message, optional, tag = "1")]
    pub amount: ::core::option::Option<super::super::super::num::v1alpha1::Amount>,
    /// If present, the asset ID of the token used to pay fees.
    /// If absent, specifies the staking token implicitly.
    #[prost(message, optional, tag = "2")]
    pub asset_id: ::core::option::Option<super::super::super::asset::v1alpha1::AssetId>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GasPrices {
    /// The price per unit block space in terms of the staking token, with an implicit 1,000 denominator.
    #[prost(uint64, tag = "1")]
    pub block_space_price: u64,
    /// The price per unit compact block space in terms of the staking token, with an implicit 1,000 denominator.
    #[prost(uint64, tag = "2")]
    pub compact_block_space_price: u64,
    /// The price per unit verification cost in terms of the staking token, with an implicit 1,000 denominator.
    #[prost(uint64, tag = "3")]
    pub verification_price: u64,
    /// The price per unit execution cost in terms of the staking token, with an implicit 1,000 denominator.
    #[prost(uint64, tag = "4")]
    pub execution_price: u64,
}
/// Fee component configuration data.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct FeeParameters {}
/// Fee-specific genesis content.
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GenesisContent {
    /// The FeeParameters present at genesis.
    #[prost(message, optional, tag = "1")]
    pub fee_params: ::core::option::Option<FeeParameters>,
    /// The initial gas prices.
    #[prost(message, optional, tag = "2")]
    pub gas_prices: ::core::option::Option<GasPrices>,
}
