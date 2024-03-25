use crate::{
    lp::{
        action::{PositionClose, PositionOpen, PositionWithdraw},
        position::{self, Position},
    },
    swap::Swap,
    swap_claim::SwapClaim,
    BatchSwapOutputData, SwapExecution,
};

use penumbra_asset::asset;
use penumbra_num::Amount;
use penumbra_proto::penumbra::core::component::dex::v1 as pb;

pub fn swap(swap: &Swap) -> pb::EventSwap {
    pb::EventSwap {
        trading_pair: Some(swap.body.trading_pair.into()),
        delta_1_i: Some(swap.body.delta_1_i.into()),
        delta_2_i: Some(swap.body.delta_2_i.into()),
        swap_commitment: Some(swap.body.payload.commitment.into()),
    }
}

pub fn swap_claim(swap_claim: &SwapClaim) -> pb::EventSwapClaim {
    pb::EventSwapClaim {
        trading_pair: Some(swap_claim.body.output_data.trading_pair.into()),
        output_1_commitment: Some(swap_claim.body.output_1_commitment.into()),
        output_2_commitment: Some(swap_claim.body.output_2_commitment.into()),
        nullifier: Some(swap_claim.body.nullifier.into()),
    }
}

pub fn position_open(position_open: &PositionOpen) -> pb::EventPositionOpen {
    pb::EventPositionOpen {
        position_id: Some(position_open.position.id().into()),
        trading_pair: Some(position_open.position.phi.pair.into()),
        reserves_1: Some(position_open.position.reserves.r1.into()),
        reserves_2: Some(position_open.position.reserves.r2.into()),
        trading_fee: position_open.position.phi.component.fee,
    }
}

pub fn position_close(action: &PositionClose) -> pb::EventPositionClose {
    // TODO: should we have another event triggered by the position manager for when
    // the position is actually closed?
    pb::EventPositionClose {
        position_id: Some(action.position_id.into()),
    }
}

pub fn position_withdraw(
    position_withdraw: &PositionWithdraw,
    final_position_state: &Position,
) -> pb::EventPositionWithdraw {
    let sequence = if let position::State::Withdrawn { sequence, .. } = final_position_state.state {
        sequence + 1
    } else {
        0
    };
    pb::EventPositionWithdraw {
        position_id: Some(position_withdraw.position_id.into()),
        trading_pair: Some(final_position_state.phi.pair.into()),
        reserves_1: Some(final_position_state.reserves.r1.into()),
        reserves_2: Some(final_position_state.reserves.r2.into()),
        sequence,
    }
}

pub fn position_execution(post_execution_state: Position) -> pb::EventPositionExecution {
    pb::EventPositionExecution {
        position_id: Some(post_execution_state.id().into()),
        trading_pair: Some(post_execution_state.phi.pair.into()),
        reserves_1: Some(post_execution_state.reserves.r1.into()),
        reserves_2: Some(post_execution_state.reserves.r2.into()),
    }
}

pub fn batch_swap(
    bsod: BatchSwapOutputData,
    swap_execution_1_for_2: Option<SwapExecution>,
    swap_execution_2_for_1: Option<SwapExecution>,
) -> pb::EventBatchSwap {
    pb::EventBatchSwap {
        batch_swap_output_data: Some(bsod.into()),
        swap_execution_1_for_2: swap_execution_1_for_2.map(Into::into),
        swap_execution_2_for_1: swap_execution_2_for_1.map(Into::into),
    }
}

pub fn arb_execution(height: u64, swap_execution: SwapExecution) -> pb::EventArbExecution {
    pb::EventArbExecution {
        height,
        swap_execution: Some(swap_execution.into()),
    }
}

pub fn vcb_credit(
    asset_id: asset::Id,
    previous_balance: Amount,
    new_balance: Amount,
) -> pb::EventValueCircuitBreakerCredit {
    pb::EventValueCircuitBreakerCredit {
        asset_id: Some(asset_id.into()),
        previous_balance: Some(previous_balance.into()),
        new_balance: Some(new_balance.into()),
    }
}

pub fn vcb_debit(
    asset_id: asset::Id,
    previous_balance: Amount,
    new_balance: Amount,
) -> pb::EventValueCircuitBreakerDebit {
    pb::EventValueCircuitBreakerDebit {
        asset_id: Some(asset_id.into()),
        previous_balance: Some(previous_balance.into()),
        new_balance: Some(new_balance.into()),
    }
}
