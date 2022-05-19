mod proof_verification {
    use super::super::*;

    #[async_trait]
    pub trait ChannelProofVerifier: StateExt {
        async fn verify_channel_proof(
            &self,
            connection: &ConnectionEnd,
            proofs: &ibc::proofs::Proofs,
            channel_id: &ChannelId,
            port_id: &PortId,
            expected_channel: &ChannelEnd,
        ) -> anyhow::Result<()> {
            // get the stored client state for the counterparty
            let trusted_client_state = self.get_client_state(connection.client_id()).await?;

            // check if the client is frozen
            // TODO: should we also check if the client is expired here?
            if trusted_client_state.is_frozen() {
                return Err(anyhow::anyhow!("client is frozen"));
            }

            // get the stored consensus state for the counterparty
            let trusted_consensus_state = self
                .get_verified_consensus_state(proofs.height(), connection.client_id().clone())
                .await?;

            let client_def = AnyClient::from_client_type(trusted_client_state.client_type());

            // PROOF VERIFICATION. verify that our counterparty committed expected_channel to its
            // state.
            client_def.verify_channel_state(
                &trusted_client_state,
                proofs.height(),
                connection.counterparty().prefix(),
                proofs.object_proof(),
                trusted_consensus_state.root(),
                port_id,
                channel_id,
                expected_channel,
            )?;

            Ok(())
        }
    }
    impl<T: StateExt> ChannelProofVerifier for T {}
}

pub mod channel_open_init {
    use super::super::*;

    #[async_trait]
    pub trait ChannelOpenInitCheck: StateExt + inner::Inner {
        async fn validate(&self, msg: &MsgChannelOpenInit) -> anyhow::Result<()> {
            let channel_id = self.get_channel_id().await?;

            self.verify_channel_does_not_exist(&channel_id, &msg.port_id)
                .await?;

            // NOTE: optimistic channel handshakes are allowed, so we don't check if the connection
            // is open here.
            self.verify_connections_exist(msg).await?;

            // TODO: do we want to do capability authentication?

            Ok(())
        }
    }
    mod inner {
        use super::*;

        #[async_trait]
        pub trait Inner: StateExt {
            async fn verify_connections_exist(
                &self,
                msg: &MsgChannelOpenInit,
            ) -> anyhow::Result<()> {
                self.get_connection(&msg.channel.connection_hops[0])
                    .await?
                    .ok_or_else(|| anyhow::anyhow!("connection not found"))
                    .map(|_| ())
            }
            async fn get_channel_id(&self) -> anyhow::Result<ChannelId> {
                let counter = self.get_channel_counter().await?;

                Ok(ChannelId::new(counter))
            }
            async fn verify_channel_does_not_exist(
                &self,
                channel_id: &ChannelId,
                port_id: &PortId,
            ) -> anyhow::Result<()> {
                let channel = self.get_channel(channel_id, port_id).await?;
                if channel.is_some() {
                    return Err(anyhow::anyhow!("channel already exists"));
                }
                Ok(())
            }
        }
        impl<T: StateExt> Inner for T {}
    }
    impl<T: StateExt> ChannelOpenInitCheck for T {}
}

pub mod channel_open_try {
    use super::super::*;
    use super::proof_verification::ChannelProofVerifier;

    #[async_trait]
    pub trait ChannelOpenTryCheck: StateExt + inner::Inner {
        async fn validate(&self, msg: &MsgChannelOpenTry) -> anyhow::Result<()> {
            let channel_id = ChannelId::new(self.get_channel_counter().await?);

            let connection = self.verify_connections_open(msg).await?;

            // TODO: do we want to do capability authentication?
            // TODO: version intersection

            let expected_counterparty = Counterparty::new(msg.port_id.clone(), None);

            let expected_channel = ChannelEnd {
                state: ChannelState::Init,
                ordering: msg.channel.ordering,
                remote: expected_counterparty,
                connection_hops: vec![connection
                    .counterparty()
                    .connection_id
                    .clone()
                    .ok_or_else(|| anyhow::anyhow!("no counterparty connection id provided"))?],
                version: msg.counterparty_version.clone(),
            };

            self.verify_channel_proof(
                &connection,
                &msg.proofs,
                &channel_id,
                &msg.port_id,
                &expected_channel,
            )
            .await
        }
    }
    mod inner {
        use super::*;

        #[async_trait]
        pub trait Inner: StateExt {
            async fn verify_connections_open(
                &self,
                msg: &MsgChannelOpenTry,
            ) -> anyhow::Result<ConnectionEnd> {
                let connection = self
                    .get_connection(&msg.channel.connection_hops[0])
                    .await?
                    .ok_or_else(|| anyhow::anyhow!("connection not found"))?;

                if connection.state != ConnectionState::Open {
                    Err(anyhow::anyhow!("connection for channel is not open"))
                } else {
                    Ok(connection)
                }
            }
        }
        impl<T: StateExt> Inner for T {}
    }
    impl<T: StateExt> ChannelOpenTryCheck for T {}
}

pub mod channel_open_ack {
    use super::super::*;
    use super::proof_verification::ChannelProofVerifier;

    fn channel_state_is_correct(channel: &ChannelEnd) -> anyhow::Result<()> {
        if channel.state == ChannelState::Init || channel.state == ChannelState::TryOpen {
            Ok(())
        } else {
            Err(anyhow::anyhow!("channel is not in the correct state"))
        }
    }

    #[async_trait]
    pub trait ChannelOpenAckCheck: StateExt + inner::Inner {
        async fn validate(&self, msg: &MsgChannelOpenAck) -> anyhow::Result<()> {
            let channel = self
                .get_channel(&msg.channel_id, &msg.port_id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("channel not found"))?;

            channel_state_is_correct(&channel)?;

            // TODO: capability authentication?

            let connection = self.verify_channel_connection_open(&channel).await?;

            let expected_counterparty =
                Counterparty::new(msg.port_id.clone(), Some(msg.channel_id));

            let expected_connection_hops = vec![connection
                .counterparty()
                .connection_id
                .clone()
                .ok_or_else(|| anyhow::anyhow!("no counterparty connection id provided"))?];

            let expected_channel = ChannelEnd {
                state: ChannelState::TryOpen,
                ordering: channel.ordering,
                remote: expected_counterparty,
                connection_hops: expected_connection_hops,
                version: msg.counterparty_version.clone(),
            };

            self.verify_channel_proof(
                &connection,
                &msg.proofs,
                &msg.counterparty_channel_id,
                &channel.remote.port_id,
                &expected_channel,
            )
            .await
        }
    }
    mod inner {
        use super::*;

        #[async_trait]
        pub trait Inner: StateExt {
            async fn verify_channel_connection_open(
                &self,
                channel: &ChannelEnd,
            ) -> anyhow::Result<ConnectionEnd> {
                let connection = self
                    .get_connection(&channel.connection_hops[0])
                    .await?
                    .ok_or_else(|| anyhow::anyhow!("connection not found for channel"))?;

                if connection.state != ConnectionState::Open {
                    Err(anyhow::anyhow!("connection for channel is not open"))
                } else {
                    Ok(connection)
                }
            }
        }
        impl<T: StateExt> Inner for T {}
    }

    impl<T: StateExt> ChannelOpenAckCheck for T {}
}

pub mod channel_open_confirm {
    use super::super::*;
    use super::proof_verification::ChannelProofVerifier;

    #[async_trait]
    pub trait ChannelOpenConfirmCheck: StateExt {
        async fn validate(&self, msg: &MsgChannelOpenConfirm) -> anyhow::Result<()> {
            let channel = self
                .get_channel(&msg.channel_id, &msg.port_id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("channel not found"))?;
            if !channel.state_matches(&ChannelState::TryOpen) {
                return Err(anyhow::anyhow!("channel is not in the correct state"));
            }

            // TODO: capability authentication?

            let connection = self
                .get_connection(&channel.connection_hops[0])
                .await?
                .ok_or_else(|| anyhow::anyhow!("connection not found for channel"))?;
            if !connection.state_matches(&ConnectionState::Open) {
                return Err(anyhow::anyhow!("connection for channel is not open"));
            }

            let expected_connection_hops = vec![connection
                .counterparty()
                .connection_id
                .clone()
                .ok_or_else(|| anyhow::anyhow!("no counterparty connection id provided"))?];

            let expected_counterparty =
                Counterparty::new(msg.port_id.clone(), Some(msg.channel_id));

            let expected_channel = ChannelEnd {
                state: ChannelState::Open,
                ordering: channel.ordering,
                remote: expected_counterparty,
                connection_hops: expected_connection_hops,
                version: channel.version.clone(),
            };

            self.verify_channel_proof(
                &connection,
                &msg.proofs,
                &channel
                    .remote
                    .channel_id
                    .ok_or_else(|| anyhow::anyhow!("no channel id"))?,
                &channel.remote.port_id,
                &expected_channel,
            )
            .await
        }
    }

    impl<T: StateExt> ChannelOpenConfirmCheck for T {}
}

pub mod channel_close_init {
    use super::super::*;

    #[async_trait]
    pub trait ChannelCloseInitCheck: StateExt {
        async fn validate(&self, msg: &MsgChannelCloseInit) -> anyhow::Result<()> {
            // TODO: capability authentication?
            //
            // we probably do need capability authentication here, or some other authorization
            // method, to prevent anyone from spuriously closing channels.
            //
            let channel = self
                .get_channel(&msg.channel_id, &msg.port_id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("channel not found"))?;
            if channel.state_matches(&ChannelState::Closed) {
                return Err(anyhow::anyhow!("channel is already closed"));
            }

            let connection = self
                .get_connection(&channel.connection_hops[0])
                .await?
                .ok_or_else(|| anyhow::anyhow!("connection not found for channel"))?;
            if !connection.state_matches(&ConnectionState::Open) {
                return Err(anyhow::anyhow!("connection for channel is not open"));
            }

            Ok(())
        }
    }

    impl<T: StateExt> ChannelCloseInitCheck for T {}
}

pub mod channel_close_confirm {
    use super::super::*;
    use super::proof_verification::ChannelProofVerifier;

    #[async_trait]
    pub trait ChannelCloseConfirmCheck: StateExt {
        async fn validate(&self, msg: &MsgChannelCloseConfirm) -> anyhow::Result<()> {
            // TODO: capability authentication?
            //
            // we probably do need capability authentication here, or some other authorization
            // method, to prevent anyone from spuriously closing channels.
            //
            let channel = self
                .get_channel(&msg.channel_id, &msg.port_id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("channel not found"))?;
            if channel.state_matches(&ChannelState::Closed) {
                return Err(anyhow::anyhow!("channel is already closed"));
            }

            let connection = self
                .get_connection(&channel.connection_hops[0])
                .await?
                .ok_or_else(|| anyhow::anyhow!("connection not found for channel"))?;
            if !connection.state_matches(&ConnectionState::Open) {
                return Err(anyhow::anyhow!("connection for channel is not open"));
            }

            let expected_connection_hops = vec![connection
                .counterparty()
                .connection_id
                .clone()
                .ok_or_else(|| anyhow::anyhow!("no counterparty connection id provided"))?];

            let expected_counterparty =
                Counterparty::new(msg.port_id.clone(), Some(msg.channel_id));

            let expected_channel = ChannelEnd {
                state: ChannelState::Closed,
                ordering: channel.ordering,
                remote: expected_counterparty,
                connection_hops: expected_connection_hops,
                version: channel.version.clone(),
            };

            self.verify_channel_proof(
                &connection,
                &msg.proofs,
                &channel
                    .remote
                    .channel_id
                    .ok_or_else(|| anyhow::anyhow!("no channel id"))?,
                &channel.remote.port_id,
                &expected_channel,
            )
            .await
        }
    }

    impl<T: StateExt> ChannelCloseConfirmCheck for T {}
}
