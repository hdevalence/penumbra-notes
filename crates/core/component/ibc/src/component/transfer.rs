use std::str::FromStr;

use anyhow::{Context, Result};
use async_trait::async_trait;
use ibc_types::{
    core::channel::{
        channel::Order as ChannelOrder,
        msgs::{
            MsgAcknowledgement, MsgChannelCloseConfirm, MsgChannelCloseInit, MsgChannelOpenAck,
            MsgChannelOpenConfirm, MsgChannelOpenInit, MsgChannelOpenTry, MsgRecvPacket,
            MsgTimeout,
        },
        ChannelId, PortId, Version,
    },
    transfer::acknowledgement::TokenTransferAcknowledgement,
};
use penumbra_asset::{asset, asset::DenomMetadata, Value};
use penumbra_keys::Address;
use penumbra_num::Amount;
use penumbra_proto::{
    core::ibc::v1alpha1::FungibleTokenPacketData, StateReadProto, StateWriteProto,
};
use penumbra_shielded_pool::component::{NoteManager, SupplyWrite};
use penumbra_storage::{StateRead, StateWrite};
use prost::Message;

use crate::{
    component::{
        app_handler::{AppHandler, AppHandlerCheck, AppHandlerExecute},
        packet::{
            IBCPacket, SendPacketRead as _, SendPacketWrite as _, Unchecked,
            WriteAcknowledgement as _,
        },
        state_key,
    },
    Ics20Withdrawal,
};

// returns a bool indicating if the provided denom was issued locally or if it was bridged in.
// this logic is a bit tricky, and adapted from https://github.com/cosmos/ibc/tree/main/spec/app/ics-020-fungible-token-transfer (sendFungibleTokens).
//
// what we want to do is to determine if the denom being withdrawn is a native token (one
// that originates from Penumbra) or a bridged token (one that was sent into penumbra from
// IBC).
//
// A simple way of doing this is by parsing the denom, looking for a prefix that is only
// appended in the case of a bridged token. That is what this logic does.
fn is_source(source_port: &PortId, source_channel: &ChannelId, denom: &DenomMetadata) -> bool {
    let prefix = format!("{source_port}/{source_channel}/");

    denom.starts_with(&prefix)
}

#[derive(Clone)]
pub struct Ics20Transfer {}

#[async_trait]
pub trait Ics20TransferReadExt: StateRead {
    async fn withdrawal_check(&self, withdrawal: &Ics20Withdrawal) -> Result<()> {
        // create packet
        let packet: IBCPacket<Unchecked> = withdrawal.clone().into();

        // send packet
        self.send_packet_check(packet).await?;

        Ok(())
    }
}

impl<T: StateRead + ?Sized> Ics20TransferReadExt for T {}

#[async_trait]
pub trait Ics20TransferWriteExt: StateWrite {
    async fn withdrawal_execute(&mut self, withdrawal: &Ics20Withdrawal) {
        // create packet, assume it's already checked since the component caller contract calls `check` before `execute`
        let checked_packet = IBCPacket::<Unchecked>::from(withdrawal.clone()).assume_checked();

        let prefix = format!("transfer/{}/", &withdrawal.source_channel);
        if !withdrawal.denom.starts_with(&prefix) {
            // we are the source. add the value balance to the escrow channel.
            let existing_value_balance: Amount = self
                .get(&state_key::ics20_value_balance(
                    &withdrawal.source_channel,
                    &withdrawal.denom.id(),
                ))
                .await
                .unwrap()
                .unwrap_or_else(Amount::zero);

            let new_value_balance = existing_value_balance + withdrawal.amount;
            self.put(
                state_key::ics20_value_balance(&withdrawal.source_channel, &withdrawal.denom.id()),
                new_value_balance,
            );
        } else {
            // receiver is the source, burn utxos

            // NOTE: this burning should already be accomplished by the value balance check from
            // the withdrawal's balance commitment, so nothing to do here.
        }

        self.send_packet_execute(checked_packet).await;
    }
}

impl<T: StateWrite + ?Sized> Ics20TransferWriteExt for T {}

// TODO: Ics20 implementation.
// see: https://github.com/cosmos/ibc/tree/master/spec/app/ics-020-fungible-token-transfer
// TODO (ava): add versioning to AppHandlers
#[async_trait]
impl AppHandlerCheck for Ics20Transfer {
    async fn chan_open_init_check<S: StateRead>(_state: S, msg: &MsgChannelOpenInit) -> Result<()> {
        if msg.ordering != ChannelOrder::Unordered {
            anyhow::bail!("channel order must be unordered for Ics20 transfer");
        }
        let ics20_version = Version::new("ics20-1".to_string());
        if msg.version_proposal != ics20_version {
            anyhow::bail!("channel version must be ics20 for Ics20 transfer");
        }

        Ok(())
    }

    async fn chan_open_try_check<S: StateRead>(_state: S, msg: &MsgChannelOpenTry) -> Result<()> {
        if msg.ordering != ChannelOrder::Unordered {
            anyhow::bail!("channel order must be unordered for Ics20 transfer");
        }
        let ics20_version = Version::new("ics20-1".to_string());

        if msg.version_supported_on_a != ics20_version {
            anyhow::bail!("counterparty version must be ics20-1 for Ics20 transfer");
        }

        Ok(())
    }

    async fn chan_open_ack_check<S: StateRead>(_state: S, msg: &MsgChannelOpenAck) -> Result<()> {
        let ics20_version = Version::new("ics20-1".to_string());
        if msg.version_on_b != ics20_version {
            anyhow::bail!("counterparty version must be ics20-1 for Ics20 transfer");
        }

        Ok(())
    }

    async fn chan_open_confirm_check<S: StateRead>(
        _state: S,
        _msg: &MsgChannelOpenConfirm,
    ) -> Result<()> {
        // accept channel confirmations, port has already been validated, version has already been validated
        Ok(())
    }

    async fn chan_close_confirm_check<S: StateRead>(
        _state: S,
        _msg: &MsgChannelCloseConfirm,
    ) -> Result<()> {
        // no action necessary
        Ok(())
    }

    async fn chan_close_init_check<S: StateRead>(
        _state: S,
        _msg: &MsgChannelCloseInit,
    ) -> Result<()> {
        // always abort transaction
        anyhow::bail!("ics20 always aborts on close init");
    }

    async fn recv_packet_check<S: StateRead>(_state: S, _msg: &MsgRecvPacket) -> Result<()> {
        // all checks on recv_packet done in execute
        Ok(())
    }

    async fn timeout_packet_check<S: StateRead>(state: S, msg: &MsgTimeout) -> Result<()> {
        let packet_data = FungibleTokenPacketData::decode(msg.packet.data.as_slice())?;
        let denom: asset::DenomMetadata = packet_data.denom.as_str().try_into()?;

        if is_source(&msg.packet.port_on_a, &msg.packet.chan_on_a, &denom) {
            // check if we have enough balance to refund tokens to sender
            let value_balance: Amount = state
                .get(&state_key::ics20_value_balance(
                    &msg.packet.chan_on_a,
                    &denom.id(),
                ))
                .await?
                .unwrap_or_else(Amount::zero);

            let amount_penumbra: Amount = packet_data.amount.try_into()?;
            if value_balance < amount_penumbra {
                anyhow::bail!("insufficient balance to refund tokens to sender");
            }
        }

        Ok(())
    }

    async fn acknowledge_packet_check<S: StateRead>(
        _state: S,
        _msg: &MsgAcknowledgement,
    ) -> Result<()> {
        Ok(())
    }
}

// the main entry point for ICS20 transfer packet handling
async fn recv_transfer_packet_inner<S: StateWrite>(
    mut state: S,
    msg: &MsgRecvPacket,
) -> Result<()> {
    // parse if we are source or dest, and mint or burn accordingly
    //
    // see this part of the spec for this logic:
    //
    // https://github.com/cosmos/ibc/tree/main/spec/app/ics-020-fungible-token-transfer (onRecvPacket)
    //
    let packet_data = FungibleTokenPacketData::decode(msg.packet.data.as_slice())?;
    let denom: asset::DenomMetadata = packet_data
        .denom
        .as_str()
        .try_into()
        .context("couldnt decode denom in ICS20 transfer")?;
    let receiver_amount: Amount = packet_data
        .amount
        .try_into()
        .context("couldnt decode amount in ICS20 transfer")?;
    let receiver_address = Address::from_str(&packet_data.receiver)?;

    // NOTE: here we assume we are chain A.

    // 2. check if we are the source chain for the denom.
    if is_source(&msg.packet.port_on_a, &msg.packet.chan_on_a, &denom) {
        // mint tokens to receiver in the amount of packet_data.amount in the denom of denom (with
        // the source removed, since we're the source)
        let prefix = format!(
            "{source_port}/{source_chan}/",
            source_port = msg.packet.port_on_a,
            source_chan = msg.packet.chan_on_a
        );

        let unprefixed_denom: asset::DenomMetadata = packet_data
            .denom
            .replace(&prefix, "")
            .as_str()
            .try_into()
            .context("couldnt decode denom in ICS20 transfer")?;

        let value: Value = Value {
            amount: receiver_amount,
            asset_id: unprefixed_denom.id(),
        };

        // assume AppHandlerCheck has already been called, and we have enough balance to mint tokens to receiver
        // check if we have enough balance to unescrow tokens to receiver
        let value_balance: Amount = state
            .get(&state_key::ics20_value_balance(
                &msg.packet.chan_on_b,
                &unprefixed_denom.id(),
            ))
            .await?
            .unwrap_or_else(Amount::zero);

        if value_balance < receiver_amount {
            // error text here is from the ics20 spec
            anyhow::bail!("transfer coins failed");
        }

        state
            .mint_note(
                value,
                &receiver_address,
                penumbra_chain::NoteSource::Ics20Transfer, // TODO
            )
            .await
            .unwrap();

        // update the value balance
        let value_balance: Amount = state
            .get(&state_key::ics20_value_balance(
                &msg.packet.chan_on_b,
                &unprefixed_denom.id(),
            ))
            .await?
            .unwrap_or_else(Amount::zero);

        // note: this arithmetic was checked above, but we do it again anyway.
        let new_value_balance = value_balance.checked_sub(&receiver_amount).unwrap();
        state.put(
            state_key::ics20_value_balance(&msg.packet.chan_on_b, &denom.id()),
            new_value_balance,
        );
    } else {
        // create new denom:
        //
        // prefix = "{packet.destPort}/{packet.destChannel}/"
        // prefixedDenomination = prefix + data.denom
        //
        // then mint that denom to packet_data.receiver in packet_data.amount
        // no value balance to update here since this is an exogenous denom
        let prefixed_denomination = format!(
            "{}/{}/{}",
            msg.packet.port_on_b, msg.packet.chan_on_b, packet_data.denom
        );

        let denom: asset::DenomMetadata = prefixed_denomination.as_str().try_into().unwrap();
        state.register_denom(&denom).await.unwrap();

        let value = Value {
            amount: receiver_amount,
            asset_id: denom.id(),
        };

        state
            .mint_note(
                value,
                &receiver_address,
                penumbra_chain::NoteSource::Ics20Transfer,
            )
            .await
            .context("failed to mint notes in ibc transfer")?;
    }

    Ok(())
}

// see: https://github.com/cosmos/ibc/blob/8326e26e7e1188b95c32481ff00348a705b23700/spec/app/ics-020-fungible-token-transfer/README.md?plain=1#L297
async fn timeout_packet_inner<S: StateWrite>(mut state: S, msg: &MsgTimeout) -> Result<()> {
    let packet_data = FungibleTokenPacketData::decode(msg.packet.data.as_slice())?;
    let denom: asset::DenomMetadata = packet_data // CRITICAL: verify that this denom is validated in upstream timeout handling
        .denom
        .as_str()
        .try_into()
        .context("couldn't decode denom in ics20 transfer timeout")?;
    // receiver was source chain, mint vouchers back to sender
    let amount: Amount = packet_data
        .amount
        .try_into()
        .context("couldn't decode amount in ics20 transfer timeout")?;

    let receiver = Address::from_str(&packet_data.receiver)
        .context("couldn't decode receiver address in ics20 timeout")?;

    let value: Value = Value {
        amount,
        asset_id: denom.id(),
    };

    if is_source(&msg.packet.port_on_a, &msg.packet.chan_on_a, &denom) {
        // sender was source chain, unescrow tokens back to sender
        let value_balance: Amount = state
            .get(&state_key::ics20_value_balance(
                &msg.packet.chan_on_a,
                &denom.id(),
            ))
            .await?
            .unwrap_or_else(Amount::zero);

        if value_balance < amount {
            anyhow::bail!("couldn't return coins in timeout: not enough value balance");
        }

        state
            .mint_note(value, &receiver, penumbra_chain::NoteSource::Ics20Transfer)
            .await
            .context("couldn't mint note in timeout_packet_inner")?;

        // update the value balance
        let value_balance: Amount = state
            .get(&state_key::ics20_value_balance(
                &msg.packet.chan_on_a,
                &denom.id(),
            ))
            .await?
            .unwrap_or_else(Amount::zero);

        // note: this arithmetic was checked above, but we do it again anyway.
        let new_value_balance = value_balance.checked_sub(&amount).unwrap();
        state.put(
            state_key::ics20_value_balance(&msg.packet.chan_on_a, &denom.id()),
            new_value_balance,
        );
    } else {
        state
            .mint_note(value, &receiver, penumbra_chain::NoteSource::Ics20Transfer) // NOTE: should this be Ics20TransferTimeout?
            .await
            .context("failed to mint return voucher in ics20 transfer timeout")?;
    }

    Ok(())
}

// NOTE: should these be fallible, now that our enclosing state machine is fallible in execution?
#[async_trait]
impl AppHandlerExecute for Ics20Transfer {
    async fn chan_open_init_execute<S: StateWrite>(_state: S, _msg: &MsgChannelOpenInit) {}
    async fn chan_open_try_execute<S: StateWrite>(_state: S, _msg: &MsgChannelOpenTry) {}
    async fn chan_open_ack_execute<S: StateWrite>(_state: S, _msg: &MsgChannelOpenAck) {}
    async fn chan_open_confirm_execute<S: StateWrite>(_state: S, _msg: &MsgChannelOpenConfirm) {}
    async fn chan_close_confirm_execute<S: StateWrite>(_state: S, _msg: &MsgChannelCloseConfirm) {}
    async fn chan_close_init_execute<S: StateWrite>(_state: S, _msg: &MsgChannelCloseInit) {}
    async fn recv_packet_execute<S: StateWrite>(mut state: S, msg: &MsgRecvPacket) {
        // recv packet should never fail a transaction, but it should record a failure acknowledgement.
        let ack: Vec<u8> = match recv_transfer_packet_inner(&mut state, msg).await {
            Ok(_) => {
                // record packet acknowledgement without error
                TokenTransferAcknowledgement::success().into()
            }
            Err(e) => {
                tracing::debug!("couldnt execute transfer: {}", e);
                // record packet acknowledgement with error
                TokenTransferAcknowledgement::Error(e.to_string()).into()
            }
        };

        state
            .write_acknowledgement(&msg.packet, &ack)
            .await
            .context("critical: failed to write acknowledgement")
            .unwrap();
    }

    async fn timeout_packet_execute<S: StateWrite>(mut state: S, msg: &MsgTimeout) {
        // timeouts should never fail
        timeout_packet_inner(&mut state, msg)
            .await
            .context("critical: failed to timeout packet")
            .unwrap();
    }

    async fn acknowledge_packet_execute<S: StateWrite>(_state: S, _msg: &MsgAcknowledgement) {}
}

impl AppHandler for Ics20Transfer {}
