use std::str::FromStr;

use ibc::{
    clients::ics07_tendermint::{client_state, consensus_state},
    core::{
        ics02_client::{client_consensus::AnyConsensusState, client_state::AnyClientState},
        ics24_host::identifier::ClientId,
    },
};
use penumbra_proto::{ibc as pb, Protobuf};
use tendermint_proto::Protobuf as TmProtobuf;

pub const TENDERMINT_CLIENT_STATE_TYPE_URL: &str = "/ibc.lightclients.tendermint.v1.ClientState";
pub const TENDERMINT_CONSENSUS_STATE_TYPE_URL: &str =
    "/ibc.lightclients.tendermint.v1.ConsensusState";

#[derive(Clone, Debug)]
pub struct ClientState(AnyClientState);

impl Protobuf<prost_types::Any> for ClientState {}

impl TryFrom<prost_types::Any> for ClientState {
    type Error = anyhow::Error;

    fn try_from(raw: prost_types::Any) -> Result<Self, Self::Error> {
        match raw.type_url.as_str() {
            TENDERMINT_CLIENT_STATE_TYPE_URL => Ok(ClientState(AnyClientState::Tendermint(
                client_state::ClientState::decode_vec(&raw.value)
                    .map_err(|_| anyhow::anyhow!("could not decode tendermint client state"))?,
            ))),

            _ => Err(anyhow::anyhow!("unknown client type: {}", raw.type_url)),
        }
    }
}

impl From<ClientState> for prost_types::Any {
    fn from(value: ClientState) -> Self {
        match value {
            ClientState(AnyClientState::Tendermint(value)) => prost_types::Any {
                type_url: TENDERMINT_CLIENT_STATE_TYPE_URL.to_string(),
                value: value
                    .encode_vec()
                    .expect("encoding to `Any` from `ClientState::Tendermint`"),
            },
        }
    }
}

#[derive(Clone, Debug)]
pub struct ConsensusState(AnyConsensusState);

impl Protobuf<prost_types::Any> for ConsensusState {}

impl TryFrom<prost_types::Any> for ConsensusState {
    type Error = anyhow::Error;

    fn try_from(raw: prost_types::Any) -> Result<Self, Self::Error> {
        match raw.type_url.as_str() {
            TENDERMINT_CONSENSUS_STATE_TYPE_URL => {
                Ok(ConsensusState(AnyConsensusState::Tendermint(
                    consensus_state::ConsensusState::decode_vec(&raw.value).map_err(|_| {
                        anyhow::anyhow!("could not decode tendermint consensus state")
                    })?,
                )))
            }

            _ => Err(anyhow::anyhow!(
                "unknown consensus state type: {}",
                raw.type_url
            )),
        }
    }
}

impl From<ConsensusState> for prost_types::Any {
    fn from(value: ConsensusState) -> Self {
        match value {
            ConsensusState(AnyConsensusState::Tendermint(value)) => prost_types::Any {
                type_url: TENDERMINT_CONSENSUS_STATE_TYPE_URL.to_string(),
                value: value
                    .encode_vec()
                    .expect("encoding to `Any` from `ConsensusState::Tendermint`"),
            },
        }
    }
}

/// ClientData encapsulates the data that represents an ICS-02 client, stored in the Penumbra
/// state.
#[derive(Clone, Debug)]
pub struct ClientData {
    pub client_id: ClientId,
    pub client_state: ClientState,
    pub consensus_state: ConsensusState,
    pub processed_time: String,
    pub processed_height: u64,
}

impl Protobuf<pb::ClientData> for ClientData {}

impl TryFrom<pb::ClientData> for ClientData {
    type Error = anyhow::Error;

    fn try_from(msg: pb::ClientData) -> Result<Self, Self::Error> {
        Ok(ClientData {
            client_id: ClientId::from_str(&msg.client_id)?,
            client_state: ClientState::try_from(msg.client_state.unwrap())?,
            consensus_state: ConsensusState::try_from(msg.consensus_state.unwrap())?,
            processed_time: msg.processed_time,
            processed_height: msg.processed_height,
        })
    }
}

impl From<ClientData> for pb::ClientData {
    fn from(d: ClientData) -> Self {
        Self {
            client_id: d.client_id.to_string(),
            client_state: Some(d.client_state.into()),
            consensus_state: Some(d.consensus_state.into()),
            processed_time: d.processed_time,
            processed_height: d.processed_height,
        }
    }
}
