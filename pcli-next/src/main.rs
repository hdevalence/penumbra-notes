// Rust analyzer complains without this (but rustc is happy regardless)
#![recursion_limit = "256"]
#![allow(clippy::clone_on_copy)]
use std::{net::SocketAddr, path::PathBuf};

use anyhow::{Context, Result};
use directories::ProjectDirs;
use futures::StreamExt;
use penumbra_crypto::{keys::SpendKey, FullViewingKey};
use penumbra_custody::{CustodyClient, SoftHSM};
use penumbra_proto::{
    custody::{
        custody_protocol_client::CustodyProtocolClient,
        custody_protocol_server::CustodyProtocolServer,
    },
    view::{view_protocol_client::ViewProtocolClient, view_protocol_server::ViewProtocolServer},
};
use penumbra_view::{ViewClient, ViewService};
use structopt::StructOpt;

mod command;
mod legacy;
mod network;
mod wallet;
mod warning;

use wallet::Wallet;

const CUSTODY_FILE_NAME: &'static str = "custody.json";
const VIEW_FILE_NAME: &'static str = "pcli-view.sqlite";

use command::*;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "pcli-next",
    about = "The Penumbra command-line interface.",
    version = env!("VERGEN_GIT_SEMVER"),
)]
pub struct Opt {
    /// The address of the pd+tendermint node.
    #[structopt(short, long, default_value = "testnet.penumbra.zone")]
    pub node: String,
    /// The port to use to speak to tendermint's RPC server.
    #[structopt(long, default_value = "26657")]
    pub tendermint_port: u16,
    /// The port to use to speak to pd's gRPC server.
    #[structopt(long, default_value = "8080")]
    pub pd_port: u16,
    #[structopt(subcommand)]
    pub cmd: Command,
    /// The directory to store the wallet and view data in [default: platform appdata directory]
    #[structopt(short, long)]
    pub data_path: Option<String>,
    /// If set, use a remote view service instead of local synchronization.
    #[structopt(short, long)]
    pub view_address: Option<SocketAddr>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Display a warning message to the user so they don't get upset when all their tokens are lost.
    if std::env::var("PCLI_UNLEASH_DANGER").is_err() {
        warning::display();
    }

    tracing_subscriber::fmt::init();
    let opt = Opt::from_args();

    let default_data_dir = ProjectDirs::from("zone", "penumbra", "pcli")
        .context("Failed to get platform data dir")?
        .data_dir()
        .to_path_buf();
    let data_dir = opt
        .data_path
        .as_ref()
        .map(|s| PathBuf::from(s))
        .unwrap_or(default_data_dir);

    // Create the data directory if it is missing.
    std::fs::create_dir_all(&data_dir).context("Failed to create data directory")?;

    let custody_path = data_dir.join(CUSTODY_FILE_NAME);
    let view_path = data_dir.join(VIEW_FILE_NAME);

    let legacy_wallet_path = data_dir.join(legacy::WALLET_FILE_NAME);

    // Try to auto-migrate the legacy wallet file to the new location, if:
    // - the legacy wallet file exists
    // - the new wallet file does not exist
    if legacy_wallet_path.exists() && !custody_path.exists() {
        legacy::migrate(&legacy_wallet_path, &custody_path)?;
    }

    // The wallet command takes the data dir directly, since it may need to
    // create the client state, so handle it specially here so that we can have
    // common code for the other subcommands.
    if let Command::Wallet(wallet_cmd) = &opt.cmd {
        wallet_cmd.exec(data_dir)?;
        return Ok(());
    }

    // Otherwise, build the custody service...
    let wallet = Wallet::load(custody_path)?;
    let soft_hsm = SoftHSM::new(vec![wallet.spend_key.clone()]);
    let custody = CustodyProtocolClient::new(CustodyProtocolServer::new(soft_hsm));

    let fvk = wallet.spend_key.full_viewing_key().clone();
    // ...and the view service...
    if let Some(view_address) = opt.view_address {
        // Use a remote view service.
        let view = ViewProtocolClient::connect(format!("http://{}", view_address)).await?;

        main_inner(opt, wallet.spend_key, fvk, view, custody).await
    } else {
        // Use an in-memory view service.
        let mut oc_client = opt.oblivious_client().await?;
        let view_storage = penumbra_view::Storage::load_or_initialize(
            view_path
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("Non-UTF8 view path"))?
                .to_string(),
            &fvk,
            &mut oc_client,
        )
        .await?;
        let view_service = ViewService::new(
            view_storage,
            oc_client,
            opt.node.clone(),
            opt.tendermint_port,
        )
        .await?;

        // Now build the view and custody clients, doing gRPC with ourselves
        let view = ViewProtocolClient::new(ViewProtocolServer::new(view_service));

        main_inner(opt, wallet.spend_key, fvk, view, custody).await
    }
}

/// The rest of the main function is split into a separate function so that we
/// can erase the `ViewProtocolClient` and `CustodyProtocolClient` types.
async fn main_inner<V: ViewClient, C: CustodyClient>(
    opt: Opt,
    // TODO: remove this
    sk: SpendKey,
    fvk: FullViewingKey,
    mut view: V,
    mut custody: C,
) -> Result<()> {
    //let viewservice2 = view_service.clone();

    if opt.cmd.needs_sync() {
        // this has to manually invoke the method on the "domain trait" because we haven't
        // forgotten the concrete type, which has a method of the same name.
        let mut status_stream = ViewClient::status_stream(&mut view, fvk.hash()).await?;

        // Pull out the first message from the stream, which has the current state, and use
        // it to set up a progress bar.
        let initial_status = status_stream
            .next()
            .await
            .transpose()?
            .ok_or_else(|| anyhow::anyhow!("view service did not report sync status"))?;

        println!(
            "Scanning blocks from last sync height {} to latest height {}",
            initial_status.sync_height, initial_status.latest_known_block_height,
        );

        use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
        let progress_bar = ProgressBar::with_draw_target(
            initial_status.latest_known_block_height - initial_status.sync_height,
            ProgressDrawTarget::stdout(),
        )
        .with_style(
            ProgressStyle::default_bar()
                .template("[{elapsed}] {bar:50.cyan/blue} {pos:>7}/{len:7} {per_sec} ETA: {eta}"),
        );

        while let Some(status) = status_stream.next().await.transpose()? {
            progress_bar.set_position(status.sync_height - initial_status.sync_height);
        }
        progress_bar.finish();
    }

    // TODO: this is a mess, figure out the right way to bundle up the clients + fvk
    // make sure to be compatible with client for remote view service, with different
    // concrete type

    match &opt.cmd {
        Command::Wallet(_) => unreachable!("wallet command already executed"),
        Command::Sync => {
            // We have already synchronized the wallet above, so we can just return.
        }
        Command::Tx(tx_cmd) => tx_cmd.exec(&opt, &fvk, &mut view, &mut custody).await?,
        Command::Addr(addr_cmd) => addr_cmd.exec(&fvk)?,
        Command::Balance(balance_cmd) => balance_cmd.exec(&fvk, &mut view).await?,
        Command::Validator(cmd) => cmd.exec(&opt, &sk, &mut view, &mut custody).await?,
        Command::Stake(cmd) => cmd.exec(&opt, &fvk, &mut view, &mut custody).await?,
        Command::Chain(cmd) => cmd.exec(&opt, &fvk, &mut view).await?,
    }

    Ok(())
}
