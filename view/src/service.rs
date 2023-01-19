use std::{
    collections::BTreeMap,
    pin::Pin,
    sync::{Arc, Mutex},
};

use anyhow::anyhow;
use async_stream::try_stream;
use camino::Utf8Path;
use futures::stream::{StreamExt, TryStreamExt};
use penumbra_crypto::{
    asset,
    keys::{AccountID, AddressIndex, FullViewingKey},
    Amount,
};
use penumbra_proto::{
    client::v1alpha1::{
        tendermint_proxy_service_client::TendermintProxyServiceClient, GetStatusRequest,
    },
    core::crypto::v1alpha1 as pbc,
    view::v1alpha1::{
        self as pb, view_protocol_service_server::ViewProtocolService, ChainParametersResponse,
        FmdParametersResponse, NoteByCommitmentResponse, StatusResponse, SwapByCommitmentResponse,
        TransactionHashesResponse, TransactionsResponse, WitnessResponse,
    },
};
use penumbra_tct::{Commitment, Proof};
use penumbra_transaction::{plan::TransactionPlan, TransactionPerspective, WitnessData};
use rand_core::OsRng;
use tokio::sync::{watch, RwLock};
use tokio_stream::wrappers::WatchStream;
use tonic::async_trait;
use tracing::instrument;

use crate::{Storage, Worker};

/// A service that synchronizes private chain state and responds to queries
/// about it.
///
/// The [`ViewService`] implements the Tonic-derived [`ViewProtocol`] trait,
/// so it can be used as a gRPC server, or called directly.  It spawns a task
/// internally that performs synchronization and scanning.  The
/// [`ViewService`] can be cloned; each clone will read from the same shared
/// state, but there will only be a single scanning task.
#[derive(Clone)]
pub struct ViewService {
    storage: Storage,
    // A shared error slot for errors bubbled up by the worker. This is a regular Mutex
    // rather than a Tokio Mutex because it should be uncontended.
    error_slot: Arc<Mutex<Option<anyhow::Error>>>,
    account_id: AccountID,
    // A copy of the NCT used by the worker task.
    note_commitment_tree: Arc<RwLock<penumbra_tct::Tree>>,
    // The address of the pd+tendermint node.
    node: String,
    /// The port to talk to tendermint on.
    pd_port: u16,
    /// Used to watch for changes to the sync height.
    sync_height_rx: watch::Receiver<u64>,
}

impl ViewService {
    /// Convenience method that calls [`Storage::load_or_initialize`] and then [`Self::new`].
    pub async fn load_or_initialize(
        storage_path: impl AsRef<Utf8Path>,
        fvk: &FullViewingKey,
        node: String,
        pd_port: u16,
    ) -> anyhow::Result<Self> {
        let storage = Storage::load_or_initialize(storage_path, fvk, node.clone(), pd_port).await?;

        Self::new(storage, node, pd_port).await
    }

    /// Constructs a new [`ViewService`], spawning a sync task internally.
    ///
    /// The sync task uses the provided `client` to sync with the chain.
    ///
    /// To create multiple [`ViewService`]s, clone the [`ViewService`] returned
    /// by this method, rather than calling it multiple times.  That way, each clone
    /// will be backed by the same scanning task, rather than each spawning its own.
    pub async fn new(storage: Storage, node: String, pd_port: u16) -> Result<Self, anyhow::Error> {
        let (worker, nct, error_slot, sync_height_rx) =
            Worker::new(storage.clone(), node.clone(), pd_port).await?;

        tokio::spawn(worker.run());

        let fvk = storage.full_viewing_key().await?;
        let account_id = fvk.hash();

        Ok(Self {
            storage,
            account_id,
            error_slot,
            sync_height_rx,
            note_commitment_tree: nct,
            node,
            pd_port,
        })
    }

    async fn check_fvk(&self, fvk: Option<&pbc::AccountId>) -> Result<(), tonic::Status> {
        // Takes an Option to avoid making the caller handle missing fields,
        // should error on None or wrong account ID
        match fvk {
            Some(fvk) => {
                if fvk != &self.account_id.into() {
                    return Err(tonic::Status::new(
                        tonic::Code::InvalidArgument,
                        "Invalid account ID",
                    ));
                }

                Ok(())
            }
            None => Err(tonic::Status::new(
                tonic::Code::InvalidArgument,
                "Missing FVK",
            )),
        }
    }

    async fn check_worker(&self) -> Result<(), tonic::Status> {
        // If the shared error slot is set, then an error has occurred in the worker
        // that we should bubble up.
        if self.error_slot.lock().unwrap().is_some() {
            return Err(tonic::Status::new(
                tonic::Code::Internal,
                format!(
                    "Worker failed: {}",
                    self.error_slot.lock().unwrap().as_ref().unwrap()
                ),
            ));
        }

        // TODO: check whether the worker is still alive, else fail, when we have a way to do that
        // (if the worker is to crash without setting the error_slot, the service should die as well)

        Ok(())
    }

    /// Return the latest block height known by the fullnode or its peers, as
    /// well as whether the fullnode is caught up with that height.
    #[instrument(skip(self))]
    pub async fn latest_known_block_height(&self) -> Result<(u64, bool), anyhow::Error> {
        let mut client =
            TendermintProxyServiceClient::connect(format!("http://{}:{}", self.node, self.pd_port))
                .await?;

        let rsp = client.get_status(GetStatusRequest {}).await?.into_inner();

        //tracing::debug!("{:#?}", rsp);

        let sync_info = rsp
            .sync_info
            .ok_or_else(|| anyhow::anyhow!("could not parse sync_info in gRPC response"))?;

        let latest_block_height = sync_info.latest_block_height;

        let node_catching_up = sync_info.catching_up;

        // There is a `max_peer_block_height` available in TM 0.35, however it should not be used
        // as it does not seem to reflect the consensus height. Since clients use `latest_known_block_height`
        // to determine the height to attempt syncing to, a validator reporting a non-consensus height
        // can cause a DoS to clients attempting to sync if `max_peer_block_height` is used.
        let latest_known_block_height = latest_block_height;

        tracing::debug!(
            ?latest_block_height,
            ?node_catching_up,
            ?latest_known_block_height
        );

        Ok((latest_known_block_height, node_catching_up))
    }

    #[instrument(skip(self))]
    pub async fn status(&self) -> Result<StatusResponse, anyhow::Error> {
        let sync_height = self.storage.last_sync_height().await?.unwrap_or(0);

        let (latest_known_block_height, node_catching_up) =
            self.latest_known_block_height().await?;

        let height_diff = latest_known_block_height
            .checked_sub(sync_height)
            .ok_or_else(|| anyhow!("sync height ahead of node height"))?;

        let catching_up = match (node_catching_up, height_diff) {
            // We're synced to the same height as the node
            (false, 0) => false,
            // We're one block behind, and will learn about it soon, close enough
            (false, 1) => false,
            // We're behind the node
            (false, _) => true,
            // The node is behind the network
            (true, _) => true,
        };

        Ok(StatusResponse {
            sync_height,
            catching_up,
        })
    }
}

#[async_trait]
impl ViewProtocolService for ViewService {
    type NotesStream =
        Pin<Box<dyn futures::Stream<Item = Result<pb::NotesResponse, tonic::Status>> + Send>>;
    type AssetsStream =
        Pin<Box<dyn futures::Stream<Item = Result<pb::AssetsResponse, tonic::Status>> + Send>>;
    type StatusStreamStream = Pin<
        Box<dyn futures::Stream<Item = Result<pb::StatusStreamResponse, tonic::Status>> + Send>,
    >;
    type TransactionHashesStream = Pin<
        Box<
            dyn futures::Stream<Item = Result<pb::TransactionHashesResponse, tonic::Status>> + Send,
        >,
    >;
    type TransactionsStream = Pin<
        Box<dyn futures::Stream<Item = Result<pb::TransactionsResponse, tonic::Status>> + Send>,
    >;
    type BalanceByAddressStream = Pin<
        Box<dyn futures::Stream<Item = Result<pb::BalanceByAddressResponse, tonic::Status>> + Send>,
    >;

    async fn transaction_perspective(
        &self,
        request: tonic::Request<pb::TransactionPerspectiveRequest>,
    ) -> Result<tonic::Response<pb::TransactionPerspectiveResponse>, tonic::Status> {
        self.check_worker().await?;

        let request = request.into_inner();

        let fvk =
            self.storage.full_viewing_key().await.map_err(|_| {
                tonic::Status::failed_precondition("Error retrieving full viewing key")
            })?;

        let tx = self
            .storage
            .transaction_by_hash(&request.tx_hash)
            .await
            .map_err(|_| {
                tonic::Status::failed_precondition(format!(
                    "Error retrieving transaction by hash {}",
                    hex::encode(&request.tx_hash)
                ))
            })?
            .ok_or_else(|| {
                tonic::Status::failed_precondition(format!(
                    "No transaction found with this hash {}",
                    hex::encode(&request.tx_hash)
                ))
            })?;

        let payload_keys = tx
            .payload_keys(&fvk)
            .map_err(|_| tonic::Status::failed_precondition("Error generating payload keys"))?;

        let mut spend_nullifiers = BTreeMap::new();

        for action in tx.actions() {
            if let penumbra_transaction::Action::Spend(spend) = action {
                let nullifier = spend.body.nullifier;
                // An error here indicates we don't know the nullifier, so we omit it from the Perspective.
                if let Ok(spendable_note_record) =
                    self.storage.note_by_nullifier(nullifier, false).await
                {
                    spend_nullifiers.insert(nullifier, spendable_note_record.note);
                }
            }
        }

        // TODO: query for advice notes
        let advice_notes = Default::default();

        let txp = TransactionPerspective {
            payload_keys,
            spend_nullifiers,
            advice_notes,
        };

        let response = pb::TransactionPerspectiveResponse {
            txp: Some(txp.into()),
            tx: Some(tx.into()),
        };

        Ok(tonic::Response::new(response))
    }

    async fn swap_by_commitment(
        &self,
        request: tonic::Request<pb::SwapByCommitmentRequest>,
    ) -> Result<tonic::Response<pb::SwapByCommitmentResponse>, tonic::Status> {
        self.check_worker().await?;
        self.check_fvk(request.get_ref().account_id.as_ref())
            .await?;

        let request = request.into_inner();

        let swap_commitment = request
            .swap_commitment
            .ok_or_else(|| {
                tonic::Status::failed_precondition("Missing swap commitment in request")
            })?
            .try_into()
            .map_err(|_| {
                tonic::Status::failed_precondition("Invalid swap commitment in request")
            })?;

        let swap = pb::SwapRecord::from(
            self.storage
                .swap_by_commitment(swap_commitment, request.await_detection)
                .await
                .map_err(|e| tonic::Status::internal(format!("error: {}", e)))?,
        );

        Ok(tonic::Response::new(SwapByCommitmentResponse {
            swap: Some(swap),
        }))
    }

    async fn balance_by_address(
        &self,
        request: tonic::Request<pb::BalanceByAddressRequest>,
    ) -> Result<tonic::Response<Self::BalanceByAddressStream>, tonic::Status> {
        let address = request
            .into_inner()
            .address
            .ok_or_else(|| tonic::Status::failed_precondition("Missing address in request"))?
            .try_into()
            .map_err(|_| tonic::Status::failed_precondition("Invalid address in request"))?;

        let result = self
            .storage
            .balance_by_address(address)
            .await
            .map_err(|e| tonic::Status::internal(format!("error: {}", e)))?;

        let stream = try_stream! {
            for element in result {
                yield pb::BalanceByAddressResponse {
                    asset: Some(element.0.into()),
                    amount: Some(Amount { inner: element.1 }.into())

                }
            }
        };

        Ok(tonic::Response::new(
            stream
                .map_err(|e: anyhow::Error| {
                    tonic::Status::unavailable(format!("error getting balances: {}", e))
                })
                .boxed(),
        ))
    }

    async fn note_by_commitment(
        &self,
        request: tonic::Request<pb::NoteByCommitmentRequest>,
    ) -> Result<tonic::Response<pb::NoteByCommitmentResponse>, tonic::Status> {
        self.check_worker().await?;
        self.check_fvk(request.get_ref().account_id.as_ref())
            .await?;

        let request = request.into_inner();

        let note_commitment = request
            .note_commitment
            .ok_or_else(|| {
                tonic::Status::failed_precondition("Missing note commitment in request")
            })?
            .try_into()
            .map_err(|_| {
                tonic::Status::failed_precondition("Invalid note commitment in request")
            })?;

        let spendable_note = pb::SpendableNoteRecord::from(
            self.storage
                .note_by_commitment(note_commitment, request.await_detection)
                .await
                .map_err(|e| tonic::Status::internal(format!("error: {}", e)))?,
        );

        Ok(tonic::Response::new(NoteByCommitmentResponse {
            spendable_note: Some(spendable_note),
        }))
    }

    async fn nullifier_status(
        &self,
        request: tonic::Request<pb::NullifierStatusRequest>,
    ) -> Result<tonic::Response<pb::NullifierStatusResponse>, tonic::Status> {
        self.check_worker().await?;
        self.check_fvk(request.get_ref().account_id.as_ref())
            .await?;

        let request = request.into_inner();

        let nullifier = request
            .nullifier
            .ok_or_else(|| tonic::Status::failed_precondition("Missing nullifier in request"))?
            .try_into()
            .map_err(|_| tonic::Status::failed_precondition("Invalid nullifier in request"))?;

        Ok(tonic::Response::new(pb::NullifierStatusResponse {
            spent: self
                .storage
                .nullifier_status(nullifier, request.await_detection)
                .await
                .map_err(|e| tonic::Status::internal(format!("error: {}", e)))?,
        }))
    }

    async fn status(
        &self,
        request: tonic::Request<pb::StatusRequest>,
    ) -> Result<tonic::Response<pb::StatusResponse>, tonic::Status> {
        self.check_worker().await?;
        self.check_fvk(request.get_ref().account_id.as_ref())
            .await?;

        Ok(tonic::Response::new(self.status().await.map_err(|e| {
            tonic::Status::internal(format!("error: {}", e))
        })?))
    }

    async fn status_stream(
        &self,
        request: tonic::Request<pb::StatusStreamRequest>,
    ) -> Result<tonic::Response<Self::StatusStreamStream>, tonic::Status> {
        self.check_worker().await?;
        self.check_fvk(request.get_ref().account_id.as_ref())
            .await?;

        let (latest_known_block_height, _) =
            self.latest_known_block_height().await.map_err(|e| {
                tonic::Status::unknown(format!(
                    "unable to fetch latest known block height from fullnode: {}",
                    e
                ))
            })?;

        // Create a stream of sync height updates from our worker, and send them to the client
        // until we've reached the latest known block height at the time the request was made.
        let mut sync_height_stream = WatchStream::new(self.sync_height_rx.clone());
        let stream = try_stream! {
            while let Some(sync_height) = sync_height_stream.next().await {
                yield pb::StatusStreamResponse {
                    latest_known_block_height,
                    sync_height,
                };
                if sync_height >= latest_known_block_height {
                    break;
                }
            }
        };

        Ok(tonic::Response::new(stream.boxed()))
    }

    async fn notes(
        &self,
        request: tonic::Request<pb::NotesRequest>,
    ) -> Result<tonic::Response<Self::NotesStream>, tonic::Status> {
        self.check_worker().await?;
        self.check_fvk(request.get_ref().account_id.as_ref())
            .await?;

        let include_spent = request.get_ref().include_spent;
        let asset_id = request
            .get_ref()
            .asset_id
            .to_owned()
            .map(asset::Id::try_from)
            .map_or(Ok(None), |v| v.map(Some))
            .map_err(|_| tonic::Status::invalid_argument("invalid asset id"))?;
        let address_index = request
            .get_ref()
            .address_index
            .to_owned()
            .map(AddressIndex::try_from)
            .map_or(Ok(None), |v| v.map(Some))
            .map_err(|_| tonic::Status::invalid_argument("invalid address index"))?;
        let amount_to_spend = request.get_ref().amount_to_spend;

        let notes = self
            .storage
            .notes(include_spent, asset_id, address_index, amount_to_spend)
            .await
            .map_err(|e| tonic::Status::unavailable(format!("error fetching notes: {}", e)))?;

        let stream = try_stream! {
            for note in notes {
                yield pb::NotesResponse {
                    note_record: Some(note.into()),
                }
            }
        };

        Ok(tonic::Response::new(
            stream
                .map_err(|e: anyhow::Error| {
                    tonic::Status::unavailable(format!("error getting notes: {}", e))
                })
                .boxed(),
        ))
    }

    async fn assets(
        &self,
        _request: tonic::Request<pb::AssetsRequest>,
    ) -> Result<tonic::Response<Self::AssetsStream>, tonic::Status> {
        self.check_worker().await?;

        // Fetch assets from storage.
        let assets = self
            .storage
            .assets()
            .await
            .map_err(|e| tonic::Status::unavailable(format!("error fetching assets: {}", e)))?;

        let stream = try_stream! {
            for asset in assets {
                yield
                    pb::AssetsResponse {
                        asset: Some(asset.into()),
                    }
            }
        };

        Ok(tonic::Response::new(
            stream
                .map_err(|e: anyhow::Error| {
                    tonic::Status::unavailable(format!("error getting assets: {}", e))
                })
                .boxed(),
        ))
    }

    async fn transaction_hashes(
        &self,
        request: tonic::Request<pb::TransactionHashesRequest>,
    ) -> Result<tonic::Response<Self::TransactionHashesStream>, tonic::Status> {
        self.check_worker().await?;
        // Fetch transactions from storage.
        let txs = self
            .storage
            .transaction_hashes(request.get_ref().start_height, request.get_ref().end_height)
            .await
            .map_err(|e| {
                tonic::Status::unavailable(format!("error fetching transactions: {}", e))
            })?;

        let stream = try_stream! {
            for tx in txs {
                yield TransactionHashesResponse {
                    block_height: tx.0,
                    tx_hash: tx.1,
                }
            }
        };

        Ok(tonic::Response::new(
            stream
                .map_err(|e: anyhow::Error| {
                    tonic::Status::unavailable(format!("error getting transactions: {}", e))
                })
                .boxed(),
        ))
    }

    async fn transactions(
        &self,
        request: tonic::Request<pb::TransactionsRequest>,
    ) -> Result<tonic::Response<Self::TransactionsStream>, tonic::Status> {
        self.check_worker().await?;
        // Fetch transactions from storage.
        let txs = self
            .storage
            .transactions(request.get_ref().start_height, request.get_ref().end_height)
            .await
            .map_err(|e| {
                tonic::Status::unavailable(format!("error fetching transactions: {}", e))
            })?;

        let stream = try_stream! {
            for tx in txs {
                yield TransactionsResponse {
                    block_height: tx.0,
                    tx_hash: tx.1,
                    tx: Some(tx.2.into())
                }
            }
        };

        Ok(tonic::Response::new(
            stream
                .map_err(|e: anyhow::Error| {
                    tonic::Status::unavailable(format!("error getting transactions: {}", e))
                })
                .boxed(),
        ))
    }

    async fn transaction_by_hash(
        &self,
        request: tonic::Request<pb::TransactionByHashRequest>,
    ) -> Result<tonic::Response<pb::TransactionByHashResponse>, tonic::Status> {
        self.check_worker().await?;

        // Fetch transactions from storage.
        let tx = self
            .storage
            .transaction_by_hash(&request.get_ref().tx_hash)
            .await
            .map_err(|e| {
                tonic::Status::unavailable(format!("error fetching transaction: {}", e))
            })?;

        Ok(tonic::Response::new(pb::TransactionByHashResponse {
            tx: tx.map(Into::into),
        }))
    }

    async fn witness(
        &self,
        request: tonic::Request<pb::WitnessRequest>,
    ) -> Result<tonic::Response<WitnessResponse>, tonic::Status> {
        self.check_worker().await?;
        self.check_fvk(request.get_ref().account_id.as_ref())
            .await?;

        // Acquire a read lock for the NCT that will live for the entire request,
        // so that all auth paths are relative to the same NCT root.
        let nct = self.note_commitment_tree.read().await;

        // Read the NCT root
        let anchor = nct.root();

        // Obtain an auth path for each requested note commitment
        let requested_note_commitments = request
            .get_ref()
            .note_commitments
            .iter()
            .map(|nc| Commitment::try_from(nc.clone()))
            .collect::<Result<Vec<Commitment>, _>>()
            .map_err(|_| {
                tonic::Status::new(
                    tonic::Code::InvalidArgument,
                    "Unable to deserialize note commitment",
                )
            })?;

        tracing::debug!(?requested_note_commitments);

        let auth_paths: Vec<Proof> = requested_note_commitments
            .iter()
            .map(|nc| {
                nct.witness(*nc).ok_or_else(|| {
                    tonic::Status::new(tonic::Code::InvalidArgument, "Note commitment missing")
                })
            })
            .collect::<Result<Vec<Proof>, tonic::Status>>()?;

        // Release the read lock on the NCT
        drop(nct);

        let mut witness_data = WitnessData {
            anchor,
            note_commitment_proofs: auth_paths
                .into_iter()
                .map(|proof| (proof.commitment(), proof))
                .collect(),
        };

        tracing::debug!(?witness_data);

        let tx_plan: TransactionPlan =
            request
                .get_ref()
                .to_owned()
                .transaction_plan
                .map_or(TransactionPlan::default(), |x| {
                    x.try_into()
                        .expect("TransactionPlan should exist in request")
                });

        // Now we need to augment the witness data with dummy proofs such that
        // note commitments corresponding to dummy spends also have proofs.
        for nc in tx_plan
            .spend_plans()
            .filter(|plan| plan.note.amount() == 0u64.into())
            .map(|plan| plan.note.commit())
        {
            witness_data.add_proof(nc, Proof::dummy(&mut OsRng, nc));
        }

        let witness_response = WitnessResponse {
            witness_data: Some(witness_data.into()),
        };
        Ok(tonic::Response::new(witness_response))
    }

    async fn chain_parameters(
        &self,
        _request: tonic::Request<pb::ChainParametersRequest>,
    ) -> Result<tonic::Response<pb::ChainParametersResponse>, tonic::Status> {
        self.check_worker().await?;

        let parameters = self.storage.chain_params().await.map_err(|e| {
            tonic::Status::unavailable(format!("error getting chain params: {}", e))
        })?;

        let response = ChainParametersResponse {
            parameters: Some(parameters.into()),
        };

        Ok(tonic::Response::new(response))
    }

    async fn fmd_parameters(
        &self,
        _request: tonic::Request<pb::FmdParametersRequest>,
    ) -> Result<tonic::Response<pb::FmdParametersResponse>, tonic::Status> {
        self.check_worker().await?;

        let parameters =
            self.storage.fmd_parameters().await.map_err(|e| {
                tonic::Status::unavailable(format!("error getting FMD params: {}", e))
            })?;

        let response = FmdParametersResponse {
            parameters: Some(parameters.into()),
        };

        Ok(tonic::Response::new(response))
    }
}
