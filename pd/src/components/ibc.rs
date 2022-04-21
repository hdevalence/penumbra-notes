use std::convert::TryFrom;

use anyhow::Result;
use async_trait::async_trait;
use ibc::{
    clients::ics07_tendermint::{
        client_def::TendermintClient, client_state::ClientState as TendermintClientState,
        consensus_state::ConsensusState as TendermintConsensusState,
        header::Header as TendermintHeader,
    },
    core::{
        ics02_client::{
            client_consensus::AnyConsensusState,
            client_state::{AnyClientState, ClientState},
            header::AnyHeader,
            height::Height,
            msgs::{create_client::MsgCreateAnyClient, update_client::MsgUpdateAnyClient},
        },
        ics24_host::identifier::ClientId,
    },
};
use penumbra_ibc::{ClientCounter, ClientData, ConsensusState, IBCAction, VerifiedHeights};
use penumbra_proto::ibc::ibc_action::Action::{CreateClient, UpdateClient};
use penumbra_transaction::{Action, Transaction};
use tendermint::{abci, Time};
use tendermint_light_client_verifier::{
    types::{Time as LightClientTime, TrustedBlockState, UntrustedBlockState},
    ProdVerifier, Verdict, Verifier,
};
use tracing::instrument;

use super::{app::View as _, Component};
use crate::{genesis, Overlay, OverlayExt};

pub struct IBCComponent {
    overlay: Overlay,
}

#[async_trait]
impl Component for IBCComponent {
    #[instrument(name = "ibc", skip(overlay))]
    async fn new(overlay: Overlay) -> Result<Self> {
        Ok(Self { overlay })
    }

    #[instrument(name = "ibc", skip(self, _app_state))]
    async fn init_chain(&mut self, _app_state: &genesis::AppState) -> Result<()> {
        // set the initial client count
        self.overlay.put_client_counter(ClientCounter(0)).await;

        Ok(())
    }

    #[instrument(name = "ibc", skip(self, _begin_block))]
    async fn begin_block(&mut self, _begin_block: &abci::request::BeginBlock) -> Result<()> {
        Ok(())
    }

    #[instrument(name = "ibc", skip(tx))]
    fn check_tx_stateless(tx: &Transaction) -> Result<()> {
        for ibc_action in tx.ibc_actions() {
            validate_ibc_action_stateless(ibc_action)?;
        }
        Ok(())
    }

    #[instrument(name = "ibc", skip(self, tx))]
    async fn check_tx_stateful(&self, tx: &Transaction) -> Result<()> {
        for ibc_action in tx.ibc_actions() {
            self.validate_ibc_action_stateful(ibc_action).await?;
        }
        Ok(())
    }

    #[instrument(name = "ibc", skip(self, tx))]
    async fn execute_tx(&mut self, tx: &Transaction) -> Result<()> {
        // Handle any IBC actions found in the transaction.
        for ibc_action in tx.ibc_actions() {
            self.execute_ibc_action(ibc_action).await;
        }

        Ok(())
    }

    #[instrument(name = "ibc", skip(self, _end_block))]
    async fn end_block(&mut self, _end_block: &abci::request::EndBlock) -> Result<()> {
        Ok(())
    }
}

// validates the given ibc action statelessly
fn validate_ibc_action_stateless(ibc_action: &IBCAction) -> Result<(), anyhow::Error> {
    match &ibc_action.action {
        CreateClient(msg) => {
            let msg_create_client = MsgCreateAnyClient::try_from(msg.clone())?;

            validate_create_client_stateless(&msg_create_client)?;
        }
        UpdateClient(msg) => {
            let msg_update_client = MsgUpdateAnyClient::try_from(msg.clone())?;

            validate_update_client_stateless(&msg_update_client)?;
        }
        _ => return Ok(()),
    }

    Ok(())
}

// check that the client is Tendermint
fn validate_create_client_stateless(
    create_client: &MsgCreateAnyClient,
) -> Result<(), anyhow::Error> {
    match create_client.client_state {
        AnyClientState::Tendermint(_) => {}
        _ => {
            return Err(anyhow::anyhow!(
                "only Tendermint clients are supported at this time"
            ))
        }
    }
    match create_client.consensus_state {
        AnyConsensusState::Tendermint(_) => {}
        _ => {
            return Err(anyhow::anyhow!(
                "only Tendermint consensus is supported at this time"
            ))
        }
    }

    Ok(())
}

fn validate_update_client_stateless(
    update_client: &MsgUpdateAnyClient,
) -> Result<(), anyhow::Error> {
    match update_client.header {
        AnyHeader::Tendermint(_) => {}
        _ => {
            return Err(anyhow::anyhow!(
                "only Tendermint clients are supported at this time"
            ))
        }
    }

    Ok(())
}

impl IBCComponent {
    // validates the given IBC action statefully.
    async fn validate_ibc_action_stateful(&self, ibc_action: &IBCAction) -> Result<()> {
        match &ibc_action.action {
            CreateClient(msg) => {
                let msg_create_client = MsgCreateAnyClient::try_from(msg.clone())?;

                self.validate_create_client_stateful(msg_create_client)
                    .await?;
            }
            UpdateClient(msg) => {
                let msg_update_client = MsgUpdateAnyClient::try_from(msg.clone())?;

                self.validate_update_client_stateful(msg_update_client)
                    .await?;
            }
            _ => return Ok(()),
        }

        Ok(())
    }

    // executes the given IBC action, assuming that it has already been validated.
    async fn execute_ibc_action(&mut self, ibc_action: &IBCAction) {
        match &ibc_action.action {
            CreateClient(raw_msg_create_client) => {
                let msg_create_client =
                    MsgCreateAnyClient::try_from(raw_msg_create_client.clone()).unwrap();

                self.execute_create_client(msg_create_client).await;
            }
            UpdateClient(raw_msg_update_client) => {
                let msg_update_client =
                    MsgUpdateAnyClient::try_from(raw_msg_update_client.clone()).unwrap();

                self.execute_update_client(msg_update_client).await;
            }
            _ => {}
        }
    }

    // validate IBC UpdateClient. This is one of the most important IBC messages, as it has
    // the responsibility of verifying consensus state updates (through the semantics of
    // the light client's verification fn).
    //
    // verify:
    // - we have a client corresponding to the UpdateClient's client_id
    // - the stored client is not frozen
    // - the stored client is not expired
    // - the supplied update verifies using the semantics of the light client's header verification fn
    async fn validate_update_client_stateful(
        &self,
        msg_update_client: MsgUpdateAnyClient,
    ) -> Result<()> {
        // get the latest client state
        let client_data = self
            .overlay
            .get_client_data(&msg_update_client.client_id)
            .await?;

        // check that the client is not frozen or expired
        if client_data.client_state.0.is_frozen() {
            return Err(anyhow::anyhow!("client is frozen"));
        }

        // check if client is expired
        let latest_consensus_state = self
            .overlay
            .get_verified_consensus_state(
                client_data.client_state.0.latest_height(),
                client_data.client_id.clone(),
            )
            .await?;

        let latest_consensus_state_tm = match latest_consensus_state.0 {
            AnyConsensusState::Tendermint(consensus_state) => consensus_state,
            _ => {
                return Err(anyhow::anyhow!(
                    "consensus state is not a Tendermint client"
                ))
            }
        };
        let now = self.overlay.get_block_timestamp().await?;
        let stamp = latest_consensus_state_tm.timestamp.to_rfc3339();
        let duration = now.duration_since(Time::parse_from_rfc3339(&stamp).unwrap())?;
        if client_data.client_state.0.expired(duration) {
            return Err(anyhow::anyhow!("client is expired"));
        }

        // verify the clientupdate's header
        let tm_client_state = match client_data.clone().client_state.0 {
            AnyClientState::Tendermint(tm_state) => tm_state,
            _ => return Err(anyhow::anyhow!("unsupported client type")),
        };
        let tm_header = match msg_update_client.header {
            AnyHeader::Tendermint(tm_header) => tm_header,
            _ => {
                return Err(anyhow::anyhow!("client update is not a Tendermint header"));
            }
        };

        self.verify_tendermint_update(
            msg_update_client.client_id.clone(),
            tm_client_state,
            tm_header.clone(),
        )
        .await?;

        Ok(())
    }

    async fn validate_create_client_stateful(
        &self,
        msg_create_client: MsgCreateAnyClient,
    ) -> Result<()> {
        let id_counter = self.overlay.client_counter().await?;
        ClientId::new(msg_create_client.client_state.client_type(), id_counter.0)?;

        Ok(())
    }

    // execute a UpdateClient IBC action. this assumes that the UpdateClient has already been
    // validated, including header verification.
    async fn execute_update_client(&mut self, msg_update_client: MsgUpdateAnyClient) {
        // get the latest client state
        let client_data = self
            .overlay
            .get_client_data(&msg_update_client.client_id)
            .await
            .unwrap();

        let tm_client_state = match client_data.clone().client_state.0 {
            AnyClientState::Tendermint(tm_state) => tm_state,
            _ => panic!("unsupported client type"),
        };
        let tm_header = match msg_update_client.header {
            AnyHeader::Tendermint(tm_header) => tm_header,
            _ => {
                panic!("update header is not a Tendermint header");
            }
        };

        let (next_tm_client_state, next_tm_consensus_state) = self
            .next_tendermint_state(
                msg_update_client.client_id.clone(),
                tm_client_state,
                tm_header.clone(),
            )
            .await;

        // store the updated client and consensus states
        let height = self.overlay.get_block_height().await.unwrap();
        let now = self.overlay.get_block_timestamp().await.unwrap();
        let next_client_data = client_data.with_new_client_state(
            AnyClientState::Tendermint(next_tm_client_state),
            now.to_rfc3339(),
            height,
        );
        self.overlay.put_client_data(next_client_data).await;
        self.overlay
            .put_verified_consensus_state(
                tm_header.height(),
                msg_update_client.client_id.clone(),
                ConsensusState(AnyConsensusState::Tendermint(next_tm_consensus_state)),
            )
            .await
            .unwrap();
    }

    // execute IBC CreateClient.
    //
    //  we compute the client's ID (a concatenation of a monotonically increasing integer, the
    //  number of clients on Penumbra, and the client type) and commit the following to our state:
    // - client type
    // - consensus state
    // - processed time and height
    async fn execute_create_client(&mut self, msg_create_client: MsgCreateAnyClient) {
        // get the current client counter
        let id_counter = self.overlay.client_counter().await.unwrap();
        let client_id =
            ClientId::new(msg_create_client.client_state.client_type(), id_counter.0).unwrap();

        tracing::info!("creating client {:?}", client_id);

        let height = self.overlay.get_block_height().await.unwrap();
        let timestamp = self.overlay.get_block_timestamp().await.unwrap();

        let data = ClientData::new(
            client_id.clone(),
            msg_create_client.client_state,
            timestamp.to_rfc3339(),
            height,
        );

        // store the client data
        self.overlay.put_client_data(data.clone()).await;

        // store the genesis consensus state
        self.overlay
            .put_verified_consensus_state(
                data.client_state.0.latest_height(),
                client_id,
                ConsensusState(msg_create_client.consensus_state),
            )
            .await
            .unwrap();

        // increment client counter
        let counter = self
            .overlay
            .client_counter()
            .await
            .unwrap_or(ClientCounter(0));
        self.overlay
            .put_client_counter(ClientCounter(counter.0 + 1))
            .await;
    }

    // given an already verified tendermint header, and a trusted tendermint client state, compute
    // the next client and consensus states.
    async fn next_tendermint_state(
        &self,
        client_id: ClientId,
        trusted_client_state: TendermintClientState,
        verified_header: TendermintHeader,
    ) -> (TendermintClientState, TendermintConsensusState) {
        let verified_consensus_state = TendermintConsensusState::from(verified_header.clone());

        // if we have a stored consensus state for this height that conflicts, we need to freeze
        // the client. if it doesn't conflict, we can return early
        if let Some(stored_cs_state) = self
            .overlay
            .get_verified_consensus_state(verified_header.height(), client_id.clone())
            .await
            .ok()
        {
            let stored_cs_state_tm = stored_cs_state.as_tendermint().unwrap();
            if stored_cs_state_tm == verified_consensus_state {
                return (trusted_client_state, verified_consensus_state);
            } else {
                return (
                    trusted_client_state
                        .with_header(verified_header.clone())
                        .with_frozen_height(verified_header.height())
                        .unwrap(),
                    verified_consensus_state,
                );
            }
        }

        // check that updates have monotonic timestamps. we may receive client updates that are
        // disjoint: the header we received and validated may be older than the newest header we
        // have. In that case, we need to verify that the timestamp is correct. if it isn't, freeze
        // the client.
        let next_consensus_state = self
            .overlay
            .next_verified_consensus_state(&client_id, verified_header.height())
            .await
            .unwrap();
        let prev_consensus_state = self
            .overlay
            .prev_verified_consensus_state(&client_id, verified_header.height())
            .await
            .unwrap();

        // case 1: if we have a verified consensus state previous to this header, verify that this
        // header's timestamp is greater than or equal to the stored consensus state's timestamp
        if let Some(prev_state) = prev_consensus_state {
            let prev_state_tm = prev_state.as_tendermint().unwrap();
            if !(verified_header.signed_header.header().time >= prev_state_tm.timestamp) {
                return (
                    trusted_client_state
                        .with_header(verified_header.clone())
                        .with_frozen_height(verified_header.height())
                        .unwrap(),
                    verified_consensus_state,
                );
            }
        }
        // case 2: if we have a verified consensus state with higher block height than this header,
        // verify that this header's timestamp is less than or equal to this header's timestamp.
        if let Some(next_state) = next_consensus_state {
            let next_state_tm = next_state.as_tendermint().unwrap();
            if !(verified_header.signed_header.header().time <= next_state_tm.timestamp) {
                return (
                    trusted_client_state
                        .with_header(verified_header.clone())
                        .with_frozen_height(verified_header.height())
                        .unwrap(),
                    verified_consensus_state,
                );
            }
        }

        return (
            trusted_client_state.with_header(verified_header.clone()),
            verified_consensus_state,
        );
    }

    // verify a new header for a tendermint client, given a trusted client state.
    async fn verify_tendermint_update(
        &self,
        client_id: ClientId,
        trusted_client_state: TendermintClientState,
        untrusted_header: TendermintHeader,
    ) -> Result<()> {
        let untrusted_consensus_state = TendermintConsensusState::from(untrusted_header.clone());

        if untrusted_header.height().revision_number != trusted_client_state.chain_id.version() {
            return Err(anyhow::anyhow!(
                "client update revision number does not match client state"
            ));
        }

        // check if we already have a consensus state for this height, if we do, check that it is
        // the same as this update, if it is, return early.
        if let Some(stored_consensus_state) = self
            .overlay
            .get_verified_consensus_state(untrusted_header.height(), client_id.clone())
            .await
            .ok()
        {
            let stored_tm_consensus_state = stored_consensus_state.as_tendermint()?;
            if stored_tm_consensus_state == untrusted_consensus_state {
                return Ok(());
            }
        }

        let last_trusted_consensus_state = self
            .overlay
            .get_verified_consensus_state(untrusted_header.trusted_height, client_id.clone())
            .await?
            .as_tendermint()?;

        let trusted_state = TrustedBlockState {
            header_time: last_trusted_consensus_state.timestamp,
            height: untrusted_header
                .trusted_height
                .revision_height
                .try_into()
                .map_err(|_| anyhow::anyhow!("invalid header height"))?,
            next_validators: &untrusted_header.trusted_validator_set,
            next_validators_hash: last_trusted_consensus_state.next_validators_hash,
        };

        let untrusted_state = UntrustedBlockState {
            signed_header: &untrusted_header.signed_header,
            validators: &untrusted_header.validator_set,

            // TODO: do we need this?
            next_validators: None,
        };

        let options = trusted_client_state.as_light_client_options()?;
        let verifier = ProdVerifier::default();
        let current_block_timestamp = LightClientTime::parse_from_rfc3339(
            &self.overlay.get_block_timestamp().await?.to_rfc3339(),
        )
        .unwrap();

        let verdict = verifier.verify(
            untrusted_state,
            trusted_state,
            &options,
            current_block_timestamp,
        );
        match verdict {
            Verdict::Success => {}
            Verdict::NotEnoughTrust(voting_power_tally) => {
                return Err(anyhow::anyhow!(
                    "not enough trust, voting power tally: {:?}",
                    voting_power_tally
                ));
            }
            Verdict::Invalid(detail) => {
                return Err(anyhow::anyhow!(
                    "could not verify tendermint header: invalid: {:?}",
                    detail
                ));
            }
        }

        // consensus state is verified
        Ok(())
    }
}

#[async_trait]
pub trait View: OverlayExt + Send + Sync {
    async fn put_client_counter(&mut self, counter: ClientCounter) {
        self.put_domain("ibc/ics02-client/client_counter".into(), counter)
            .await;
    }
    async fn client_counter(&self) -> Result<ClientCounter> {
        self.get_domain("ibc/ics02-client/client_counter".into())
            .await
            .map(|counter| counter.unwrap_or(ClientCounter(0)))
    }
    async fn put_client_data(&mut self, data: ClientData) {
        self.put_domain(
            format!(
                "ibc/ics02-client/clients/{}",
                hex::encode(data.client_id.as_bytes())
            )
            .into(),
            data,
        )
        .await;
    }
    async fn get_client_data(&self, client_id: &ClientId) -> Result<ClientData> {
        let client_data = self
            .get_domain(
                format!(
                    "ibc/ics02-client/clients/{}",
                    hex::encode(client_id.as_bytes())
                )
                .into(),
            )
            .await?;

        client_data.ok_or(anyhow::anyhow!("client not found"))
    }

    async fn get_verified_heights(&self, client_id: &ClientId) -> Result<Option<VerifiedHeights>> {
        self.get_domain(
            format!(
                "ibc/ics02-client/clients/{}/verified_heights",
                hex::encode(client_id.as_bytes())
            )
            .into(),
        )
        .await
    }

    async fn put_verified_heights(
        &mut self,
        client_id: &ClientId,
        verified_heights: VerifiedHeights,
    ) {
        self.put_domain(
            format!(
                "ibc/ics02-client/clients/{}/verified_heights",
                hex::encode(client_id.as_bytes())
            )
            .into(),
            verified_heights,
        )
        .await;
    }

    async fn get_verified_consensus_state(
        &self,
        height: Height,
        client_id: ClientId,
    ) -> Result<ConsensusState> {
        self.get_domain(
            format!(
                "ibc/ics02-client/clients/{}/consensus_state/{}",
                hex::encode(client_id.as_bytes()),
                height
            )
            .into(),
        )
        .await?
        .ok_or(anyhow::anyhow!("consensus state not found"))
    }

    async fn put_verified_consensus_state(
        &mut self,
        height: Height,
        client_id: ClientId,
        consensus_state: ConsensusState,
    ) -> Result<()> {
        self.put_domain(
            format!(
                "ibc/ics02-client/clients/{}/consensus_state/{}",
                hex::encode(client_id.as_bytes()),
                height
            )
            .into(),
            consensus_state,
        )
        .await;

        // update verified heights
        let mut verified_heights =
            self.get_verified_heights(&client_id)
                .await?
                .unwrap_or(VerifiedHeights {
                    heights: Vec::new(),
                });

        verified_heights.heights.push(height.clone());

        self.put_verified_heights(&client_id, verified_heights)
            .await;

        Ok(())
    }

    // returns the lowest verified consensus state that is higher than the given height, if it
    // exists.
    async fn next_verified_consensus_state(
        &self,
        client_id: &ClientId,
        height: Height,
    ) -> Result<Option<ConsensusState>> {
        let mut verified_heights =
            self.get_verified_heights(client_id)
                .await?
                .unwrap_or(VerifiedHeights {
                    heights: Vec::new(),
                });

        // WARNING: load-bearing sort
        verified_heights.heights.sort();

        if let Some(next_height) = verified_heights
            .heights
            .iter()
            .find(|&verified_height| verified_height > &height)
        {
            let next_cons_state = self
                .get_verified_consensus_state(*next_height, client_id.clone())
                .await?;
            return Ok(Some(next_cons_state));
        } else {
            return Ok(None);
        }
    }

    // returns the highest verified consensus state that is lower than the given height, if it
    // exists.
    async fn prev_verified_consensus_state(
        &self,
        client_id: &ClientId,
        height: Height,
    ) -> Result<Option<ConsensusState>> {
        let mut verified_heights =
            self.get_verified_heights(client_id)
                .await?
                .unwrap_or(VerifiedHeights {
                    heights: Vec::new(),
                });

        // WARNING: load-bearing sort
        verified_heights.heights.sort();

        if let Some(prev_height) = verified_heights
            .heights
            .iter()
            .find(|&verified_height| verified_height < &height)
        {
            let prev_cons_state = self
                .get_verified_consensus_state(*prev_height, client_id.clone())
                .await?;
            return Ok(Some(prev_cons_state));
        } else {
            return Ok(None);
        }
    }
}

impl<T: OverlayExt + Send + Sync> View for T {}
