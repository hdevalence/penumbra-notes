use anyhow::Result;
use clap::Parser as _;
use pindexer::block::Block;
use pindexer::{Indexer, IndexerExt as _, Options};

#[tokio::main]
async fn main() -> Result<()> {
    Indexer::new(Options::parse())
        .with_default_tracing()
        .with_index(pindexer::supply::Component::new())
        .run()
        .await?;

    Ok(())
}
