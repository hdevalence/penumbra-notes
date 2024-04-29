use super::FeeTier;
use crate::App;
use anyhow::{anyhow, bail, Context};
use clap::Subcommand;
use penumbra_asset::Value;
use penumbra_auction::auction::{dutch::DutchAuction, AuctionId};
use penumbra_dex::lp::position::Position;
use penumbra_fee::state_key::gas_prices;
use penumbra_keys::keys::AddressIndex;
use penumbra_proto::{view::v1::GasPricesRequest, DomainType, Name};
use penumbra_view::SpendableNoteRecord;
use penumbra_wallet::plan::Planner;
use rand::RngCore;
use rand_core::OsRng;

#[derive(Debug, Subcommand)]
pub enum AuctionCmd {
    /// Commands related to Dutch auctions
    #[clap(display_order = 100, subcommand)]
    Dutch(DutchCmd),
}

/// Commands related to Dutch auctions
#[derive(Debug, Subcommand)]
pub enum DutchCmd {
    /// Schedule a Dutch auction, a tool to help accomplish price discovery.
    #[clap(display_order = 100, name = "schedule")]
    DutchAuctionSchedule {
        /// Source account initiating the auction.
        #[clap(long, display_order = 100, default_value = "0")]
        source: u32,
        /// The value the seller wishes to auction.
        #[clap(long, display_order = 200)]
        input: String,
        /// The maximum output the seller can receive.
        ///
        /// This implicitly defines the starting price for the auction.
        #[clap(long, display_order = 400)]
        max_output: String,
        /// The minimum output the seller is willing to receive.
        ///
        /// This implicitly defines the ending price for the auction.
        #[clap(long, display_order = 500)]
        min_output: String,
        /// The block height at which the auction begins.
        ///
        /// This allows the seller to schedule an auction at a future time.
        #[clap(long, display_order = 600)]
        start_height: u64,
        /// The block height at which the auction ends.
        ///
        /// Together with `start_height`, `max_output`, and `min_output`,
        /// this implicitly defines the speed of the auction.
        #[clap(long, display_order = 700)]
        end_height: u64,
        /// The number of discrete price steps to use for the auction.
        ///
        /// `end_height - start_height` must be a multiple of `step_count`.
        #[clap(long, display_order = 800)]
        step_count: u64,
        /// The selected fee tier to multiply the fee amount by.
        #[clap(short, long, value_enum, default_value_t, display_order = 1000)]
        fee_tier: FeeTier,
    },
    /// Terminate a Dutch auction.
    #[clap(display_order = 300, name = "end")]
    DutchAuctionEnd {
        /// Source account terminating the auction.
        #[clap(long, display_order = 100, default_value = "0")]
        source: u32,
        /// Identifier of the auction.
        #[clap(long, display_order = 200)]
        auction_id: String,
        /// The selected fee tier to multiply the fee amount by.
        #[clap(short, long, value_enum, default_value_t, display_order = 300)]
        fee_tier: FeeTier,
    },
    /// Withdraw a Dutch auction, and claim its reserves.
    #[clap(display_order = 200, name = "withdraw")]
    DutchAuctionWithdraw {
        /// Source account withdrawing from the auction.
        #[clap(long, display_order = 100)]
        source: u32,
        /// The auction to withdraw funds from.
        #[clap(long, display_order = 200)]
        auction_id: String,
        //    ///  The sequence number of the withdrawal.
        //    #[clap(long, display_order = 300)]
        //    seq: u64,
        //    /// The amount of the input asset directly owned by the auction.
        //    ///
        //    /// The auction may also own the input asset indirectly,
        //    /// via the reserves of `current_position` if it exists.
        //    #[clap(long, display_order = 400)]
        //    reserves_input: String,
        //    /// The amount of the output asset directly owned by the auction.
        //    ///
        //    /// The auction may also own the output asset indirectly,
        //    /// via the reserves of `current_position` if it exists.
        //    #[clap(long, display_order = 500)]
        //    reserves_output: String,
        /// The selected fee tier to multiply the fee amount by.
        #[clap(short, long, value_enum, default_value_t, display_order = 600)]
        fee_tier: FeeTier,
    },
}

impl DutchCmd {
    /// Process the command by performing the appropriate action.
    pub async fn exec(&self, app: &mut App) -> anyhow::Result<()> {

        match self {
            DutchCmd::DutchAuctionSchedule {
                source,
                input,
                max_output,
                min_output,
                start_height,
                end_height,
                step_count,
                fee_tier,
            } => {
                // let input = input.parse::<Value>()?;
                Ok(())
            }
            DutchCmd::DutchAuctionWithdraw {
                source,
                auction_id,
                fee_tier,
            } => {
                Ok(())
            }
            DutchCmd::DutchAuctionEnd {
                auction_id,
                source,
                fee_tier,
            } => {
                Ok(())
            }
        }
    }
}