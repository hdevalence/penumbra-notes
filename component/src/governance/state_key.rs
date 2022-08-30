use penumbra_crypto::IdentityKey;

pub fn latest_proposal_id() -> &'static str {
    "governance/latest_proposal_id"
}

pub fn proposal_description(proposal_id: u64) -> String {
    format!("governance/proposal/{}/description", proposal_id)
}

pub fn proposal_payload(proposal_id: u64) -> String {
    format!("governance/proposal/{}/payload", proposal_id)
}

pub fn proposal_state(proposal_id: u64) -> String {
    format!("governance/proposal/{}/state", proposal_id)
}

pub fn proposal_voting_start(proposal_id: u64) -> String {
    format!("governance/proposal/{}/voting_start", proposal_id)
}

pub fn proposal_voting_end(proposal_id: u64) -> String {
    format!("governance/proposal/{}/voting_end", proposal_id)
}

pub fn unfinished_proposals() -> &'static str {
    "governance/unfinished_proposals"
}

pub fn proposal_withdraw_key(proposal_id: u64) -> String {
    format!("governance/proposal/{}/withdraw_key", proposal_id)
}

pub fn voting_validators(proposal_id: u64) -> String {
    format!("governance/proposal/{}/voting_validators", proposal_id)
}

pub fn validator_vote(proposal_id: u64, identity_key: IdentityKey) -> String {
    format!(
        "governance/proposal/{}/validator_vote/{}",
        proposal_id, identity_key
    )
}
