use crate::{
    box_grpc_svc::{self, BoxGrpcService},
    App, Command,
};
use anyhow::Result;
use camino::Utf8PathBuf;
use clap::Parser;
use directories::ProjectDirs;
use penumbra_custody::soft_kms::SoftKms;
use penumbra_keys::FullViewingKey;
use penumbra_proto::{
    custody::v1alpha1::{
        custody_protocol_service_client::CustodyProtocolServiceClient,
        custody_protocol_service_server::CustodyProtocolServiceServer,
    },
    view::v1alpha1::{
        view_protocol_service_client::ViewProtocolServiceClient,
        view_protocol_service_server::ViewProtocolServiceServer,
    },
};
use penumbra_view::ViewService;
use penumbra_wallet::KeyStore;
use tracing_subscriber::EnvFilter;
use url::Url;

#[derive(Debug, Parser)]
#[clap(
    name = "pcli",
    about = "The Penumbra command-line interface.",
    version = env!("VERGEN_GIT_SEMVER"),
)]
pub struct Opt {
    /// The remote URL of the pd gRPC endpoint
    #[clap(
        short,
        long,
        default_value = "https://grpc.testnet.penumbra.zone",
        env = "PENUMBRA_NODE_PD_URL",
        parse(try_from_str = Url::parse),
    )]
    node: Url,
    #[clap(subcommand)]
    pub cmd: Command,
    /// The home directory used to store configuration and data.
    #[clap(long, default_value_t = default_home(), env = "PENUMBRA_PCLI_HOME")]
    pub home: Utf8PathBuf,
    /// If set, use a remote view service instead of local synchronization.
    /// Should be specified as a URL, e.g. http://127.0.0.1:8081.
    #[clap(short, long, env = "PENUMBRA_VIEW_ADDRESS")]
    view_address: Option<Url>,
    /// The filter for `pcli`'s log messages.
    #[clap( long, default_value_t = EnvFilter::new("warn"), env = "RUST_LOG")]
    tracing_filter: EnvFilter,
}

impl Opt {
    pub fn init_tracing(&mut self) {
        tracing_subscriber::fmt()
            .with_env_filter(
                std::mem::take(&mut self.tracing_filter)
                    // Without explicitly disabling the `r1cs` target, the ZK proof implementations
                    // will spend an enormous amount of CPU and memory building useless tracing output.
                    .add_directive(
                        "r1cs=off"
                            .parse()
                            .expect("rics=off is a valid filter directive"),
                    ),
            )
            .with_writer(std::io::stderr)
            .init();
    }

    pub async fn into_app(self) -> Result<(App, Command)> {
        let home = self.home.clone();
        let custody_path = home.join(crate::CUSTODY_FILE_NAME);

        // Build the custody service...
        let wallet = KeyStore::load(custody_path)?;
        let soft_kms = SoftKms::new(wallet.spend_key.clone().into());
        let custody_svc = CustodyProtocolServiceServer::new(soft_kms);
        let custody = CustodyProtocolServiceClient::new(box_grpc_svc::local(custody_svc));

        let fvk = wallet.spend_key.full_viewing_key().clone();

        // ...and the view service...
        let view = if !self.cmd.offline() {
            Some(self.view_client(&fvk).await?)
        } else {
            None
        };

        let pd_url = self.node;

        let app = App {
            view,
            custody,
            fvk,
            wallet,
            pd_url,
        };
        Ok((app, self.cmd))
    }

    /// Constructs a [`ViewProtocolServiceClient`] based on the command-line options.
    async fn view_client(
        &self,
        fvk: &FullViewingKey,
    ) -> Result<ViewProtocolServiceClient<BoxGrpcService>> {
        let svc = if let Some(address) = self.view_address.clone() {
            // Use a remote view service.
            tracing::info!(%address, "using remote view service");

            let ep = tonic::transport::Endpoint::new(address.to_string())?;
            box_grpc_svc::connect(ep).await?
        } else {
            // Use an in-memory view service.
            let path = self.home.join(crate::VIEW_FILE_NAME);
            tracing::info!(%path, "using local view service");

            let svc = ViewService::load_or_initialize(Some(path), fvk, self.node.clone()).await?;

            // Now build the view and custody clients, doing gRPC with ourselves
            let svc = ViewProtocolServiceServer::new(svc);
            box_grpc_svc::local(svc)
        };

        Ok(ViewProtocolServiceClient::new(svc))
    }
}

fn default_home() -> Utf8PathBuf {
    let path = ProjectDirs::from("zone", "penumbra", "pcli")
        .expect("Failed to get platform data dir")
        .data_dir()
        .to_path_buf();
    Utf8PathBuf::from_path_buf(path).expect("Platform default data dir was not UTF-8")
}
