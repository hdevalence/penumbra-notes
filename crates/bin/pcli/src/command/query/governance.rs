//use std::io::{stdout, Write};

use anyhow::Result;
//use serde::Serialize;

use crate::App;

#[derive(Debug, clap::Subcommand)]
pub enum GovernanceCmd {
    /// List all governance proposals by number.
    ListProposals {
        /// Whether to include proposals which have already finished voting.
        #[clap(short, long)]
        inactive: bool,
    },
    /// Query for information about a particular proposal.
    Proposal {
        /// The proposal id to query.
        proposal_id: u64,
        /// The query to ask of it.
        #[clap(subcommand)]
        query: PerProposalCmd,
    },
}

#[derive(Debug, clap::Subcommand)]
pub enum PerProposalCmd {
    /// Fetch the details of a proposal, as submitted to the chain.
    Definition,
    /// Display the current state of a proposal.
    State,
    /// Display the voting period of a proposal.
    Period,
    /// Display the most recent tally of votes on the proposal.
    Tally,
}

impl GovernanceCmd {
    pub async fn exec(&self, _app: &mut App) -> Result<()> {
        // use PerProposalCmd::*;

        match self {
            GovernanceCmd::ListProposals { .. } => {
                let mut client = GovernanceQueryServiceClient::new(app.pd_channel().await?);
                let proposals = client
                    .list_proposals(ListProposalsRequest { chain_id })
                    .await?
                    .into_inner()
                    .try_into()
                    .context("cannot process proposal list data");
                // below code is not usable outside of our own crates because
                // it does raw state key accesses.
                /*
                let proposal_id_list: Vec<u64> = if *inactive {
                    let next: u64 = client
                        .key_proto(next_proposal_id())
                        .await?
                        .context("no proposal found")?;

                    (0..next).collect()
                } else {
                    let mut unfinished = client
                        .prefix_value(PrefixValueRequest {
                            prefix: all_unfinished_proposals().into(),
                            ..Default::default()
                        })
                        .await?
                        .into_inner();
                    let mut unfinished_proposals: Vec<u64> = Vec::new();
                    while let Some(PrefixValueResponse { key, .. }) =
                        unfinished.next().await.transpose()?
                    {
                        let proposal_id = u64::from_str(
                            key.rsplit('/').next().context("key must contain a slash")?,
                        )
                        .context("proposal id was not a valid u64")?;
                        unfinished_proposals.push(proposal_id);
                    }
                    unfinished_proposals
                };

                let mut writer = stdout();
                for proposal_id in proposal_id_list {
                    let proposal: Proposal = client
                        .key_domain(proposal_definition(proposal_id))
                        .await?
                        .context(format!("proposal {} not found", proposal_id))?;

                    let proposal_title = proposal.title;

                    let proposal_state: ProposalState = client
                        .key_domain(proposal_state(proposal_id))
                        .await?
                        .context(format!("proposal state for {} not found", proposal_id))?;

                    writeln!(
                        writer,
                        "#{proposal_id} {proposal_state:?}    {proposal_title}"
                    )?;
                }
                 */
            }
            GovernanceCmd::Proposal { .. } => {
                unimplemented!("governance component needs an RPC defined");
                // below code is not usable outside of our own crates because
                // it does raw state key accesses.
                /*
                    match query {
                    Definition => {
                        let proposal: Proposal = client
                            .key_domain(proposal_definition(*proposal_id))
                            .await?
                            .context(format!(
                                "proposal definition for proposal {} not found",
                                proposal_id
                            ))?;
                        toml(&proposal)?;
                    }
                    State => {
                        let state: ProposalState = client
                            .key_domain(proposal_state(*proposal_id))
                            .await?
                            .context(format!(
                                "proposal state for proposal {} not found",
                                proposal_id
                            ))?;
                        json(&state)?;
                    }
                    Period => {
                        let start: u64 = client
                            .key_proto(proposal_voting_start(*proposal_id))
                            .await?
                            .context(format!(
                                "proposal voting start for proposal {} not found",
                                proposal_id
                            ))?;
                        let end: u64 = client
                            .key_proto(proposal_voting_end(*proposal_id))
                            .await?
                            .context(format!(
                                "proposal voting end for proposal {} not found",
                                proposal_id
                            ))?;
                        let period = json!({
                            "voting_start_block": start,
                            "voting_end_block": end,
                        });
                        json(&period)?;
                    }
                    Tally => {
                        let validator_votes: BTreeMap<IdentityKey, Vote> = client
                            .prefix_domain::<Vote>(all_validator_votes_for_proposal(*proposal_id))
                            .await?
                            .and_then(|r| async move {
                                let identity_key = IdentityKey::from_str(
                                    r.0.rsplit('/').next().context("invalid key")?,
                                )?;
                                Ok((identity_key, r.1))
                            })
                            .try_collect()
                            .await?;

                        let mut validator_votes_and_power: BTreeMap<IdentityKey, (Vote, u64)> =
                            BTreeMap::new();
                        for (identity_key, vote) in validator_votes.iter() {
                            let power: u64 = client
                                .key_proto(voting_power_at_proposal_start(*proposal_id, *identity_key))
                                .await
                                .context("Error looking for validator power")?
                                .context("validator power not found")?;

                            validator_votes_and_power.insert(*identity_key, (*vote, power));
                        }

                        let mut delegator_tallies: BTreeMap<IdentityKey, penumbra_governance::Tally> =
                            client
                                .prefix_domain::<penumbra_governance::Tally>(
                                    all_tallied_delegator_votes_for_proposal(*proposal_id),
                                )
                                .await?
                                .and_then(|r| async move {
                                    Ok((
                                        IdentityKey::from_str(
                                            r.0.rsplit('/').next().context("invalid key")?,
                                        )?,
                                        r.1,
                                    ))
                                })
                                .try_collect()
                                .await?;

                        // Combine the two mappings
                        let mut total = penumbra_governance::Tally::default();
                        let mut all_votes_and_power: BTreeMap<String, serde_json::Value> =
                            BTreeMap::new();
                        for (identity_key, (vote, power)) in validator_votes_and_power.into_iter() {
                            all_votes_and_power.insert(identity_key.to_string(), {
                                let mut map = serde_json::Map::new();
                                map.insert(
                                    "validator".to_string(),
                                    json!({
                                        vote.to_string(): power,
                                    }),
                                );
                                let delegator_tally =
                                    if let Some(tally) = delegator_tallies.remove(&identity_key) {
                                        map.insert("delegators".to_string(), json_tally(&tally));
                                        tally
                                    } else {
                                        Default::default()
                                    };
                                // Subtract delegator total from validator power, then add delegator
                                // tally in to get the total tally for this validator:
                                let sub_total = penumbra_governance::Tally::from((
                                    vote,
                                    power - delegator_tally.total(),
                                )) + delegator_tally;
                                map.insert("sub_total".to_string(), json_tally(&sub_total));
                                total += sub_total;
                                map.into()
                            });
                        }
                        for (identity_key, tally) in delegator_tallies.into_iter() {
                            all_votes_and_power.insert(identity_key.to_string(), {
                                let mut map = serde_json::Map::new();
                                let sub_total = tally;
                                map.insert("delegators".to_string(), json_tally(&tally));
                                map.insert("sub_total".to_string(), json_tally(&sub_total));
                                total += sub_total;
                                map.into()
                            });
                        }

                        json(&json!({
                            "total": json_tally(&total),
                            "details": all_votes_and_power,
                        }))?;
                    }
                },
                */
            }
        }

        // Ok(())
    }
}

/*
fn json<T: Serialize>(value: &T) -> Result<()> {
    let mut writer = stdout();
    serde_json::to_writer_pretty(&mut writer, value)?;
    writer.write_all(b"\n")?;
    Ok(())
}

fn json_tally(tally: &penumbra_governance::Tally) -> serde_json::Value {
    let mut map = serde_json::Map::new();
    if tally.yes() > 0 {
        map.insert("yes".to_string(), tally.yes().into());
    }
    if tally.no() > 0 {
        map.insert("no".to_string(), tally.no().into());
    }
    if tally.abstain() > 0 {
        map.insert("abstain".to_string(), tally.abstain().into());
    }
    map.into()
}

fn toml<T: Serialize>(value: &T) -> Result<()> {
    let mut writer = stdout();
    let string = toml::to_string_pretty(value)?;
    writer.write_all(string.as_bytes())?;
    Ok(())
}
 */
