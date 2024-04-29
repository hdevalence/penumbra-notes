//! Logic for handling chain upgrades.
//!
//! When consensus-breaking changes are made to the Penumbra software,
//! node operators must coordinate to perform a chain upgrade.
//! This module declares how local `pd` state should be altered, if at all,
//! in order to be compatible with the network post-chain-upgrade.
mod testnet72;

use anyhow::Context;
use futures::StreamExt as _;
use std::path::PathBuf;

use cnidarium::{EscapedByteSlice, StateDelta, StateRead, StateWrite, Storage};
use jmt::RootHash;
use penumbra_app::{app::StateReadExt, SUBSTORE_PREFIXES};
use penumbra_num::Amount;
use penumbra_sct::component::clock::{EpochManager, EpochRead};

use crate::testnet::generate::TestnetConfig;

use flate2::write::GzEncoder;
use flate2::Compression;
use std::fs::File;

/// The kind of migration that should be performed.
pub enum Migration {
    /// No-op migration.
    Noop,
    /// A simple migration: adds a key to the consensus state.
    /// This is useful for testing upgrade mechanisms, including in production.
    SimpleMigration,
    /// Testnet-70 migration: move swap executions from the jmt to nv-storage.
    Testnet70,
    /// Testnet-72 migration:
    /// - Migrate `BatchSwapOutputData` to new protobuf, replacing epoch height with index.
    Testnet72,
    /// Testnet-74 migration: change liquidity positions to be ordered in descending order rather than ascending.
    Testnet74,
}

impl Migration {
    pub async fn migrate(
        &self,
        path_to_export: PathBuf,
        genesis_start: Option<tendermint::time::Time>,
    ) -> anyhow::Result<()> {
        match self {
            Migration::Noop => Ok(()),
            Migration::SimpleMigration => {
                let rocksdb_dir = path_to_export.join("rocksdb");
                let storage = Storage::load(rocksdb_dir, SUBSTORE_PREFIXES.to_vec()).await?;
                let export_state = storage.latest_snapshot();
                let root_hash = export_state.root_hash().await.expect("can get root hash");
                let app_hash_pre_migration: RootHash = root_hash.into();
                let height = export_state
                    .get_block_height()
                    .await
                    .expect("can get block height");
                let post_ugprade_height = height.wrapping_add(1);

                /* --------- writing to the jmt  ------------ */
                tracing::info!(?app_hash_pre_migration, "app hash pre-upgrade");
                let mut delta = StateDelta::new(export_state);
                delta.put_raw(
                    "banana".to_string(),
                    "a good fruit (and migration works!)".into(),
                );
                delta.put_block_height(0u64);
                let root_hash = storage.commit_in_place(delta).await?;
                let app_hash_post_migration: RootHash = root_hash.into();
                tracing::info!(?app_hash_post_migration, "app hash post upgrade");

                /* --------- collecting genesis data -------- */
                tracing::info!("generating genesis");
                let migrated_state = storage.latest_snapshot();
                let root_hash = migrated_state.root_hash().await.expect("can get root hash");
                let app_hash: RootHash = root_hash.into();
                tracing::info!(?root_hash, "root hash from snapshot (post-upgrade)");

                /* ---------- generate genesis ------------  */
                let chain_id = migrated_state.get_chain_id().await?;
                let app_state = penumbra_app::genesis::Content {
                    chain_id,
                    ..Default::default()
                };
                let mut genesis =
                    TestnetConfig::make_genesis(app_state.clone()).expect("can make genesis");
                genesis.app_hash = app_hash
                    .0
                    .to_vec()
                    .try_into()
                    .expect("infaillible conversion");
                genesis.initial_height = post_ugprade_height as i64;
                genesis.genesis_time = genesis_start.unwrap_or_else(|| {
                    let now = tendermint::time::Time::now();
                    tracing::info!(%now, "no genesis time provided, detecting a testing setup");
                    now
                });
                let checkpoint = app_hash.0.to_vec();
                let genesis = TestnetConfig::make_checkpoint(genesis, Some(checkpoint));

                let genesis_json = serde_json::to_string(&genesis).expect("can serialize genesis");
                tracing::info!("genesis: {}", genesis_json);
                let genesis_path = path_to_export.join("genesis.json");
                std::fs::write(genesis_path, genesis_json).expect("can write genesis");

                let validator_state_path = path_to_export.join("priv_validator_state.json");
                let fresh_validator_state =
                    crate::testnet::generate::TestnetValidator::initial_state();
                std::fs::write(validator_state_path, fresh_validator_state)
                    .expect("can write validator state");
                Ok(())
            }
            Migration::Testnet70 => {
                // Our goal is to fetch all swap executions from the jmt and store them in nv-storage.
                // In particular, we want to make sure that client lookups for (height, trading pair)
                // resolve to the same value as before.

                // Setup:
                let start_time = std::time::SystemTime::now();
                let rocksdb_dir = path_to_export.join("rocksdb");
                let storage =
                    Storage::load(rocksdb_dir.clone(), SUBSTORE_PREFIXES.to_vec()).await?;
                let export_state = storage.latest_snapshot();
                let root_hash = export_state.root_hash().await.expect("can get root hash");
                let pre_upgrade_root_hash: RootHash = root_hash.into();
                let pre_upgrade_height = export_state
                    .get_block_height()
                    .await
                    .expect("can get block height");
                let post_upgrade_height = pre_upgrade_height.wrapping_add(1);

                // We initialize a `StateDelta` and start by reaching into the JMT for all entries matching the
                // swap execution prefix. Then, we write each entry to the nv-storage.
                let mut delta = StateDelta::new(export_state);

                let prefix_key = "dex/swap_execution/";
                let mut swap_execution_stream = delta.prefix_raw(prefix_key);

                while let Some(r) = swap_execution_stream.next().await {
                    let (key, swap_execution) = r?;
                    tracing::info!("migrating swap execution: {}", key);
                    delta.nonverifiable_put_raw(key.into_bytes(), swap_execution);
                }

                delta.put_block_height(0u64);

                let post_upgrade_root_hash = storage.commit_in_place(delta).await?;
                tracing::info!(?post_upgrade_root_hash, "post-upgrade root hash");

                let migration_duration = start_time.elapsed().unwrap();

                // Reload storage so we can make reads against its migrated state:
                storage.release().await;
                let storage = Storage::load(rocksdb_dir, SUBSTORE_PREFIXES.to_vec()).await?;
                let migrated_state = storage.latest_snapshot();

                // The migration is complete, now we need to generate a genesis file. To do this, we need
                // to lookup a validator view from the chain, and specify the post-upgrade app hash and
                // initial height.
                let chain_id = migrated_state.get_chain_id().await?;
                let app_state = penumbra_app::genesis::Content {
                    chain_id,
                    ..Default::default()
                };
                let mut genesis =
                    TestnetConfig::make_genesis(app_state.clone()).expect("can make genesis");
                genesis.app_hash = post_upgrade_root_hash
                    .0
                    .to_vec()
                    .try_into()
                    .expect("infaillible conversion");
                genesis.initial_height = post_upgrade_height as i64;
                genesis.genesis_time = genesis_start.unwrap_or_else(|| {
                    let now = tendermint::time::Time::now();
                    tracing::info!(%now, "no genesis time provided, detecting a testing setup");
                    now
                });
                let checkpoint = post_upgrade_root_hash.0.to_vec();
                let genesis = TestnetConfig::make_checkpoint(genesis, Some(checkpoint));

                let genesis_json = serde_json::to_string(&genesis).expect("can serialize genesis");
                tracing::info!("genesis: {}", genesis_json);
                let genesis_path = path_to_export.join("genesis.json");
                std::fs::write(genesis_path, genesis_json).expect("can write genesis");

                let validator_state_path = path_to_export.join("priv_validator_state.json");
                let fresh_validator_state =
                    crate::testnet::generate::TestnetValidator::initial_state();
                std::fs::write(validator_state_path, fresh_validator_state)
                    .expect("can write validator state");

                tracing::info!(
                    pre_upgrade_height,
                    post_upgrade_height,
                    ?pre_upgrade_root_hash,
                    ?post_upgrade_root_hash,
                    duration = migration_duration.as_secs(),
                    "successful migration!"
                );

                Ok(())
            }
            Migration::Testnet72 => testnet72::migrate(path_to_export, genesis_start).await,
            Migration::Testnet74 => {
                // Lookups for liquidity positions based on starting asset were ordered backwards
                // and returning the positions with the least liquidity first. This migration
                // needs to modify the keys stored under the JMT `dex/ra/` prefix key to reverse
                // the ordering of the existing data.

                // Setup:
                let start_time = std::time::SystemTime::now();
                let rocksdb_dir = path_to_export.join("rocksdb");
                let storage =
                    Storage::load(rocksdb_dir.clone(), SUBSTORE_PREFIXES.to_vec()).await?;
                let export_state = storage.latest_snapshot();
                let root_hash = export_state.root_hash().await.expect("can get root hash");
                let pre_upgrade_root_hash: RootHash = root_hash.into();
                let pre_upgrade_height = export_state
                    .get_block_height()
                    .await
                    .expect("can get block height");
                let post_upgrade_height = pre_upgrade_height.wrapping_add(1);

                // We initialize a `StateDelta` and start by reaching into the JMT for all entries matching the
                // swap execution prefix. Then, we write each entry to the nv-storage.
                let mut delta = StateDelta::new(export_state);

                let prefix_key = "dex/ra/".as_bytes();
                tracing::trace!(prefix_key = ?EscapedByteSlice(&prefix_key), "updating liquidity position indices");
                let mut liquidity_stream = delta.nonverifiable_prefix_raw(&prefix_key).boxed();

                while let Some(r) = liquidity_stream.next().await {
                    let (old_key, asset_id) = r?;
                    tracing::info!(?old_key, asset_id = ?EscapedByteSlice(&asset_id), "migrating asset liquidity");

                    // Construct the new key:
                    let mut new_key = [0u8; 55];
                    new_key[0..7].copy_from_slice(b"dex/ra/");
                    // The "from" asset ID remains the same in both keys.
                    new_key[7..32 + 7].copy_from_slice(&old_key[7..32 + 7]);
                    // Use the complement of the amount to ensure that the keys are ordered in descending order.
                    let a_from_b = Amount::from_be_bytes(old_key[32 + 7..32 + 7 + 16].try_into()?);
                    new_key[32 + 7..32 + 7 + 16].copy_from_slice(&(!a_from_b).to_be_bytes());

                    // Delete the old incorrectly ordered key:
                    delta.nonverifiable_delete(old_key.clone());

                    // Store the correctly formatted new key:
                    delta.nonverifiable_put_raw(new_key.to_vec(), asset_id);
                    tracing::info!(
                        new_key = ?EscapedByteSlice(&new_key),
                        ?old_key,
                        "updated liquidity index"
                    );
                }

                delta.put_block_height(0u64);

                let post_upgrade_root_hash = storage.commit_in_place(delta).await?;
                tracing::info!(?post_upgrade_root_hash, "post-upgrade root hash");

                let migration_duration = start_time.elapsed().unwrap();

                // Reload storage so we can make reads against its migrated state:
                storage.release().await;
                let storage = Storage::load(rocksdb_dir, SUBSTORE_PREFIXES.to_vec()).await?;
                let migrated_state = storage.latest_snapshot();

                // The migration is complete, now we need to generate a genesis file. To do this, we need
                // to lookup a validator view from the chain, and specify the post-upgrade app hash and
                // initial height.
                let chain_id = migrated_state.get_chain_id().await?;
                let app_state = penumbra_app::genesis::Content {
                    chain_id,
                    ..Default::default()
                };
                let mut genesis =
                    TestnetConfig::make_genesis(app_state.clone()).expect("can make genesis");
                genesis.app_hash = post_upgrade_root_hash
                    .0
                    .to_vec()
                    .try_into()
                    .expect("infaillible conversion");
                genesis.initial_height = post_upgrade_height as i64;
                genesis.genesis_time = genesis_start.unwrap_or_else(|| {
                    let now = tendermint::time::Time::now();
                    tracing::info!(%now, "no genesis time provided, detecting a testing setup");
                    now
                });
                let checkpoint = post_upgrade_root_hash.0.to_vec();
                let genesis = TestnetConfig::make_checkpoint(genesis, Some(checkpoint));

                let genesis_json = serde_json::to_string(&genesis).expect("can serialize genesis");
                tracing::info!("genesis: {}", genesis_json);
                let genesis_path = path_to_export.join("genesis.json");
                std::fs::write(genesis_path, genesis_json).expect("can write genesis");

                let validator_state_path = path_to_export.join("priv_validator_state.json");
                let fresh_validator_state =
                    crate::testnet::generate::TestnetValidator::initial_state();
                std::fs::write(validator_state_path, fresh_validator_state)
                    .expect("can write validator state");

                tracing::info!(
                    pre_upgrade_height,
                    post_upgrade_height,
                    ?pre_upgrade_root_hash,
                    ?post_upgrade_root_hash,
                    duration = migration_duration.as_secs(),
                    "successful migration!"
                );

                Ok(())
            }
        }
    }
}

/// Compress single directory to gzipped tar archive. Accepts an Option for naming
/// the subdir within the tar archive, which defaults to ".", meaning no nesting.
pub fn archive_directory(
    src_directory: PathBuf,
    archive_filepath: PathBuf,
    subdir_within_archive: Option<String>,
) -> anyhow::Result<()> {
    // Don't clobber an existing target archive.
    if archive_filepath.exists() {
        tracing::error!(
            "export archive filepath already exists: {}",
            archive_filepath.display()
        );
        anyhow::bail!("refusing to overwrite existing archive");
    }

    tracing::info!(
        "creating archive {} -> {}",
        src_directory.display(),
        archive_filepath.display()
    );
    let tarball_file = File::create(&archive_filepath)
        .context("failed to create file for archive: check parent directory and permissions")?;
    let enc = GzEncoder::new(tarball_file, Compression::default());
    let mut tarball = tar::Builder::new(enc);
    let subdir_within_archive = subdir_within_archive.unwrap_or(String::from("."));
    tarball
        .append_dir_all(subdir_within_archive, src_directory.as_path())
        .context("failed to package archive contents")?;
    Ok(())
}
