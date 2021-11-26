use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "pcli",
    about = "The Penumbra command-line interface.",
    version = env!("VERGEN_GIT_SEMVER"),
)]
pub struct Opt {
    /// The address of the pd+tendermint node.
    #[structopt(short, long, default_value = "127.0.0.1")]
    pub node: String,
    /// The port to use to speak to tendermint.
    #[structopt(short, long, default_value = "26657")]
    pub abci_port: u16,
    /// The port to use to speak to pd's light wallet server.
    #[structopt(short, long, default_value = "26666")]
    pub lightwallet_port: u16,
    #[structopt(subcommand)]
    pub cmd: Command,
    /// The location of the wallet file [default: platform appdata directory]
    #[structopt(short, long)]
    pub wallet_location: Option<String>,
}

#[derive(Debug, StructOpt)]
pub enum Command {
    /// Creates a transaction.
    Tx(TxCmd),
    /// Manages the wallet state.
    Wallet(WalletCmd),
    /// Manages addresses.
    Addr(AddrCmd),
    /// Synchronizes the client, privately scanning the chain state.
    ///
    /// `pcli` syncs automatically prior to any action requiring chain state,
    /// but this command can be used to "pre-sync" before interactive use.
    Sync,
    /// Displays the current wallet balance.
    Balance {
        /// If set, breaks down balances by address.
        #[structopt(short, long)]
        by_address: bool,
    },
}

#[derive(Debug, StructOpt)]
pub enum WalletCmd {
    /// Import an existing spend seed.
    Import {
        /// A 32-byte hex string encoding the spend seed.
        spend_seed: String,
    },
    /// Export the spend seed for the wallet.
    Export,
    /// Generate a new spend seed.
    Generate,
    /// Keep the spend seed, but reset all other client state.
    Reset,
    /// Delete the entire wallet permanently.
    Delete,
}

#[derive(Debug, StructOpt)]
pub enum AddrCmd {
    /// List addresses.
    List,
    /// Show the address with the given index.
    Show {
        /// The index of the address to show.
        #[structopt(short, long)]
        index: u32,
        /// If true, emits only the address and not the (local) label for it.
        #[structopt(short, long)]
        addr_only: bool,
    },
    /// Create a new address.
    New {
        /// A freeform label for the address, stored only locally.
        label: String,
    },
}

#[derive(Debug, StructOpt)]
pub enum TxCmd {
    /// Send transaction to the node.
    Send {
        /// Amount to send.
        amount: u64,
        /// Denomination.
        denomination: String,
        /// Destination address.
        address: String,
        /// Fee.
        #[structopt(default_value = "0")]
        fee: u64,
        /// If set, spend funds originally sent to the specified address.
        #[structopt(short, long)]
        source_address_id: Option<u64>,
    },
}
