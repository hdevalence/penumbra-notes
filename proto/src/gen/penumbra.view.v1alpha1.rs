#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TransactionsRequest {
    /// If present, return only transactions after this height.
    #[prost(uint64, optional, tag="1")]
    pub start_height: ::core::option::Option<u64>,
    /// If present, return only transactions before this height.
    #[prost(uint64, optional, tag="2")]
    pub end_height: ::core::option::Option<u64>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TransactionHashStreamResponse {
    #[prost(uint64, tag="1")]
    pub block_height: u64,
    #[prost(bytes="vec", tag="2")]
    pub tx_hash: ::prost::alloc::vec::Vec<u8>,
}
/// A streaming full transaction response
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TransactionStreamResponse {
    #[prost(uint64, tag="1")]
    pub block_height: u64,
    #[prost(bytes="vec", tag="2")]
    pub tx_hash: ::prost::alloc::vec::Vec<u8>,
    #[prost(message, optional, tag="3")]
    pub tx: ::core::option::Option<super::super::core::transaction::v1alpha1::Transaction>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct NoteByCommitmentRequest {
    #[prost(message, optional, tag="1")]
    pub account_id: ::core::option::Option<super::super::core::crypto::v1alpha1::AccountId>,
    #[prost(message, optional, tag="2")]
    pub note_commitment: ::core::option::Option<super::super::core::crypto::v1alpha1::NoteCommitment>,
    /// If set to true, waits to return until the requested note is detected.
    #[prost(bool, tag="3")]
    pub await_detection: bool,
}
/// Requests the current chain parameters from the view service.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ChainParamsRequest {
}
/// Requests the current FMD parameters from the view service.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct FmdParametersRequest {
}
/// Requests all assets known to the view service.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct AssetRequest {
}
/// Requests sync status of the view service.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct StatusRequest {
    /// Identifies the FVK for the notes to query.
    #[prost(message, optional, tag="1")]
    pub account_id: ::core::option::Option<super::super::core::crypto::v1alpha1::AccountId>,
}
/// Returns the status of the view service and whether it is synchronized with the chain state.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct StatusResponse {
    /// The height the view service has synchronized to so far
    #[prost(uint64, tag="1")]
    pub sync_height: u64,
    /// Whether the view service is catching up with the chain state
    #[prost(bool, tag="2")]
    pub catching_up: bool,
}
/// Requests streaming updates on the sync height until the view service is synchronized.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct StatusStreamRequest {
    /// Identifies the FVK for the notes to query.
    #[prost(message, optional, tag="1")]
    pub account_id: ::core::option::Option<super::super::core::crypto::v1alpha1::AccountId>,
}
/// A streaming sync status update
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct StatusStreamResponse {
    #[prost(uint64, tag="1")]
    pub latest_known_block_height: u64,
    #[prost(uint64, tag="2")]
    pub sync_height: u64,
}
/// A note plaintext with associated metadata about its status.
#[derive(::serde::Deserialize, ::serde::Serialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SpendableNoteRecord {
    /// The note commitment, identifying the note.
    #[prost(message, optional, tag="1")]
    pub note_commitment: ::core::option::Option<super::super::core::crypto::v1alpha1::NoteCommitment>,
    /// The note plaintext itself.
    #[prost(message, optional, tag="2")]
    pub note: ::core::option::Option<super::super::core::crypto::v1alpha1::Note>,
    /// A precomputed decryption of the note's address incore.dex.v1alpha1.
    #[prost(message, optional, tag="3")]
    pub address_index: ::core::option::Option<super::super::core::crypto::v1alpha1::AddressIndex>,
    /// The note's nullifier.
    #[prost(message, optional, tag="4")]
    pub nullifier: ::core::option::Option<super::super::core::crypto::v1alpha1::Nullifier>,
    /// The height at which the note was created.
    #[prost(uint64, tag="5")]
    pub height_created: u64,
    /// Records whether the note was spent (and if so, at what height).
    #[prost(uint64, optional, tag="6")]
    pub height_spent: ::core::option::Option<u64>,
    /// The note position.
    #[prost(uint64, tag="7")]
    pub position: u64,
    /// The source of the note (a tx hash or otherwise)
    #[prost(message, optional, tag="8")]
    pub source: ::core::option::Option<super::super::core::chain::v1alpha1::NoteSource>,
}
/// A query for notes known by the view service.
///
/// This message uses the fact that all proto fields are optional
/// to allow various filtering on the returned notes.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct NotesRequest {
    /// Identifies the FVK for the notes to query.
    #[prost(message, optional, tag="1")]
    pub account_id: ::core::option::Option<super::super::core::crypto::v1alpha1::AccountId>,
    /// If set, return spent notes as well as unspent notes.
    #[prost(bool, tag="2")]
    pub include_spent: bool,
    /// If set, only return notes with the specified asset id.
    #[prost(message, optional, tag="3")]
    pub asset_id: ::core::option::Option<super::super::core::crypto::v1alpha1::AssetId>,
    /// If set, only return notes with the specified address incore.dex.v1alpha1.
    #[prost(message, optional, tag="4")]
    pub address_index: ::core::option::Option<super::super::core::crypto::v1alpha1::AddressIndex>,
    /// If set, stop returning notes once the total exceeds this amount.
    ///
    /// Ignored if `asset_id` is unset or if `include_spent` is set.
    #[prost(uint64, tag="5")]
    pub amount_to_spend: u64,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WitnessRequest {
    /// Identifies the FVK for the note commitments to query.
    #[prost(message, optional, tag="1")]
    pub account_id: ::core::option::Option<super::super::core::crypto::v1alpha1::AccountId>,
    /// The note commitments to obtain auth paths for.
    #[prost(message, repeated, tag="2")]
    pub note_commitments: ::prost::alloc::vec::Vec<super::super::core::crypto::v1alpha1::NoteCommitment>,
}
/// The plaintext of a note that has been quarantined until the end of an unbonding period.
#[derive(::serde::Deserialize, ::serde::Serialize)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct QuarantinedNoteRecord {
    /// The note commitment, identifying the note.
    #[prost(message, optional, tag="1")]
    pub note_commitment: ::core::option::Option<super::super::core::crypto::v1alpha1::NoteCommitment>,
    /// The note plaintext itself.
    #[prost(message, optional, tag="2")]
    pub note: ::core::option::Option<super::super::core::crypto::v1alpha1::Note>,
    /// A precomputed decryption of the note's address incore.dex.v1alpha1.
    #[prost(message, optional, tag="3")]
    pub address_index: ::core::option::Option<super::super::core::crypto::v1alpha1::AddressIndex>,
    /// The height at which the note was created.
    #[prost(uint64, tag="4")]
    pub height_created: u64,
    /// The epoch at which the note will exit quarantine, if unbonding is not interrupted by slashing.
    #[prost(uint64, tag="5")]
    pub unbonding_epoch: u64,
    /// The validator identity the quarantining is bound to.
    #[prost(message, optional, tag="6")]
    pub identity_key: ::core::option::Option<super::super::core::crypto::v1alpha1::IdentityKey>,
    /// The source of the note (a tx hash or otherwise)
    #[prost(message, optional, tag="7")]
    pub source: ::core::option::Option<super::super::core::chain::v1alpha1::NoteSource>,
}
/// A query for quarantined notes known by the view service.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct QuarantinedNotesRequest {
    /// Identifies the FVK for the notes to query.
    #[prost(message, optional, tag="1")]
    pub account_id: ::core::option::Option<super::super::core::crypto::v1alpha1::AccountId>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct NullifierStatusRequest {
    #[prost(message, optional, tag="1")]
    pub account_id: ::core::option::Option<super::super::core::crypto::v1alpha1::AccountId>,
    #[prost(message, optional, tag="2")]
    pub nullifier: ::core::option::Option<super::super::core::crypto::v1alpha1::Nullifier>,
    #[prost(bool, tag="3")]
    pub await_detection: bool,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct NullifierStatusResponse {
    #[prost(bool, tag="1")]
    pub spent: bool,
}
/// Generated client implementations.
pub mod view_protocol_client {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    use tonic::codegen::http::Uri;
    /// The view protocol is used by a view client, who wants to do some
    /// transaction-related actions, to request data from a view service, which is
    /// responsible for synchronizing and scanning the public chain state with one or
    /// more full viewing keys.
    ///
    /// View protocol requests include a hash of the full viewing key, used to
    /// identify which set of data to query.  This also works as a pseudo-auth system
    /// (assuming transport security, the client has to know the FVK to request its
    /// data).  (TODO: refine this)
    #[derive(Debug, Clone)]
    pub struct ViewProtocolClient<T> {
        inner: tonic::client::Grpc<T>,
    }
    impl ViewProtocolClient<tonic::transport::Channel> {
        /// Attempt to create a new client by connecting to a given endpoint.
        pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
        where
            D: std::convert::TryInto<tonic::transport::Endpoint>,
            D::Error: Into<StdError>,
        {
            let conn = tonic::transport::Endpoint::new(dst)?.connect().await?;
            Ok(Self::new(conn))
        }
    }
    impl<T> ViewProtocolClient<T>
    where
        T: tonic::client::GrpcService<tonic::body::BoxBody>,
        T::Error: Into<StdError>,
        T::ResponseBody: Body<Data = Bytes> + Send + 'static,
        <T::ResponseBody as Body>::Error: Into<StdError> + Send,
    {
        pub fn new(inner: T) -> Self {
            let inner = tonic::client::Grpc::new(inner);
            Self { inner }
        }
        pub fn with_origin(inner: T, origin: Uri) -> Self {
            let inner = tonic::client::Grpc::with_origin(inner, origin);
            Self { inner }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> ViewProtocolClient<InterceptedService<T, F>>
        where
            F: tonic::service::Interceptor,
            T::ResponseBody: Default,
            T: tonic::codegen::Service<
                http::Request<tonic::body::BoxBody>,
                Response = http::Response<
                    <T as tonic::client::GrpcService<tonic::body::BoxBody>>::ResponseBody,
                >,
            >,
            <T as tonic::codegen::Service<
                http::Request<tonic::body::BoxBody>,
            >>::Error: Into<StdError> + Send + Sync,
        {
            ViewProtocolClient::new(InterceptedService::new(inner, interceptor))
        }
        /// Compress requests with the given encoding.
        ///
        /// This requires the server to support it otherwise it might respond with an
        /// error.
        #[must_use]
        pub fn send_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.inner = self.inner.send_compressed(encoding);
            self
        }
        /// Enable decompressing responses.
        #[must_use]
        pub fn accept_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.inner = self.inner.accept_compressed(encoding);
            self
        }
        /// Get current status of chain sync
        pub async fn status(
            &mut self,
            request: impl tonic::IntoRequest<super::StatusRequest>,
        ) -> Result<tonic::Response<super::StatusResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/penumbra.view.v1alpha1.ViewProtocol/Status",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// Stream sync status updates until the view service has caught up with the core.chain.v1alpha1.
        pub async fn status_stream(
            &mut self,
            request: impl tonic::IntoRequest<super::StatusStreamRequest>,
        ) -> Result<
            tonic::Response<tonic::codec::Streaming<super::StatusStreamResponse>>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/penumbra.view.v1alpha1.ViewProtocol/StatusStream",
            );
            self.inner.server_streaming(request.into_request(), path, codec).await
        }
        /// Queries for notes that have been accepted by the core.chain.v1alpha1.
        pub async fn notes(
            &mut self,
            request: impl tonic::IntoRequest<super::NotesRequest>,
        ) -> Result<
            tonic::Response<tonic::codec::Streaming<super::SpendableNoteRecord>>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/penumbra.view.v1alpha1.ViewProtocol/Notes",
            );
            self.inner.server_streaming(request.into_request(), path, codec).await
        }
        /// Queries for notes that have been quarantined until the end of an unbonding period.
        pub async fn quarantined_notes(
            &mut self,
            request: impl tonic::IntoRequest<super::QuarantinedNotesRequest>,
        ) -> Result<
            tonic::Response<tonic::codec::Streaming<super::QuarantinedNoteRecord>>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/penumbra.view.v1alpha1.ViewProtocol/QuarantinedNotes",
            );
            self.inner.server_streaming(request.into_request(), path, codec).await
        }
        /// Returns authentication paths for the given note commitments.
        ///
        /// This method takes a batch of input commitments, rather than just one, so
        /// that the client can get a consistent set of authentication paths to a
        /// common root.  (Otherwise, if a client made multiple requests, the wallet
        /// service could have advanced the note commitment tree state between queries).
        pub async fn witness(
            &mut self,
            request: impl tonic::IntoRequest<super::WitnessRequest>,
        ) -> Result<
            tonic::Response<
                super::super::super::core::transaction::v1alpha1::WitnessData,
            >,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/penumbra.view.v1alpha1.ViewProtocol/Witness",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// Queries for assets.
        pub async fn assets(
            &mut self,
            request: impl tonic::IntoRequest<super::AssetRequest>,
        ) -> Result<
            tonic::Response<
                tonic::codec::Streaming<
                    super::super::super::core::crypto::v1alpha1::Asset,
                >,
            >,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/penumbra.view.v1alpha1.ViewProtocol/Assets",
            );
            self.inner.server_streaming(request.into_request(), path, codec).await
        }
        /// Query for the current chain parameters.
        pub async fn chain_parameters(
            &mut self,
            request: impl tonic::IntoRequest<super::ChainParamsRequest>,
        ) -> Result<
            tonic::Response<super::super::super::core::chain::v1alpha1::ChainParameters>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/penumbra.view.v1alpha1.ViewProtocol/ChainParameters",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// Query for the current FMD parameters.
        pub async fn fmd_parameters(
            &mut self,
            request: impl tonic::IntoRequest<super::FmdParametersRequest>,
        ) -> Result<
            tonic::Response<super::super::super::core::chain::v1alpha1::FmdParameters>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/penumbra.view.v1alpha1.ViewProtocol/FMDParameters",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// Query for a note by its note commitment, optionally waiting until the note is detected.
        pub async fn note_by_commitment(
            &mut self,
            request: impl tonic::IntoRequest<super::NoteByCommitmentRequest>,
        ) -> Result<tonic::Response<super::SpendableNoteRecord>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/penumbra.view.v1alpha1.ViewProtocol/NoteByCommitment",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// Query for whether a nullifier has been spent, optionally waiting until it is spent.
        pub async fn nullifier_status(
            &mut self,
            request: impl tonic::IntoRequest<super::NullifierStatusRequest>,
        ) -> Result<tonic::Response<super::NullifierStatusResponse>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/penumbra.view.v1alpha1.ViewProtocol/NullifierStatus",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        /// Query for the transaction hashes in the given range of blocks.
        pub async fn transaction_hashes(
            &mut self,
            request: impl tonic::IntoRequest<super::TransactionsRequest>,
        ) -> Result<
            tonic::Response<
                tonic::codec::Streaming<super::TransactionHashStreamResponse>,
            >,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/penumbra.view.v1alpha1.ViewProtocol/TransactionHashes",
            );
            self.inner.server_streaming(request.into_request(), path, codec).await
        }
        /// Query for the full transactions in the given range of blocks.
        pub async fn transactions(
            &mut self,
            request: impl tonic::IntoRequest<super::TransactionsRequest>,
        ) -> Result<
            tonic::Response<tonic::codec::Streaming<super::TransactionStreamResponse>>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/penumbra.view.v1alpha1.ViewProtocol/Transactions",
            );
            self.inner.server_streaming(request.into_request(), path, codec).await
        }
    }
}
/// Generated server implementations.
pub mod view_protocol_server {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    ///Generated trait containing gRPC methods that should be implemented for use with ViewProtocolServer.
    #[async_trait]
    pub trait ViewProtocol: Send + Sync + 'static {
        /// Get current status of chain sync
        async fn status(
            &self,
            request: tonic::Request<super::StatusRequest>,
        ) -> Result<tonic::Response<super::StatusResponse>, tonic::Status>;
        ///Server streaming response type for the StatusStream method.
        type StatusStreamStream: futures_core::Stream<
                Item = Result<super::StatusStreamResponse, tonic::Status>,
            >
            + Send
            + 'static;
        /// Stream sync status updates until the view service has caught up with the core.chain.v1alpha1.
        async fn status_stream(
            &self,
            request: tonic::Request<super::StatusStreamRequest>,
        ) -> Result<tonic::Response<Self::StatusStreamStream>, tonic::Status>;
        ///Server streaming response type for the Notes method.
        type NotesStream: futures_core::Stream<
                Item = Result<super::SpendableNoteRecord, tonic::Status>,
            >
            + Send
            + 'static;
        /// Queries for notes that have been accepted by the core.chain.v1alpha1.
        async fn notes(
            &self,
            request: tonic::Request<super::NotesRequest>,
        ) -> Result<tonic::Response<Self::NotesStream>, tonic::Status>;
        ///Server streaming response type for the QuarantinedNotes method.
        type QuarantinedNotesStream: futures_core::Stream<
                Item = Result<super::QuarantinedNoteRecord, tonic::Status>,
            >
            + Send
            + 'static;
        /// Queries for notes that have been quarantined until the end of an unbonding period.
        async fn quarantined_notes(
            &self,
            request: tonic::Request<super::QuarantinedNotesRequest>,
        ) -> Result<tonic::Response<Self::QuarantinedNotesStream>, tonic::Status>;
        /// Returns authentication paths for the given note commitments.
        ///
        /// This method takes a batch of input commitments, rather than just one, so
        /// that the client can get a consistent set of authentication paths to a
        /// common root.  (Otherwise, if a client made multiple requests, the wallet
        /// service could have advanced the note commitment tree state between queries).
        async fn witness(
            &self,
            request: tonic::Request<super::WitnessRequest>,
        ) -> Result<
            tonic::Response<
                super::super::super::core::transaction::v1alpha1::WitnessData,
            >,
            tonic::Status,
        >;
        ///Server streaming response type for the Assets method.
        type AssetsStream: futures_core::Stream<
                Item = Result<
                    super::super::super::core::crypto::v1alpha1::Asset,
                    tonic::Status,
                >,
            >
            + Send
            + 'static;
        /// Queries for assets.
        async fn assets(
            &self,
            request: tonic::Request<super::AssetRequest>,
        ) -> Result<tonic::Response<Self::AssetsStream>, tonic::Status>;
        /// Query for the current chain parameters.
        async fn chain_parameters(
            &self,
            request: tonic::Request<super::ChainParamsRequest>,
        ) -> Result<
            tonic::Response<super::super::super::core::chain::v1alpha1::ChainParameters>,
            tonic::Status,
        >;
        /// Query for the current FMD parameters.
        async fn fmd_parameters(
            &self,
            request: tonic::Request<super::FmdParametersRequest>,
        ) -> Result<
            tonic::Response<super::super::super::core::chain::v1alpha1::FmdParameters>,
            tonic::Status,
        >;
        /// Query for a note by its note commitment, optionally waiting until the note is detected.
        async fn note_by_commitment(
            &self,
            request: tonic::Request<super::NoteByCommitmentRequest>,
        ) -> Result<tonic::Response<super::SpendableNoteRecord>, tonic::Status>;
        /// Query for whether a nullifier has been spent, optionally waiting until it is spent.
        async fn nullifier_status(
            &self,
            request: tonic::Request<super::NullifierStatusRequest>,
        ) -> Result<tonic::Response<super::NullifierStatusResponse>, tonic::Status>;
        ///Server streaming response type for the TransactionHashes method.
        type TransactionHashesStream: futures_core::Stream<
                Item = Result<super::TransactionHashStreamResponse, tonic::Status>,
            >
            + Send
            + 'static;
        /// Query for the transaction hashes in the given range of blocks.
        async fn transaction_hashes(
            &self,
            request: tonic::Request<super::TransactionsRequest>,
        ) -> Result<tonic::Response<Self::TransactionHashesStream>, tonic::Status>;
        ///Server streaming response type for the Transactions method.
        type TransactionsStream: futures_core::Stream<
                Item = Result<super::TransactionStreamResponse, tonic::Status>,
            >
            + Send
            + 'static;
        /// Query for the full transactions in the given range of blocks.
        async fn transactions(
            &self,
            request: tonic::Request<super::TransactionsRequest>,
        ) -> Result<tonic::Response<Self::TransactionsStream>, tonic::Status>;
    }
    /// The view protocol is used by a view client, who wants to do some
    /// transaction-related actions, to request data from a view service, which is
    /// responsible for synchronizing and scanning the public chain state with one or
    /// more full viewing keys.
    ///
    /// View protocol requests include a hash of the full viewing key, used to
    /// identify which set of data to query.  This also works as a pseudo-auth system
    /// (assuming transport security, the client has to know the FVK to request its
    /// data).  (TODO: refine this)
    #[derive(Debug)]
    pub struct ViewProtocolServer<T: ViewProtocol> {
        inner: _Inner<T>,
        accept_compression_encodings: EnabledCompressionEncodings,
        send_compression_encodings: EnabledCompressionEncodings,
    }
    struct _Inner<T>(Arc<T>);
    impl<T: ViewProtocol> ViewProtocolServer<T> {
        pub fn new(inner: T) -> Self {
            Self::from_arc(Arc::new(inner))
        }
        pub fn from_arc(inner: Arc<T>) -> Self {
            let inner = _Inner(inner);
            Self {
                inner,
                accept_compression_encodings: Default::default(),
                send_compression_encodings: Default::default(),
            }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> InterceptedService<Self, F>
        where
            F: tonic::service::Interceptor,
        {
            InterceptedService::new(Self::new(inner), interceptor)
        }
        /// Enable decompressing requests with the given encoding.
        #[must_use]
        pub fn accept_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.accept_compression_encodings.enable(encoding);
            self
        }
        /// Compress responses with the given encoding, if the client supports it.
        #[must_use]
        pub fn send_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.send_compression_encodings.enable(encoding);
            self
        }
    }
    impl<T, B> tonic::codegen::Service<http::Request<B>> for ViewProtocolServer<T>
    where
        T: ViewProtocol,
        B: Body + Send + 'static,
        B::Error: Into<StdError> + Send + 'static,
    {
        type Response = http::Response<tonic::body::BoxBody>;
        type Error = std::convert::Infallible;
        type Future = BoxFuture<Self::Response, Self::Error>;
        fn poll_ready(
            &mut self,
            _cx: &mut Context<'_>,
        ) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
        fn call(&mut self, req: http::Request<B>) -> Self::Future {
            let inner = self.inner.clone();
            match req.uri().path() {
                "/penumbra.view.v1alpha1.ViewProtocol/Status" => {
                    #[allow(non_camel_case_types)]
                    struct StatusSvc<T: ViewProtocol>(pub Arc<T>);
                    impl<
                        T: ViewProtocol,
                    > tonic::server::UnaryService<super::StatusRequest>
                    for StatusSvc<T> {
                        type Response = super::StatusResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::StatusRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).status(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = StatusSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/penumbra.view.v1alpha1.ViewProtocol/StatusStream" => {
                    #[allow(non_camel_case_types)]
                    struct StatusStreamSvc<T: ViewProtocol>(pub Arc<T>);
                    impl<
                        T: ViewProtocol,
                    > tonic::server::ServerStreamingService<super::StatusStreamRequest>
                    for StatusStreamSvc<T> {
                        type Response = super::StatusStreamResponse;
                        type ResponseStream = T::StatusStreamStream;
                        type Future = BoxFuture<
                            tonic::Response<Self::ResponseStream>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::StatusStreamRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).status_stream(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = StatusStreamSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.server_streaming(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/penumbra.view.v1alpha1.ViewProtocol/Notes" => {
                    #[allow(non_camel_case_types)]
                    struct NotesSvc<T: ViewProtocol>(pub Arc<T>);
                    impl<
                        T: ViewProtocol,
                    > tonic::server::ServerStreamingService<super::NotesRequest>
                    for NotesSvc<T> {
                        type Response = super::SpendableNoteRecord;
                        type ResponseStream = T::NotesStream;
                        type Future = BoxFuture<
                            tonic::Response<Self::ResponseStream>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::NotesRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).notes(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = NotesSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.server_streaming(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/penumbra.view.v1alpha1.ViewProtocol/QuarantinedNotes" => {
                    #[allow(non_camel_case_types)]
                    struct QuarantinedNotesSvc<T: ViewProtocol>(pub Arc<T>);
                    impl<
                        T: ViewProtocol,
                    > tonic::server::ServerStreamingService<
                        super::QuarantinedNotesRequest,
                    > for QuarantinedNotesSvc<T> {
                        type Response = super::QuarantinedNoteRecord;
                        type ResponseStream = T::QuarantinedNotesStream;
                        type Future = BoxFuture<
                            tonic::Response<Self::ResponseStream>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::QuarantinedNotesRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).quarantined_notes(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = QuarantinedNotesSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.server_streaming(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/penumbra.view.v1alpha1.ViewProtocol/Witness" => {
                    #[allow(non_camel_case_types)]
                    struct WitnessSvc<T: ViewProtocol>(pub Arc<T>);
                    impl<
                        T: ViewProtocol,
                    > tonic::server::UnaryService<super::WitnessRequest>
                    for WitnessSvc<T> {
                        type Response = super::super::super::core::transaction::v1alpha1::WitnessData;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::WitnessRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).witness(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = WitnessSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/penumbra.view.v1alpha1.ViewProtocol/Assets" => {
                    #[allow(non_camel_case_types)]
                    struct AssetsSvc<T: ViewProtocol>(pub Arc<T>);
                    impl<
                        T: ViewProtocol,
                    > tonic::server::ServerStreamingService<super::AssetRequest>
                    for AssetsSvc<T> {
                        type Response = super::super::super::core::crypto::v1alpha1::Asset;
                        type ResponseStream = T::AssetsStream;
                        type Future = BoxFuture<
                            tonic::Response<Self::ResponseStream>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::AssetRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).assets(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = AssetsSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.server_streaming(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/penumbra.view.v1alpha1.ViewProtocol/ChainParameters" => {
                    #[allow(non_camel_case_types)]
                    struct ChainParametersSvc<T: ViewProtocol>(pub Arc<T>);
                    impl<
                        T: ViewProtocol,
                    > tonic::server::UnaryService<super::ChainParamsRequest>
                    for ChainParametersSvc<T> {
                        type Response = super::super::super::core::chain::v1alpha1::ChainParameters;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::ChainParamsRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).chain_parameters(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = ChainParametersSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/penumbra.view.v1alpha1.ViewProtocol/FMDParameters" => {
                    #[allow(non_camel_case_types)]
                    struct FMDParametersSvc<T: ViewProtocol>(pub Arc<T>);
                    impl<
                        T: ViewProtocol,
                    > tonic::server::UnaryService<super::FmdParametersRequest>
                    for FMDParametersSvc<T> {
                        type Response = super::super::super::core::chain::v1alpha1::FmdParameters;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::FmdParametersRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).fmd_parameters(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = FMDParametersSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/penumbra.view.v1alpha1.ViewProtocol/NoteByCommitment" => {
                    #[allow(non_camel_case_types)]
                    struct NoteByCommitmentSvc<T: ViewProtocol>(pub Arc<T>);
                    impl<
                        T: ViewProtocol,
                    > tonic::server::UnaryService<super::NoteByCommitmentRequest>
                    for NoteByCommitmentSvc<T> {
                        type Response = super::SpendableNoteRecord;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::NoteByCommitmentRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).note_by_commitment(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = NoteByCommitmentSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/penumbra.view.v1alpha1.ViewProtocol/NullifierStatus" => {
                    #[allow(non_camel_case_types)]
                    struct NullifierStatusSvc<T: ViewProtocol>(pub Arc<T>);
                    impl<
                        T: ViewProtocol,
                    > tonic::server::UnaryService<super::NullifierStatusRequest>
                    for NullifierStatusSvc<T> {
                        type Response = super::NullifierStatusResponse;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::NullifierStatusRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).nullifier_status(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = NullifierStatusSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/penumbra.view.v1alpha1.ViewProtocol/TransactionHashes" => {
                    #[allow(non_camel_case_types)]
                    struct TransactionHashesSvc<T: ViewProtocol>(pub Arc<T>);
                    impl<
                        T: ViewProtocol,
                    > tonic::server::ServerStreamingService<super::TransactionsRequest>
                    for TransactionHashesSvc<T> {
                        type Response = super::TransactionHashStreamResponse;
                        type ResponseStream = T::TransactionHashesStream;
                        type Future = BoxFuture<
                            tonic::Response<Self::ResponseStream>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::TransactionsRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).transaction_hashes(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = TransactionHashesSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.server_streaming(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/penumbra.view.v1alpha1.ViewProtocol/Transactions" => {
                    #[allow(non_camel_case_types)]
                    struct TransactionsSvc<T: ViewProtocol>(pub Arc<T>);
                    impl<
                        T: ViewProtocol,
                    > tonic::server::ServerStreamingService<super::TransactionsRequest>
                    for TransactionsSvc<T> {
                        type Response = super::TransactionStreamResponse;
                        type ResponseStream = T::TransactionsStream;
                        type Future = BoxFuture<
                            tonic::Response<Self::ResponseStream>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::TransactionsRequest>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move {
                                (*inner).transactions(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = TransactionsSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            );
                        let res = grpc.server_streaming(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                _ => {
                    Box::pin(async move {
                        Ok(
                            http::Response::builder()
                                .status(200)
                                .header("grpc-status", "12")
                                .header("content-type", "application/grpc")
                                .body(empty_body())
                                .unwrap(),
                        )
                    })
                }
            }
        }
    }
    impl<T: ViewProtocol> Clone for ViewProtocolServer<T> {
        fn clone(&self) -> Self {
            let inner = self.inner.clone();
            Self {
                inner,
                accept_compression_encodings: self.accept_compression_encodings,
                send_compression_encodings: self.send_compression_encodings,
            }
        }
    }
    impl<T: ViewProtocol> Clone for _Inner<T> {
        fn clone(&self) -> Self {
            Self(self.0.clone())
        }
    }
    impl<T: std::fmt::Debug> std::fmt::Debug for _Inner<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self.0)
        }
    }
    impl<T: ViewProtocol> tonic::server::NamedService for ViewProtocolServer<T> {
        const NAME: &'static str = "penumbra.view.v1alpha1.ViewProtocol";
    }
}
