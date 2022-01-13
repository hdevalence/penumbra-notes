use std::net::Ipv4Addr;
use std::net::SocketAddr;
use std::path::PathBuf;

use metrics_exporter_prometheus::PrometheusBuilder;
use pd::{App, State};
use penumbra_crypto::rdsa::{SigningKey, SpendAuth, VerificationKey};
use penumbra_proto::{
    light_wallet::light_wallet_server::LightWalletServer,
    thin_wallet::thin_wallet_server::ThinWalletServer,
};
use penumbra_stake::{FundingStream, Validator};
use rand_core::OsRng;
use structopt::StructOpt;
use tonic::transport::Server;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "pd",
    about = "The Penumbra daemon.",
    version = env!("VERGEN_GIT_SEMVER"),
)]
struct Opt {
    /// Command to run.
    #[structopt(subcommand)]
    cmd: Command,
}

#[derive(Debug, StructOpt)]
enum Command {
    /// Start running the ABCI and wallet services.
    Start {
        /// The URI used to connect to the Postgres database.
        #[structopt(short, long)]
        database_uri: String,
        /// Bind the services to this host.
        #[structopt(short, long, default_value = "127.0.0.1")]
        host: String,
        /// Bind the ABCI server to this port.
        #[structopt(short, long, default_value = "26658")]
        abci_port: u16,
        /// Bind the light wallet service to this port.
        #[structopt(short, long, default_value = "26666")]
        light_wallet_port: u16,
        /// Bind the thin wallet service to this port.
        #[structopt(short, long, default_value = "26667")]
        thin_wallet_port: u16,
        /// Bind the metrics endpoint to this port.
        #[structopt(short, long, default_value = "9000")]
        metrics_port: u16,
    },

    /// Generates a directory structure containing necessary files to run a
    /// testnet based on input configuration.
    GenerateTestnet {
        /// How many validator nodes to create configuration for.
        #[structopt(short, long, default_value = "4")]
        num_validator_nodes: usize,
        /// Number of blocks per epoch.
        #[structopt(short, long, default_value = "60")]
        epoch_duration: u64,
        /// Path to CSV file containing initial allocations.
        #[structopt(
            short,
            long,
            parse(from_os_str),
            default_value = "testnets/004-thelxinoe/allocations.csv"
        )]
        allocations_input_file: PathBuf,
        /// Path to JSON file containing initial validator configs.
        #[structopt(
            short,
            long,
            parse(from_os_str),
            default_value = "testnets/004-thelxinoe/validators.json"
        )]
        validators_input_file: PathBuf,
        /// Path to directory to store output in. Must not exist.
        #[structopt(short, long)]
        output_dir: Option<PathBuf>,
        /// Testnet name, e.g. `penumbra-euporie`
        #[structopt(short, long, default_value = "penumbra-thelxinoe")]
        chain_id: String,
        /// IP Address to start `tendermint` nodes on. Increments by three to make room for `pd` and `postgres` per node.
        #[structopt(short, long, default_value = "192.167.10.2")]
        starting_ip: Ipv4Addr,
    },
}

// Extracted from tonic's remote_addr implementation; we'd like to instrument
// spans with the remote addr at the server level rather than at the individual
// request level, but the hook available to do that gives us an http::Request
// rather than a tonic::Request, so the tonic::Request::remote_addr method isn't
// available.
fn remote_addr(req: &http::Request<()>) -> Option<SocketAddr> {
    use tonic::transport::server::TcpConnectInfo;
    // NOTE: needs to also check TlsConnectInfo if we use TLS
    req.extensions()
        .get::<TcpConnectInfo>()
        .and_then(|i| i.remote_addr())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let opt = Opt::from_args();

    match opt.cmd {
        Command::Start {
            host,
            database_uri,
            abci_port,
            light_wallet_port,
            thin_wallet_port,
            metrics_port,
        } => {
            tracing::info!(
                ?host,
                ?database_uri,
                ?abci_port,
                ?light_wallet_port,
                ?thin_wallet_port,
                "starting pd"
            );
            // Initialize state
            let state = State::connect(&database_uri).await.unwrap();

            let abci_app = App::new(state.clone()).await.unwrap();

            let (consensus, mempool, snapshot, info) = tower_abci::split::service(abci_app, 10);

            let abci_server = tokio::spawn(
                tower_abci::Server::builder()
                    .consensus(consensus)
                    .snapshot(snapshot)
                    .mempool(mempool)
                    .info(info)
                    .finish()
                    .unwrap()
                    .listen(format!("{}:{}", host, abci_port)),
            );

            let light_wallet_server = tokio::spawn(
                Server::builder()
                    .trace_fn(|req| match remote_addr(req) {
                        Some(remote_addr) => tracing::error_span!("light_wallet", ?remote_addr),
                        None => tracing::error_span!("light_wallet"),
                    })
                    .add_service(LightWalletServer::new(state.clone()))
                    .serve(
                        format!("{}:{}", host, light_wallet_port)
                            .parse()
                            .expect("this is a valid address"),
                    ),
            );
            let thin_wallet_server = tokio::spawn(
                Server::builder()
                    .trace_fn(|req| match remote_addr(req) {
                        Some(remote_addr) => tracing::error_span!("thin_wallet", ?remote_addr),
                        None => tracing::error_span!("thin_wallet"),
                    })
                    .add_service(ThinWalletServer::new(state.clone()))
                    .serve(
                        format!("{}:{}", host, thin_wallet_port)
                            .parse()
                            .expect("this is a valid address"),
                    ),
            );

            // This service lets Prometheus pull metrics from `pd`
            PrometheusBuilder::new()
                .listen_address(
                    format!("{}:{}", host, metrics_port)
                        .parse::<SocketAddr>()
                        .expect("this is a valid address"),
                )
                .install()
                .expect("metrics service set up");

            pd::register_all_metrics();

            // TODO: better error reporting
            // We error out if either service errors, rather than keep running
            tokio::select! {
                x = abci_server => x?.map_err(|e| anyhow::anyhow!(e))?,
                x = light_wallet_server => x?.map_err(|e| anyhow::anyhow!(e))?,
                x = thin_wallet_server => x?.map_err(|e| anyhow::anyhow!(e))?,
            };
        }
        Command::GenerateTestnet {
            num_validator_nodes,
            // TODO this config is gated on a "populate persistent peers"
            // setting in the Go tendermint binary. Populating the persistent
            // peers will be useful in local setups until peer discovery via a seed
            // works.
            starting_ip,
            epoch_duration,
            allocations_input_file,
            validators_input_file,
            output_dir,
            chain_id,
        } => {
            use std::fs;
            use std::fs::File;
            use std::io::Write;
            use std::str::FromStr;
            use std::time::Duration;
            use std::time::{SystemTime, UNIX_EPOCH};

            use tendermint::account::Id;
            use tendermint::public_key::Algorithm;
            use tendermint::Genesis;
            use tendermint::Time;
            use tendermint_config::NodeKey;
            use tendermint_config::PrivValidatorKey;

            use pd::genesis::ValidatorPower;
            use pd::{genesis, testnet::*};
            use penumbra_crypto::Address;
            use penumbra_stake::IdentityKey;

            assert!(
                num_validator_nodes > 0,
                "must have at least one validator node"
            );

            let genesis_time = Time::from_unix_timestamp(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("time travels linearly in a forward direction")
                    .as_secs() as i64,
                0,
            )
            .expect("able to convert current time into Time");

            // By default output directory will be in `~/.penumbra/testnet_data/`
            let output_dir = match output_dir {
                Some(o) => o,
                None => canonicalize_path("~/.penumbra/testnet_data"),
            };

            // Parse allocations from input file
            let allocations = parse_allocations_file(allocations_input_file)?;

            // Parse validators from input file
            let validators = parse_validators_file(validators_input_file)?;

            struct ValidatorKeys {
                // Penumbra spending key and viewing key for this node.
                pub validator_id_sk: SigningKey<SpendAuth>,
                pub validator_id_vk: VerificationKey<SpendAuth>,
                // Consensus key for tendermint.
                pub validator_cons_sk: tendermint::PrivateKey,
                pub validator_cons_pk: tendermint::PublicKey,
                // P2P auth key for tendermint.
                pub node_key_sk: tendermint::PrivateKey,
                #[allow(unused_variables, dead_code)]
                pub node_key_pk: tendermint::PublicKey,
            }
            let mut validator_keys = Vec::<ValidatorKeys>::new();
            // Generate a keypair for each validator
            for _ in 0..num_validator_nodes {
                // Create spending key and viewing key for this node.
                let validator_id_sk = SigningKey::<SpendAuth>::new(OsRng);
                let validator_id_vk = VerificationKey::from(&validator_id_sk);

                // generate consensus key for tendermint.
                let validator_cons_sk =
                    tendermint::PrivateKey::Ed25519(ed25519_consensus::SigningKey::new(OsRng));
                let validator_cons_pk = validator_cons_sk.public_key();

                // generate P2P auth key for tendermint.
                let node_key_sk =
                    tendermint::PrivateKey::Ed25519(ed25519_consensus::SigningKey::new(OsRng));
                let node_key_pk = node_key_sk.public_key();

                let vk = ValidatorKeys {
                    validator_id_sk,
                    validator_id_vk,
                    validator_cons_sk,
                    validator_cons_pk,
                    node_key_sk,
                    node_key_pk,
                };

                validator_keys.push(vk);
            }

            for (n, vk) in validator_keys.iter().enumerate() {
                let node_name = format!("node{}", n);

                let app_state = genesis::AppState {
                    allocations: allocations.iter().map(|a| a.into()).collect(),
                    epoch_duration,
                    validators: validators
                        .iter()
                        .map(|v| {
                            ValidatorPower {
                                validator: Validator {
                                    // Currently there's no way to set validator keys beyond
                                    // manually editing the genesis.json. Otherwise they
                                    // will be randomly generated keys.
                                    identity_key: IdentityKey(vk.validator_id_vk),
                                    consensus_key: vk.validator_cons_pk,
                                    name: v.name.clone(),
                                    website: v.website.clone(),
                                    description: v.description.clone(),
                                    funding_streams: v
                                        .funding_streams
                                        .iter()
                                        .map(|fs| FundingStream {
                                            address: Address::from_str(&fs.address).expect(
                                                "invalid funding stream address in validators.json",
                                            ),
                                            rate_bps: fs.rate_bps,
                                        })
                                        .collect(),
                                    sequence_number: v.sequence_number,
                                },
                                power: v.voting_power.into(),
                            }
                        })
                        .collect(),
                };

                // Create the directory for this node
                let mut node_dir = output_dir.clone();
                node_dir.push(&node_name);

                let mut node_config_dir = node_dir.clone();
                node_config_dir.push("config".to_string());

                let mut node_data_dir = node_dir.clone();
                node_data_dir.push("data".to_string());

                fs::create_dir_all(&node_config_dir)?;
                fs::create_dir_all(&node_data_dir)?;

                // Write this node's tendermint genesis.json file
                let validator_genesis = Genesis {
                    genesis_time,
                    chain_id: chain_id
                        .parse::<tendermint::chain::Id>()
                        .expect("able to create chain ID"),
                    initial_height: 0,
                    consensus_params: tendermint::consensus::Params {
                        block: tendermint::block::Size {
                            max_bytes: 22020096,
                            max_gas: -1,
                            // minimum time increment between consecutive blocks
                            time_iota_ms: 500,
                        },
                        // TODO Should these correspond with values used within `pd` for penumbra epochs?
                        evidence: tendermint::evidence::Params {
                            max_age_num_blocks: 100000,
                            // 1 day
                            max_age_duration: tendermint::evidence::Duration(Duration::new(
                                86400, 0,
                            )),
                            max_bytes: 1048576,
                        },
                        validator: tendermint::consensus::params::ValidatorParams {
                            pub_key_types: vec![Algorithm::Ed25519],
                        },
                        version: Some(tendermint::consensus::params::VersionParams {
                            app_version: 0,
                        }),
                    },
                    // always empty in genesis json
                    app_hash: vec![],
                    app_state: app_state,
                    // List of initial validators. Note this may be overridden entirely by
                    // the application, and may be left empty to make explicit that the
                    // application will initialize the validator set with ResponseInitChain.
                    // - https://docs.tendermint.com/v0.32/tendermint-core/using-tendermint.html
                    // For penumbra, we can leave this empty since the app_state also contains Validator
                    // configs.
                    validators: vec![],
                };
                let mut genesis_file_path = node_config_dir.clone();
                genesis_file_path.push("genesis.json");
                println!(
                    "Writing {} genesis file to: {}",
                    &node_name,
                    genesis_file_path.display()
                );
                let mut genesis_file = File::create(genesis_file_path)?;
                genesis_file
                    .write_all(serde_json::to_string_pretty(&validator_genesis)?.as_bytes())?;

                // Write this node's config.toml
                // Note that this isn't a re-implementation of the `Config` type from
                // Tendermint (https://github.com/tendermint/tendermint/blob/6291d22f46f4c4f9121375af700dbdafa51577e7/config/config.go#L92)
                // so if they change their defaults or the available fields, that won't be reflected in our template.
                let tm_config = generate_tm_config(&node_name);
                let mut config_file_path = node_config_dir.clone();
                config_file_path.push("config.toml");
                println!(
                    "Writing {} config file to: {}",
                    &node_name,
                    config_file_path.display()
                );
                let mut config_file = File::create(config_file_path)?;
                config_file.write_all(tm_config.as_bytes())?;

                // Write this node's node_key.json
                // the underlying type doesn't implement Copy or Clone (for the best)
                let priv_key = tendermint::PrivateKey::Ed25519(
                    vk.node_key_sk.ed25519_signing_key().unwrap().clone(),
                );
                let node_key = NodeKey { priv_key };
                let mut node_key_file_path = node_config_dir.clone();
                node_key_file_path.push("node_key.json");
                println!(
                    "Writing {} node key file to: {}",
                    &node_name,
                    node_key_file_path.display()
                );
                let mut node_key_file = File::create(node_key_file_path)?;
                node_key_file.write_all(serde_json::to_string_pretty(&node_key)?.as_bytes())?;

                // Write this node's priv_validator_key.json
                let address: Id = vk.validator_cons_pk.into();

                // the underlying type doesn't implement Copy or Clone (for the best)
                let priv_key = tendermint::PrivateKey::Ed25519(
                    vk.validator_cons_sk.ed25519_signing_key().unwrap().clone(),
                );
                let priv_validator_key = PrivValidatorKey {
                    address,
                    pub_key: vk.validator_cons_pk,
                    priv_key,
                };
                let mut priv_validator_key_file_path = node_config_dir.clone();
                priv_validator_key_file_path.push("priv_validator_key.json");
                println!(
                    "Writing {} priv validator key file to: {}",
                    &node_name,
                    priv_validator_key_file_path.display()
                );
                let mut priv_validator_key_file = File::create(priv_validator_key_file_path)?;
                priv_validator_key_file
                    .write_all(serde_json::to_string_pretty(&priv_validator_key)?.as_bytes())?;

                // Write the initial validator state:
                let mut priv_validator_state_file_path = node_data_dir.clone();
                priv_validator_state_file_path.push("priv_validator_state.json");
                println!(
                    "Writing {} priv validator state file to: {}",
                    &node_name,
                    priv_validator_state_file_path.display()
                );
                let mut priv_validator_state_file = File::create(priv_validator_state_file_path)?;
                priv_validator_state_file.write_all(get_validator_state().as_bytes())?;

                // Write the validator's signing key:
                let mut validator_signingkey_file_path = node_config_dir.clone();
                validator_signingkey_file_path.push("validator_signingkey.json");
                println!(
                    "Writing {} validator signing key file to: {}",
                    &node_name,
                    validator_signingkey_file_path.display()
                );
                let mut validator_signingkey_file = File::create(validator_signingkey_file_path)?;
                validator_signingkey_file
                    .write_all(serde_json::to_string_pretty(&vk.validator_id_sk)?.as_bytes())?;

                println!("-------------------------------------");
            }
        }
    }

    Ok(())
}
