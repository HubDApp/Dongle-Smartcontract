//! Contract events for all state-changing actions. Emitted consistently for indexing and clients.

use soroban_sdk::contractevent;

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectRegistered {
    #[topic]
    pub project_id: u64,
    #[topic]
    pub owner: soroban_sdk::Address,
    pub name: soroban_sdk::String,
    pub category: soroban_sdk::String,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectUpdated {
    #[topic]
    pub project_id: u64,
    #[topic]
    pub owner: soroban_sdk::Address,
    pub updated_at: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReviewAdded {
    #[topic]
    pub project_id: u64,
    #[topic]
    pub reviewer: soroban_sdk::Address,
    pub rating: u32,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReviewUpdated {
    #[topic]
    pub project_id: u64,
    #[topic]
    pub reviewer: soroban_sdk::Address,
    pub rating: u32,
    pub updated_at: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationRequested {
    #[topic]
    pub project_id: u64,
    #[topic]
    pub requester: soroban_sdk::Address,
    pub evidence_cid: soroban_sdk::String,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationApproved {
    #[topic]
    pub project_id: u64,
    #[topic]
    pub verifier: soroban_sdk::Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationRejected {
    #[topic]
    pub project_id: u64,
    #[topic]
    pub verifier: soroban_sdk::Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeePaid {
    #[topic]
    pub payer: soroban_sdk::Address,
    #[topic]
    pub project_id: u64,
    pub amount: u128,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeSet {
    #[topic]
    pub admin: soroban_sdk::Address,
    pub amount: u128,
    pub treasury: soroban_sdk::Address,
}
