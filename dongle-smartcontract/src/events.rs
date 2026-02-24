//! Contract events for all state-changing actions. Emitted consistently for indexing and clients.

use soroban_sdk::{contractevent, Address, String};

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectRegistered {
    #[topic]
    pub project_id: u64,
    #[topic]
    pub owner: Address,
    pub name: String,
    pub category: String,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectUpdated {
    #[topic]
    pub project_id: u64,
    #[topic]
    pub owner: Address,
    pub updated_at: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReviewAdded {
    #[topic]
    pub project_id: u64,
    #[topic]
    pub reviewer: Address,
    pub rating: u32,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReviewUpdated {
    #[topic]
    pub project_id: u64,
    #[topic]
    pub reviewer: Address,
    pub rating: u32,
    pub updated_at: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationRequested {
    #[topic]
    pub project_id: u64,
    #[topic]
    pub requester: Address,
    pub evidence_cid: String,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationApproved {
    #[topic]
    pub project_id: u64,
    #[topic]
    pub verifier: Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationRejected {
    #[topic]
    pub project_id: u64,
    #[topic]
    pub verifier: Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeePaid {
    #[topic]
    pub payer: Address,
    #[topic]
    pub project_id: u64,
    pub amount: u128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeSet {
    #[topic]
    pub admin: Address,
    pub amount: u128,
    pub treasury: Address,
}
