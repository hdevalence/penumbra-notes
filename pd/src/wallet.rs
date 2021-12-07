use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tonic::Status;

use penumbra_proto::{
    light_wallet::{light_wallet_server::LightWallet, CompactBlock, CompactBlockRangeRequest},
    thin_wallet::{
        thin_wallet_server::ThinWallet, Asset, AssetListRequest, AssetLookupRequest,
        TransactionByNoteRequest, TransactionDetail,
    },
};

use crate::State;

#[tonic::async_trait]
impl LightWallet for State {
    type CompactBlockRangeStream = ReceiverStream<Result<CompactBlock, Status>>;

    async fn compact_block_range(
        &self,
        request: tonic::Request<CompactBlockRangeRequest>,
    ) -> Result<tonic::Response<Self::CompactBlockRangeStream>, Status> {
        let CompactBlockRangeRequest {
            start_height,
            end_height,
        } = request.into_inner();

        let current_height = self
            .height()
            .await
            .map_err(|_| tonic::Status::unavailable("database error"))?
            .value() as u32;

        // Treat end_height = 0 as end_height = current_height so that if the
        // end_height is unspecified in the proto, it will be treated as a
        // request to sync up to the current height.
        let end_height = if end_height == 0 {
            current_height
        } else {
            std::cmp::min(end_height, current_height)
        };

        let (tx, rx) = mpsc::channel(100);

        let state = self.clone();
        tokio::spawn(async move {
            for height in start_height..=end_height {
                let block = state.compact_block(height.into()).await;
                tracing::info!("sending block response: {:?}", block);
                tx.send(block.map_err(|_| tonic::Status::unavailable("database error")))
                    .await
                    .unwrap();
            }
        });

        Ok(tonic::Response::new(Self::CompactBlockRangeStream::new(rx)))
    }
}

#[tonic::async_trait]
impl ThinWallet for State {
    type AssetListStream = ReceiverStream<Result<Asset, Status>>;

    async fn transaction_by_note(
        &self,
        request: tonic::Request<TransactionByNoteRequest>,
    ) -> Result<tonic::Response<TransactionDetail>, Status> {
        let state = self.clone();
        let transaction = state
            .transaction_by_note(request.into_inner().cm)
            .await
            .map_err(|_| tonic::Status::not_found("transaction not found"))?;
        Ok(tonic::Response::new(transaction))
    }

    async fn asset_lookup(
        &self,
        request: tonic::Request<AssetLookupRequest>,
    ) -> Result<tonic::Response<Asset>, Status> {
        let state = self.clone();
        let asset = state
            .asset_lookup(request.into_inner().asset_id)
            .await
            .map_err(|_| tonic::Status::not_found("asset not found"))?;
        Ok(tonic::Response::new(asset))
    }

    async fn asset_list(
        &self,
        _request: tonic::Request<AssetListRequest>,
    ) -> Result<tonic::Response<Self::AssetListStream>, Status> {
        let state = self.clone();

        let (tx, rx) = mpsc::channel(100);
        tokio::spawn(async move {
            let assets = state
                .asset_list()
                .await
                .map_err(|_| tonic::Status::unavailable("database error"))
                .unwrap();
            for asset in &assets[..] {
                tracing::info!("sending asset list response: {:?}", asset);
                tx.send(Ok(asset.clone())).await.unwrap();
            }
        });

        Ok(tonic::Response::new(Self::AssetListStream::new(rx)))
    }
}
