use anyhow::Context;
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

use penumbra_chain::params::ChainParameters;
use penumbra_proto::{core::governance::v1alpha1 as pb, DomainType, TypeUrl};

use crate::plan::TransactionPlan;

/// The protobuf type URL for a transaction plan.
pub const TRANSACTION_PLAN_TYPE_URL: &str = "/penumbra.core.transaction.v1alpha1.TransactionPlan";

/// A governance proposal.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(try_from = "pb::Proposal", into = "pb::Proposal")]
pub struct Proposal {
    /// The ID number of the proposal.
    pub id: u64,

    /// A short title describing the intent of the proposal.
    pub title: String,

    /// A natural-language description of the effect of the proposal and its justification.
    pub description: String,

    /// The specific kind and attributes of the proposal.
    pub payload: ProposalPayload,
}

/// A human-readable TOML-serializable version of a proposal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposalToml {
    pub id: u64,
    pub title: String,
    pub description: String,
    #[serde(flatten)]
    pub payload: ProposalPayloadToml,
}

impl From<Proposal> for ProposalToml {
    fn from(proposal: Proposal) -> ProposalToml {
        ProposalToml {
            id: proposal.id,
            title: proposal.title,
            description: proposal.description,
            payload: proposal.payload.into(),
        }
    }
}

impl TryFrom<ProposalToml> for Proposal {
    type Error = anyhow::Error;

    fn try_from(proposal: ProposalToml) -> Result<Proposal, Self::Error> {
        Ok(Proposal {
            id: proposal.id,
            title: proposal.title,
            description: proposal.description,
            payload: proposal.payload.try_into()?,
        })
    }
}

impl From<Proposal> for pb::Proposal {
    fn from(inner: Proposal) -> pb::Proposal {
        let mut proposal = pb::Proposal {
            id: inner.id,
            title: inner.title,
            description: inner.description,
            ..Default::default() // We're about to fill in precisely one of the fields for the payload
        };
        match inner.payload {
            ProposalPayload::Signaling { commit } => {
                proposal.signaling = Some(pb::proposal::Signaling {
                    commit: if let Some(c) = commit {
                        c
                    } else {
                        String::default()
                    },
                });
            }
            ProposalPayload::Emergency { halt_chain } => {
                proposal.emergency = Some(pb::proposal::Emergency { halt_chain });
            }
            ProposalPayload::ParameterChange { old, new } => {
                proposal.parameter_change = Some(pb::proposal::ParameterChange {
                    old_parameters: Some((*old).into()),
                    new_parameters: Some((*new).into()),
                });
            }
            ProposalPayload::DaoSpend { transaction_plan } => {
                proposal.dao_spend = Some(pb::proposal::DaoSpend {
                    transaction_plan: Some(pbjson_types::Any {
                        type_url: TRANSACTION_PLAN_TYPE_URL.to_owned(),
                        value: transaction_plan.encode_to_vec().into(),
                    }),
                });
            }
            ProposalPayload::UpgradePlan { height } => {
                proposal.upgrade_plan = Some(pb::proposal::UpgradePlan { height });
            }
        }
        proposal
    }
}

impl TryFrom<pb::Proposal> for Proposal {
    type Error = anyhow::Error;

    fn try_from(inner: pb::Proposal) -> Result<Proposal, Self::Error> {
        Ok(Proposal {
            id: inner.id,
            title: inner.title,
            description: inner.description,
            payload: if let Some(signaling) = inner.signaling {
                ProposalPayload::Signaling {
                    commit: if signaling.commit.is_empty() {
                        None
                    } else {
                        Some(signaling.commit)
                    },
                }
            } else if let Some(emergency) = inner.emergency {
                ProposalPayload::Emergency {
                    halt_chain: emergency.halt_chain,
                }
            } else if let Some(parameter_change) = inner.parameter_change {
                ProposalPayload::ParameterChange {
                    old: Box::new(
                        parameter_change
                            .old_parameters
                            .ok_or_else(|| anyhow::anyhow!("missing old parameters"))?
                            .try_into()?,
                    ),
                    new: Box::new(
                        parameter_change
                            .new_parameters
                            .ok_or_else(|| anyhow::anyhow!("missing new parameters"))?
                            .try_into()?,
                    ),
                }
            } else if let Some(dao_spend) = inner.dao_spend {
                ProposalPayload::DaoSpend {
                    transaction_plan: {
                        let transaction_plan = dao_spend
                            .transaction_plan
                            .ok_or_else(|| anyhow::anyhow!("missing transaction plan"))?;
                        if transaction_plan.type_url != TRANSACTION_PLAN_TYPE_URL {
                            anyhow::bail!(
                                "unknown transaction plan type url: {}",
                                transaction_plan.type_url
                            );
                        }
                        TransactionPlan::decode(transaction_plan.value)?
                    },
                }
            } else if let Some(upgrade_plan) = inner.upgrade_plan {
                ProposalPayload::UpgradePlan {
                    height: upgrade_plan.height,
                }
            } else {
                anyhow::bail!("missing proposal payload or unknown proposal type");
            },
        })
    }
}

impl TypeUrl for Proposal {
    const TYPE_URL: &'static str = "/penumbra.core.governance.v1alpha1.Proposal";
}

impl DomainType for Proposal {
    type Proto = pb::Proposal;
}

/// The specific kind of a proposal.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "clap", derive(clap::Subcommand))]
pub enum ProposalKind {
    /// A signaling proposal.
    #[cfg_attr(feature = "clap", clap(display_order = 100))]
    Signaling,
    /// An emergency proposal.
    #[cfg_attr(feature = "clap", clap(display_order = 200))]
    Emergency,
    /// A parameter change proposal.
    #[cfg_attr(feature = "clap", clap(display_order = 300))]
    ParameterChange,
    /// A DAO spend proposal.
    #[cfg_attr(feature = "clap", clap(display_order = 400))]
    DaoSpend,
    /// An upgrade proposal.
    #[cfg_attr(feature = "clap", clap(display_order = 500))]
    UpgradePlan,
}

impl FromStr for ProposalKind {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.replace(['-', '_', ' '], "").to_lowercase().as_str() {
            "signaling" => Ok(ProposalKind::Signaling),
            "emergency" => Ok(ProposalKind::Emergency),
            "parameterchange" => Ok(ProposalKind::ParameterChange),
            "daospend" => Ok(ProposalKind::DaoSpend),
            "upgradeplan" => Ok(ProposalKind::UpgradePlan),
            _ => Err(anyhow::anyhow!("invalid proposal kind: {}", s)),
        }
    }
}

impl Proposal {
    /// Get the kind of a proposal.
    pub fn kind(&self) -> ProposalKind {
        match self.payload {
            ProposalPayload::Signaling { .. } => ProposalKind::Signaling,
            ProposalPayload::Emergency { .. } => ProposalKind::Emergency,
            ProposalPayload::ParameterChange { .. } => ProposalKind::ParameterChange,
            ProposalPayload::DaoSpend { .. } => ProposalKind::DaoSpend,
            ProposalPayload::UpgradePlan { .. } => ProposalKind::UpgradePlan,
        }
    }
}

/// The machine-interpretable body of a proposal.
#[derive(Debug, Clone)]
pub enum ProposalPayload {
    /// A signaling proposal is merely for coordination; it does not enact anything automatically by
    /// itself.
    Signaling {
        /// An optional commit hash for code that this proposal refers to.
        commit: Option<String>,
    },
    /// An emergency proposal is immediately passed when 2/3 of all validators approve it, without
    /// waiting for the voting period to conclude.
    Emergency {
        /// If `halt_chain == true`, then the chain will immediately halt when the proposal is
        /// passed.
        halt_chain: bool,
    },
    /// A parameter change proposal describes a replacement of the chain parameters, which should
    /// take effect when the proposal is passed.
    ParameterChange {
        /// The old chain parameters to be replaced.
        ///
        /// Even if the proposal passes, the update will not be applied if the chain parameters have
        /// changed *at all* from these chain parameters. Usually, this should be set to the current
        /// chain parameters at time of proposal.
        old: Box<ChainParameters>,
        /// The new chain parameters to be set.
        ///
        /// The *entire* chain parameters will be replaced with these at the time the proposal is
        /// passed.
        new: Box<ChainParameters>,
    },
    /// A DAO spend proposal describes proposed transaction(s) to be executed or cancelled at
    /// specific heights, with the spend authority of the DAO.
    DaoSpend {
        /// The transaction plan to be executed at the time the proposal is passed.
        ///
        /// This must be a transaction plan which can be executed by the DAO, which means it can't
        /// require any witness data or authorization signatures, but it may use the `DaoSpend`
        /// action.
        transaction_plan: TransactionPlan,
    },
    /// An upgrade plan proposal describes a planned upgrade to the chain. If ratified, the chain
    /// will halt at the specified height, trigger an epoch transition, and halt the chain.
    UpgradePlan { height: u64 },
}

/// A TOML-serializable version of `ProposalPayload`, meant for human consumption.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProposalPayloadToml {
    Signaling {
        commit: Option<String>,
    },
    Emergency {
        halt_chain: bool,
    },
    ParameterChange {
        old: Box<ChainParameters>,
        new: Box<ChainParameters>,
    },
    DaoSpend {
        transaction: String,
    },
    UpgradePlan {
        height: u64,
    },
}

impl TryFrom<ProposalPayloadToml> for ProposalPayload {
    type Error = anyhow::Error;

    fn try_from(toml: ProposalPayloadToml) -> Result<Self, Self::Error> {
        Ok(match toml {
            ProposalPayloadToml::Signaling { commit } => ProposalPayload::Signaling { commit },
            ProposalPayloadToml::Emergency { halt_chain } => {
                ProposalPayload::Emergency { halt_chain }
            }
            ProposalPayloadToml::ParameterChange { old, new } => {
                ProposalPayload::ParameterChange { old, new }
            }
            ProposalPayloadToml::DaoSpend { transaction } => ProposalPayload::DaoSpend {
                transaction_plan: TransactionPlan::decode(Bytes::from(
                    base64::Engine::decode(&base64::engine::general_purpose::STANDARD, transaction)
                        .context("couldn't decode transaction plan from base64")?,
                ))
                .context("couldn't decode transaction plan from proto")?,
            },
            ProposalPayloadToml::UpgradePlan { height } => ProposalPayload::UpgradePlan { height },
        })
    }
}

impl From<ProposalPayload> for ProposalPayloadToml {
    fn from(payload: ProposalPayload) -> Self {
        match payload {
            ProposalPayload::Signaling { commit } => ProposalPayloadToml::Signaling { commit },
            ProposalPayload::Emergency { halt_chain } => {
                ProposalPayloadToml::Emergency { halt_chain }
            }
            ProposalPayload::ParameterChange { old, new } => {
                ProposalPayloadToml::ParameterChange { old, new }
            }
            ProposalPayload::DaoSpend { transaction_plan } => ProposalPayloadToml::DaoSpend {
                transaction: base64::Engine::encode(
                    &base64::engine::general_purpose::STANDARD,
                    transaction_plan.encode_to_vec(),
                ),
            },
            ProposalPayload::UpgradePlan { height } => ProposalPayloadToml::UpgradePlan { height },
        }
    }
}

impl ProposalPayload {
    pub fn is_signaling(&self) -> bool {
        matches!(self, ProposalPayload::Signaling { .. })
    }

    pub fn is_emergency(&self) -> bool {
        matches!(self, ProposalPayload::Emergency { .. })
    }

    pub fn is_parameter_change(&self) -> bool {
        matches!(self, ProposalPayload::ParameterChange { .. })
    }

    pub fn is_dao_spend(&self) -> bool {
        matches!(self, ProposalPayload::DaoSpend { .. })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(try_from = "pb::ProposalState", into = "pb::ProposalState")]
pub enum State {
    Voting,
    Withdrawn { reason: String },
    Finished { outcome: Outcome<String> },
    Claimed { outcome: Outcome<String> },
}

impl State {
    pub fn is_voting(&self) -> bool {
        matches!(self, State::Voting)
    }

    pub fn is_withdrawn(&self) -> bool {
        matches!(self, State::Withdrawn { .. })
    }

    pub fn is_finished(&self) -> bool {
        matches!(self, State::Finished { .. })
    }

    pub fn is_claimed(&self) -> bool {
        matches!(self, State::Claimed { .. })
    }

    pub fn is_passed(&self) -> bool {
        match self {
            State::Finished { outcome } => outcome.is_passed(),
            State::Claimed { outcome } => outcome.is_passed(),
            _ => false,
        }
    }

    pub fn is_failed(&self) -> bool {
        match self {
            State::Finished { outcome } => outcome.is_failed(),
            State::Claimed { outcome } => outcome.is_failed(),
            _ => false,
        }
    }

    pub fn is_slashed(&self) -> bool {
        match self {
            State::Finished { outcome } => outcome.is_slashed(),
            State::Claimed { outcome } => outcome.is_slashed(),
            _ => false,
        }
    }
}

impl State {
    pub fn withdrawn(self) -> Withdrawn<String> {
        match self {
            State::Voting => Withdrawn::No,
            State::Withdrawn { reason } => Withdrawn::WithReason { reason },
            State::Finished { outcome } => match outcome {
                Outcome::Passed => Withdrawn::No,
                Outcome::Failed { withdrawn } | Outcome::Slashed { withdrawn } => withdrawn,
            },
            State::Claimed { outcome } => match outcome {
                Outcome::Passed => Withdrawn::No,
                Outcome::Failed { withdrawn } | Outcome::Slashed { withdrawn } => withdrawn,
            },
        }
    }
}

impl TypeUrl for State {
    const TYPE_URL: &'static str = "/penumbra.core.governance.v1alpha1.ProposalState";
}

impl DomainType for State {
    type Proto = pb::ProposalState;
}

impl From<State> for pb::ProposalState {
    fn from(s: State) -> Self {
        let state = match s {
            State::Voting => pb::proposal_state::State::Voting(pb::proposal_state::Voting {}),
            State::Withdrawn { reason } => {
                pb::proposal_state::State::Withdrawn(pb::proposal_state::Withdrawn { reason })
            }
            State::Finished { outcome } => {
                pb::proposal_state::State::Finished(pb::proposal_state::Finished {
                    outcome: Some(outcome.into()),
                })
            }
            State::Claimed { outcome } => {
                pb::proposal_state::State::Finished(pb::proposal_state::Finished {
                    outcome: Some(outcome.into()),
                })
            }
        };
        pb::ProposalState { state: Some(state) }
    }
}

impl TryFrom<pb::ProposalState> for State {
    type Error = anyhow::Error;

    fn try_from(msg: pb::ProposalState) -> Result<Self, Self::Error> {
        Ok(
            match msg
                .state
                .ok_or_else(|| anyhow::anyhow!("missing proposal state"))?
            {
                pb::proposal_state::State::Voting(pb::proposal_state::Voting {}) => State::Voting,
                pb::proposal_state::State::Withdrawn(pb::proposal_state::Withdrawn { reason }) => {
                    State::Withdrawn { reason }
                }
                pb::proposal_state::State::Finished(pb::proposal_state::Finished { outcome }) => {
                    State::Finished {
                        outcome: outcome
                            .ok_or_else(|| anyhow::anyhow!("missing proposal outcome"))?
                            .try_into()?,
                    }
                }
                pb::proposal_state::State::Claimed(pb::proposal_state::Claimed { outcome }) => {
                    State::Claimed {
                        outcome: outcome
                            .ok_or_else(|| anyhow::anyhow!("missing proposal outcome"))?
                            .try_into()?,
                    }
                }
            },
        )
    }
}

// This is parameterized by `W`, the withdrawal reason, so that we can use `()` where a reason
// doesn't need to be specified. When this is the case, the serialized format in protobufs uses an
// empty string.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(
    try_from = "pb::ProposalOutcome",
    into = "pb::ProposalOutcome",
    bound = "W: Clone, pb::ProposalOutcome: From<Outcome<W>>, Outcome<W>: TryFrom<pb::ProposalOutcome, Error = anyhow::Error>"
)]
pub enum Outcome<W> {
    Passed,
    Failed { withdrawn: Withdrawn<W> },
    Slashed { withdrawn: Withdrawn<W> },
}

impl<W> Outcome<W> {
    /// Determines if the outcome should be refunded (i.e. it was not slashed).
    pub fn should_be_refunded(&self) -> bool {
        !self.is_slashed()
    }

    pub fn is_slashed(&self) -> bool {
        matches!(self, Outcome::Slashed { .. })
    }

    pub fn is_failed(&self) -> bool {
        matches!(self, Outcome::Failed { .. } | Outcome::Slashed { .. })
    }

    pub fn is_passed(&self) -> bool {
        matches!(self, Outcome::Passed)
    }

    pub fn as_ref(&self) -> Outcome<&W> {
        match self {
            Outcome::Passed => Outcome::Passed,
            Outcome::Failed { withdrawn } => Outcome::Failed {
                withdrawn: withdrawn.as_ref(),
            },
            Outcome::Slashed { withdrawn } => Outcome::Slashed {
                withdrawn: withdrawn.as_ref(),
            },
        }
    }

    pub fn map<X>(self, f: impl FnOnce(W) -> X) -> Outcome<X> {
        match self {
            Outcome::Passed => Outcome::Passed,
            Outcome::Failed { withdrawn } => Outcome::Failed {
                withdrawn: Option::from(withdrawn).map(f).into(),
            },
            Outcome::Slashed { withdrawn } => Outcome::Slashed {
                withdrawn: Option::from(withdrawn).map(f).into(),
            },
        }
    }
}

// This is parameterized by `W`, the withdrawal reason, so that we can use `()` where a reason
// doesn't need to be specified. When this is the case, the serialized format in protobufs uses an
// empty string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Withdrawn<W> {
    No,
    WithReason { reason: W },
}

impl<W> Withdrawn<W> {
    pub fn as_ref(&self) -> Withdrawn<&W> {
        match self {
            Withdrawn::No => Withdrawn::No,
            Withdrawn::WithReason { reason } => Withdrawn::WithReason { reason },
        }
    }
}

impl<W> From<Option<W>> for Withdrawn<W> {
    fn from(reason: Option<W>) -> Self {
        match reason {
            Some(reason) => Withdrawn::WithReason { reason },
            None => Withdrawn::No,
        }
    }
}

impl<W> From<Withdrawn<W>> for Option<W> {
    fn from(withdrawn: Withdrawn<W>) -> Self {
        match withdrawn {
            Withdrawn::No => None,
            Withdrawn::WithReason { reason } => Some(reason),
        }
    }
}

impl TryFrom<Withdrawn<String>> for Withdrawn<()> {
    type Error = anyhow::Error;

    fn try_from(withdrawn: Withdrawn<String>) -> Result<Self, Self::Error> {
        Ok(match withdrawn {
            Withdrawn::No => Withdrawn::No,
            Withdrawn::WithReason { reason } => {
                if reason.is_empty() {
                    Withdrawn::WithReason { reason: () }
                } else {
                    anyhow::bail!("withdrawn reason is not empty")
                }
            }
        })
    }
}

impl TypeUrl for Outcome<String> {
    const TYPE_URL: &'static str = "/penumbra.core.governance.v1alpha1.ProposalOutcome";
}

impl DomainType for Outcome<String> {
    type Proto = pb::ProposalOutcome;
}

impl From<Outcome<String>> for pb::ProposalOutcome {
    fn from(o: Outcome<String>) -> Self {
        let outcome = match o {
            Outcome::Passed => {
                pb::proposal_outcome::Outcome::Passed(pb::proposal_outcome::Passed {})
            }
            Outcome::Failed { withdrawn } => {
                pb::proposal_outcome::Outcome::Failed(pb::proposal_outcome::Failed {
                    withdrawn: match withdrawn {
                        Withdrawn::No => None,
                        Withdrawn::WithReason { reason } => {
                            Some(pb::proposal_outcome::Withdrawn { reason })
                        }
                    },
                })
            }
            Outcome::Slashed { withdrawn } => {
                pb::proposal_outcome::Outcome::Slashed(pb::proposal_outcome::Slashed {
                    withdrawn: match withdrawn {
                        Withdrawn::No => None,
                        Withdrawn::WithReason { reason } => {
                            Some(pb::proposal_outcome::Withdrawn { reason })
                        }
                    },
                })
            }
        };
        pb::ProposalOutcome {
            outcome: Some(outcome),
        }
    }
}

impl TryFrom<pb::ProposalOutcome> for Outcome<String> {
    type Error = anyhow::Error;

    fn try_from(msg: pb::ProposalOutcome) -> Result<Self, Self::Error> {
        Ok(
            match msg
                .outcome
                .ok_or_else(|| anyhow::anyhow!("missing proposal outcome"))?
            {
                pb::proposal_outcome::Outcome::Passed(pb::proposal_outcome::Passed {}) => {
                    Outcome::Passed
                }
                pb::proposal_outcome::Outcome::Failed(pb::proposal_outcome::Failed {
                    withdrawn,
                }) => Outcome::Failed {
                    withdrawn: if let Some(pb::proposal_outcome::Withdrawn { reason }) = withdrawn {
                        Withdrawn::WithReason { reason }
                    } else {
                        Withdrawn::No
                    },
                },
                pb::proposal_outcome::Outcome::Slashed(pb::proposal_outcome::Slashed {
                    withdrawn,
                }) => Outcome::Slashed {
                    withdrawn: if let Some(pb::proposal_outcome::Withdrawn { reason }) = withdrawn {
                        Withdrawn::WithReason { reason }
                    } else {
                        Withdrawn::No
                    },
                },
            },
        )
    }
}

impl TypeUrl for Outcome<()> {
    const TYPE_URL: &'static str = "/penumbra.core.governance.v1alpha1.ProposalOutcome";
}

impl DomainType for Outcome<()> {
    type Proto = pb::ProposalOutcome;
}

impl From<Outcome<()>> for pb::ProposalOutcome {
    fn from(o: Outcome<()>) -> Self {
        let outcome = match o {
            Outcome::Passed => {
                pb::proposal_outcome::Outcome::Passed(pb::proposal_outcome::Passed {})
            }
            Outcome::Failed { withdrawn } => {
                pb::proposal_outcome::Outcome::Failed(pb::proposal_outcome::Failed {
                    withdrawn: <Option<()>>::from(withdrawn).map(|()| {
                        pb::proposal_outcome::Withdrawn {
                            reason: "".to_string(),
                        }
                    }),
                })
            }
            Outcome::Slashed { withdrawn } => {
                pb::proposal_outcome::Outcome::Slashed(pb::proposal_outcome::Slashed {
                    withdrawn: <Option<()>>::from(withdrawn).map(|()| {
                        pb::proposal_outcome::Withdrawn {
                            reason: "".to_string(),
                        }
                    }),
                })
            }
        };
        pb::ProposalOutcome {
            outcome: Some(outcome),
        }
    }
}

impl TryFrom<pb::ProposalOutcome> for Outcome<()> {
    type Error = anyhow::Error;

    fn try_from(msg: pb::ProposalOutcome) -> Result<Self, Self::Error> {
        Ok(
            match msg
                .outcome
                .ok_or_else(|| anyhow::anyhow!("missing proposal outcome"))?
            {
                pb::proposal_outcome::Outcome::Passed(pb::proposal_outcome::Passed {}) => {
                    Outcome::Passed
                }
                pb::proposal_outcome::Outcome::Failed(pb::proposal_outcome::Failed {
                    withdrawn,
                }) => Outcome::Failed {
                    withdrawn: <Withdrawn<String>>::from(withdrawn.map(|w| w.reason)).try_into()?,
                },
                pb::proposal_outcome::Outcome::Slashed(pb::proposal_outcome::Slashed {
                    withdrawn,
                }) => Outcome::Slashed {
                    withdrawn: <Withdrawn<String>>::from(withdrawn.map(|w| w.reason)).try_into()?,
                },
            },
        )
    }
}
