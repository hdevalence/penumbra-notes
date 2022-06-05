// Many of the IBC message types are enums, where the number of variants differs
// depending on the build configuration, meaning that the fallthrough case gets
// marked as unreachable only when not building in test configuration.
#![allow(unreachable_patterns)]

mod channel;
mod client;
mod connection;
pub(crate) mod state_key;

use crate::{Component, Context};
use anyhow::Result;
use async_trait::async_trait;
use client::Ics2Client;
use penumbra_chain::{genesis, View as _};
use penumbra_storage::State;
use penumbra_transaction::Transaction;
use tendermint::abci;
use tracing::instrument;

pub struct IBCComponent {
    client: client::Ics2Client,
    connection: connection::ConnectionComponent,
    channel: channel::ICS4Channel,

    state: State,
}

impl IBCComponent {
    #[instrument(name = "ibc", skip(state))]
    pub async fn new(state: State) -> Self {
        let client = Ics2Client::new(state.clone()).await;
        let connection = connection::ConnectionComponent::new(state.clone()).await;
        let channel = channel::ICS4Channel::new(state.clone()).await;

        Self {
            channel,
            client,
            connection,

            state: state.clone(),
        }
    }
}

#[async_trait]
impl Component for IBCComponent {
    #[instrument(name = "ibc", skip(self, app_state))]
    async fn init_chain(&mut self, app_state: &genesis::AppState) {
        self.client.init_chain(app_state).await;
        self.connection.init_chain(app_state).await;
        self.channel.init_chain(app_state).await;
    }

    #[instrument(name = "ibc", skip(self, begin_block, ctx))]
    async fn begin_block(&mut self, ctx: Context, begin_block: &abci::request::BeginBlock) {
        self.client.begin_block(ctx.clone(), begin_block).await;
        self.connection.begin_block(ctx.clone(), begin_block).await;
        self.channel.begin_block(ctx.clone(), begin_block).await;
    }

    #[instrument(name = "ibc", skip(tx, ctx))]
    fn check_tx_stateless(ctx: Context, tx: &Transaction) -> Result<()> {
        client::Ics2Client::check_tx_stateless(ctx.clone(), tx)?;
        connection::ConnectionComponent::check_tx_stateless(ctx.clone(), tx)?;
        channel::ICS4Channel::check_tx_stateless(ctx, tx)?;

        Ok(())
    }

    #[instrument(name = "ibc", skip(self, ctx, tx))]
    async fn check_tx_stateful(&self, ctx: Context, tx: &Transaction) -> Result<()> {
        if tx.ibc_actions().count() > 0 && !self.state.get_chain_params().await?.ibc_enabled {
            return Err(anyhow::anyhow!(
                "transaction contains IBC actions, but IBC is not enabled"
            ));
        }

        self.client.check_tx_stateful(ctx.clone(), tx).await?;
        self.connection.check_tx_stateful(ctx.clone(), tx).await?;
        self.channel.check_tx_stateful(ctx.clone(), tx).await?;

        Ok(())
    }

    #[instrument(name = "ibc", skip(self, ctx, tx))]
    async fn execute_tx(&mut self, ctx: Context, tx: &Transaction) {
        self.client.execute_tx(ctx.clone(), tx).await;
        self.connection.execute_tx(ctx.clone(), tx).await;
        self.channel.execute_tx(ctx.clone(), tx).await;
    }

    #[instrument(name = "ibc", skip(self, ctx, end_block))]
    async fn end_block(&mut self, ctx: Context, end_block: &abci::request::EndBlock) {
        self.client.end_block(ctx.clone(), end_block).await;
        self.connection.end_block(ctx.clone(), end_block).await;
        self.channel.end_block(ctx.clone(), end_block).await;
    }
}
