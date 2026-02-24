//! Contract events for all state-changing actions. Emitted via env.events().publish.

use soroban_sdk::{contractevent, Address, String};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectRegistered {
    pub project_id: u64,
    #[topic]
    pub owner: Address,
    pub name: String,
    pub category: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectUpdated {
    pub project_id: u64,
    #[topic]
    pub owner: Address,
    pub updated_at: u64,
}

impl ProjectUpdated {
    pub fn publish(self, env: &Env) {
        env.events().publish(
            (
                Symbol::new(env, "ProjectUpdated"),
                self.project_id,
                self.owner,
            ),
            (self.updated_at,),
        );
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReviewAdded {
    pub project_id: u64,
    #[topic]
    pub reviewer: Address,
    pub rating: u32,
}

impl ReviewAdded {
    pub fn publish(self, env: &Env) {
        env.events().publish(
            (
                Symbol::new(env, "ReviewAdded"),
                self.project_id,
                self.reviewer,
            ),
            (self.rating,),
        );
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReviewUpdated {
    pub project_id: u64,
    #[topic]
    pub reviewer: Address,
    pub rating: u32,
    pub updated_at: u64,
}

impl ReviewUpdated {
    pub fn publish(self, env: &Env) {
        env.events().publish(
            (
                Symbol::new(env, "ReviewUpdated"),
                self.project_id,
                self.reviewer,
            ),
            (self.rating, self.updated_at),
        );
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationRequested {
    pub project_id: u64,
    #[topic]
    pub requester: Address,
    pub evidence_cid: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationApproved {
    pub project_id: u64,
    #[topic]
    pub verifier: Address,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationRejected {
    pub project_id: u64,
    #[topic]
    pub verifier: Address,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeePaid {
    #[topic]
    pub payer: Address,
    #[topic]
    pub project_id: u64,
    pub amount: u128,
}

impl FeePaid {
    pub fn publish(self, env: &Env) {
        env.events().publish(
            (Symbol::new(env, "FeePaid"), self.payer, self.project_id),
            (self.amount,),
        );
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeSet {
    #[topic]
    pub admin: Address,
    pub amount: u128,
    pub treasury: Address,
}
