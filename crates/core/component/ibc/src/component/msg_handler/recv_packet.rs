use anyhow::Result;
use async_trait::async_trait;
use ibc_types::core::{
    channel::{
        channel::{Order as ChannelOrder, State as ChannelState},
        events,
        msgs::MsgRecvPacket,
        PortId,
    },
    client::Height as IBCHeight,
    connection::State as ConnectionState,
};
use penumbra_chain::component::StateReadExt;
use penumbra_storage::StateWrite;

use crate::component::{
    app_handler::{AppHandlerCheck, AppHandlerExecute},
    channel::{StateReadExt as _, StateWriteExt},
    connection::StateReadExt as _,
    proof_verification::PacketProofVerifier,
    transfer::Ics20Transfer,
    MsgHandler,
};

#[async_trait]
impl MsgHandler for MsgRecvPacket {
    async fn check_stateless(&self) -> Result<()> {
        // NOTE: no additional stateless validation is possible

        Ok(())
    }

    async fn try_execute<S: StateWrite>(&self, mut state: S) -> Result<()> {
        tracing::debug!(msg = ?self);
        let channel = state
            .get_channel(&self.packet.chan_on_b, &self.packet.port_on_b)
            .await?
            .ok_or_else(|| anyhow::anyhow!("channel not found"))?;
        if !channel.state_matches(&ChannelState::Open) {
            anyhow::bail!("channel is not open");
        }

        // TODO: capability authentication?

        if self.packet.port_on_a != channel.counterparty().port_id {
            anyhow::bail!("packet source port does not match channel");
        }
        let counterparty_channel = channel
            .counterparty()
            .channel_id()
            .ok_or_else(|| anyhow::anyhow!("missing channel id"))?;

        if self.packet.chan_on_a.ne(counterparty_channel) {
            anyhow::bail!("packet source channel does not match channel");
        }

        let connection = state
            .get_connection(&channel.connection_hops[0])
            .await?
            .ok_or_else(|| anyhow::anyhow!("connection not found for channel"))?;

        if !connection.state_matches(&ConnectionState::Open) {
            anyhow::bail!("connection for channel is not open");
        }

        let block_height = state.get_block_height().await?;
        let height = IBCHeight::new(0, block_height)?;

        if self.packet.timeout_height_on_b.has_expired(height) {
            anyhow::bail!("packet has timed out");
        }

        let packet_timeout = self
            .packet
            .timeout_timestamp_on_b
            .into_tm_time()
            .ok_or_else(|| anyhow::anyhow!("invalid timestamp"))?;

        if state.get_block_timestamp().await? >= packet_timeout {
            anyhow::bail!("packet has timed out");
        }

        state.verify_packet_recv_proof(&connection, self).await?;

        if channel.ordering == ChannelOrder::Ordered {
            let next_sequence_recv = state
                .get_recv_sequence(&self.packet.chan_on_b, &self.packet.port_on_b)
                .await?;

            if self.packet.sequence != next_sequence_recv.into() {
                anyhow::bail!("packet sequence number does not match");
            }
        } else if state.seen_packet(&self.packet).await? {
            anyhow::bail!("packet has already been processed");
        }

        let transfer = PortId::transfer();
        if self.packet.port_on_b == transfer {
            Ics20Transfer::recv_packet_check(&mut state, self).await?;
        } else {
            anyhow::bail!("invalid port id");
        }

        if channel.ordering == ChannelOrder::Ordered {
            let mut next_sequence_recv = state
                .get_recv_sequence(&self.packet.chan_on_b, &self.packet.port_on_b)
                .await?;

            next_sequence_recv += 1;
            state.put_recv_sequence(
                &self.packet.chan_on_b,
                &self.packet.port_on_b,
                next_sequence_recv,
            );
        } else {
            // for unordered channels we must set the receipt so it can be verified on the other side
            // this receipt does not contain any data, since the packet has not yet been processed
            // it's just a single store key set to an empty string to indicate that the packet has been received
            state.put_packet_receipt(&self.packet);
        }

        state.record(
            events::packet::ReceivePacket {
                packet_data: self.packet.data.clone(),
                timeout_height: self.packet.timeout_height_on_b.clone(),
                timeout_timestamp: self.packet.timeout_timestamp_on_b.clone(),
                sequence: self.packet.sequence.clone(),
                src_port_id: self.packet.port_on_a.clone(),
                src_channel_id: self.packet.chan_on_a.clone(),
                dst_port_id: self.packet.port_on_b.clone(),
                dst_channel_id: self.packet.chan_on_b.clone(),
                channel_ordering: channel.ordering.clone(),
                dst_connection_id: channel.connection_hops[0].clone(),
            }
            .into(),
        );

        let transfer = PortId::transfer();
        if self.packet.port_on_b == transfer {
            Ics20Transfer::recv_packet_execute(state, self).await;
        } else {
            anyhow::bail!("invalid port id");
        }

        Ok(())
    }
}
