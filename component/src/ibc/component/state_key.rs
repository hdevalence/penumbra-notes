use ibc::{
    core::{
        ics04_channel::packet::Packet,
        ics24_host::identifier::{ChannelId, ClientId, ConnectionId, PortId},
    },
    Height,
};

use std::string::String;

pub fn client_type(client_id: &ClientId) -> String {
    format!("clients/{}/clientType", client_id)
}

pub fn client_state(client_id: &ClientId) -> String {
    format!("clients/{}/clientState", client_id)
}

pub fn verified_client_consensus_state(client_id: &ClientId, height: &Height) -> String {
    format!("clients/{}/consensusStates/{}", client_id, height)
}

pub fn client_processed_heights(client_id: &ClientId, height: &Height) -> String {
    format!("clients/{}/processedHeights/{}", client_id, height)
}
pub fn client_processed_times(client_id: &ClientId, height: &Height) -> String {
    format!("clients/{}/processedTimes/{}", client_id, height)
}

pub fn client_connections(client_id: &ClientId) -> String {
    format!("clients/{}/connections", client_id)
}

pub fn connection(connection_id: &ConnectionId) -> String {
    format!("connections/{}", connection_id.as_str())
}

pub fn connection_counter() -> &'static str {
    "ibc/ics03-connection/connection_counter"
}

pub fn channel(channel_id: &ChannelId, port_id: &PortId) -> String {
    format!("channelEnds/ports/{}/channels/{}", port_id, channel_id)
}

pub fn seq_recv(channel_id: &ChannelId, port_id: &PortId) -> String {
    format!(
        "seqRecvs/ports/{}/channels/{}/nextSequenceRecv",
        port_id, channel_id
    )
}

pub fn seq_ack(channel_id: &ChannelId, port_id: &PortId) -> String {
    format!(
        "seqAcks/ports/{}/channels/{}/nextSequenceAck",
        port_id, channel_id
    )
}

pub fn seq_send(channel_id: &ChannelId, port_id: &PortId) -> String {
    format!(
        "seqSends/ports/{}/channels/{}/nextSequenceSend",
        port_id, channel_id
    )
}

pub fn packet_receipt(packet: &Packet) -> String {
    format!(
        "receipts/ports/{}/channels/{}/receipts/{}",
        packet.destination_port, packet.destination_channel, packet.sequence
    )
}

pub fn packet_commitment(packet: &Packet) -> String {
    format!(
        "commitments/ports/{}/channels/{}/packets/{}",
        packet.source_port, packet.source_channel, packet.sequence
    )
}

pub fn packet_commitment_by_port(
    port_id: &PortId,
    channel_id: &ChannelId,
    sequence: u64,
) -> String {
    format!(
        "commitments/ports/{}/channels/{}/packets/{}",
        port_id, channel_id, sequence
    )
}
