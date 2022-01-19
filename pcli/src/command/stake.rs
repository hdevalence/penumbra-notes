use anyhow::{anyhow, Context, Result};
use comfy_table::{presets, Table};
use futures::stream::TryStreamExt;
use penumbra_crypto::Value;
use penumbra_proto::{light_wallet::ValidatorInfoRequest, thin_wallet::ValidatorRateRequest};
use penumbra_stake::{
    DelegationToken, Epoch, IdentityKey, RateData, ValidatorInfo, STAKING_TOKEN_ASSET_ID,
};
use rand_core::OsRng;
use structopt::StructOpt;

use crate::{ClientStateFile, Opt};

#[derive(Debug, StructOpt)]
pub enum StakeCmd {
    /// Deposit stake into a validator's delegation pool.
    Delegate {
        /// The identity key of the validator to delegate to.
        #[structopt(long)]
        to: String,
        /// The amount of stake to delegate.
        amount: String,
        /// The transaction fee (paid in upenumbra).
        #[structopt(long, default_value = "0")]
        fee: u64,
        /// Optional. Only spend funds originally received by the given address index.
        #[structopt(long)]
        source: Option<u64>,
    },
    /// Withdraw stake from a validator's delegation pool.
    Undelegate {
        /// The amount of delegation tokens to undelegate.
        amount: String,
        /// The transaction fee (paid in upenumbra).
        #[structopt(long, default_value = "0")]
        fee: u64,
        /// Optional. Only spend funds originally received by the given address index.
        #[structopt(long)]
        source: Option<u64>,
    },
    /// Redelegate stake from one validator's delegation pool to another.
    Redelegate {
        /// The identity key of the validator to withdraw delegation from.
        #[structopt(long)]
        from: String,
        /// The identity key of the validator to delegate to.
        #[structopt(long)]
        to: String,
        /// The amount of stake to delegate.
        amount: String,
        /// The transaction fee (paid in upenumbra).
        #[structopt(long, default_value = "0")]
        fee: u64,
        /// Optional. Only spend funds originally received by the given address index.
        #[structopt(long)]
        source: Option<u64>,
    },
    /// Display this wallet's delegations and their value.
    Show,
    /// Display all of the validators participating in the chain.
    ListValidators {
        /// Whether to show validators that are not currently part of the consensus set.
        #[structopt(short = "i", long)]
        show_inactive: bool,
        /// Whether to show detailed validator info.
        #[structopt(short, long)]
        detailed: bool,
    },
}

impl StakeCmd {
    pub fn needs_sync(&self) -> bool {
        true
    }

    pub async fn exec(&self, opt: &Opt, state: &mut ClientStateFile) -> Result<()> {
        match self {
            StakeCmd::Delegate {
                to,
                amount,
                fee,
                source,
            } => {
                let unbonded_amount = {
                    let Value { amount, asset_id } = amount.parse::<Value>()?;
                    if asset_id != *STAKING_TOKEN_ASSET_ID {
                        return Err(anyhow!("staking can only be done with the staking token"));
                    }
                    amount
                };

                let to = to.parse::<IdentityKey>()?;

                let current_epoch = Epoch::from_height(
                    state.last_block_height().unwrap() as u64,
                    state.chain_params().unwrap().epoch_duration,
                );
                let next_epoch = current_epoch.next();

                let mut client = opt.thin_wallet_client().await?;

                let rate_data: RateData = client
                    .validator_rate(tonic::Request::new(ValidatorRateRequest {
                        identity_key: Some(to.into()),
                        epoch_index: next_epoch.index,
                    }))
                    .await?
                    .into_inner()
                    .try_into()?;

                let transaction =
                    state.build_delegate(&mut OsRng, rate_data, unbonded_amount, *fee, *source)?;
                state.commit()?;

                opt.submit_transaction(&transaction).await?;
                // Only commit the state if the transaction was submitted successfully,
                // so that we don't store pending notes that will never appear on-chain.
                state.commit()?;
            }
            StakeCmd::Undelegate {
                amount,
                fee,
                source,
            } => {
                let Value {
                    amount: delegation_amount,
                    asset_id,
                } = amount.parse::<Value>()?;

                let delegation_token: DelegationToken = state
                    .asset_cache()
                    .get(&asset_id)
                    .ok_or_else(|| anyhow::anyhow!("unknown asset id {}", asset_id))?
                    .clone()
                    .try_into()
                    .context("could not parse supplied denomination as a delegation token")?;

                let from = delegation_token.validator();

                let current_epoch = Epoch::from_height(
                    state.last_block_height().unwrap() as u64,
                    state.chain_params().unwrap().epoch_duration,
                );
                let next_epoch = current_epoch.next();

                let mut client = opt.thin_wallet_client().await?;

                let rate_data: RateData = client
                    .validator_rate(tonic::Request::new(ValidatorRateRequest {
                        identity_key: Some(from.into()),
                        epoch_index: next_epoch.index,
                    }))
                    .await?
                    .into_inner()
                    .try_into()?;

                let transaction = state.build_undelegate(
                    &mut OsRng,
                    rate_data,
                    delegation_amount,
                    *fee,
                    *source,
                )?;

                opt.submit_transaction(&transaction).await?;
                // Only commit the state if the transaction was submitted successfully,
                // so that we don't store pending notes that will never appear on-chain.
                state.commit()?;
            }
            StakeCmd::Redelegate { .. } => {
                todo!()
            }
            StakeCmd::Show => {
                todo!()
            }
            StakeCmd::ListValidators {
                show_inactive,
                detailed,
            } => {
                let mut client = opt.light_wallet_client().await?;

                let mut validators = client
                    .validator_info(ValidatorInfoRequest {
                        show_inactive: *show_inactive,
                    })
                    .await?
                    .into_inner()
                    .try_collect::<Vec<_>>()
                    .await?
                    .into_iter()
                    .map(TryInto::try_into)
                    .collect::<Result<Vec<ValidatorInfo>, _>>()?;

                // Sort by voting power (descending)
                validators.sort_by(|a, b| b.status.voting_power.cmp(&a.status.voting_power));

                let total_voting_power = validators
                    .iter()
                    .map(|v| v.status.voting_power)
                    .sum::<u64>() as f64;

                let mut table = Table::new();
                table.load_preset(presets::NOTHING);
                table.set_header(vec!["Voting Power", "Commission", "Validator Info"]);

                for v in validators {
                    let power_percent = 100.0 * (v.status.voting_power as f64) / total_voting_power;
                    let commission_bps = v
                        .validator
                        .funding_streams
                        .iter()
                        .map(|fs| fs.rate_bps)
                        .sum::<u16>();

                    table.add_row(vec![
                        format!("{:.2}%", power_percent),
                        format!("{}bps", commission_bps),
                        v.validator.name,
                    ]);
                    table.add_row(vec![
                        "".into(),
                        "".into(),
                        format!("  {}", v.validator.identity_key),
                    ]);
                    if *detailed {
                        table.add_row(vec![
                            "".into(),
                            "".into(),
                            format!("  {}", v.validator.website),
                        ]);
                        table.add_row(vec![
                            "".into(),
                            "".into(),
                            format!("  {}", v.validator.description),
                        ]);
                    }
                }

                println!("{}", table);
            }
        }

        Ok(())
    }
}
