//! Contract events for all state-changing actions. Emitted via env.events().publish.

use soroban_sdk::{Address, Env, String, Symbol};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectRegistered {
    pub project_id: u64,
    pub owner: Address,
    pub name: String,
    pub category: String,
}

impl ProjectRegistered {
    pub fn publish(self, env: &Env) {
        env.events().publish(
            (
                Symbol::new(env, "ProjectRegistered"),
                self.project_id,
                self.owner,
            ),
            (self.name, self.category),
        );
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectUpdated {
    pub project_id: u64,
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
    pub requester: Address,
    pub evidence_cid: String,
}

impl VerificationRequested {
    pub fn publish(self, env: &Env) {
        env.events().publish(
            (
                Symbol::new(env, "VerificationRequested"),
                self.project_id,
                self.requester,
            ),
            (self.evidence_cid,),
        );
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationApproved {
    pub project_id: u64,
    pub verifier: Address,
}

impl VerificationApproved {
    pub fn publish(self, env: &Env) {
        env.events().publish(
            (
                Symbol::new(env, "VerificationApproved"),
                self.project_id,
                self.verifier,
            ),
            (),
        );
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationRejected {
    pub project_id: u64,
    pub verifier: Address,
}

impl VerificationRejected {
    pub fn publish(self, env: &Env) {
        env.events().publish(
            (
                Symbol::new(env, "VerificationRejected"),
                self.project_id,
                self.verifier,
            ),
            (),
        );
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeePaid {
    pub payer: Address,
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
    pub admin: Address,
    pub amount: u128,
    pub treasury: Address,
}

impl FeeSet {
    pub fn publish(self, env: &Env) {
        env.events().publish(
            (Symbol::new(env, "FeeSet"), self.admin),
            (self.amount, self.treasury),
        );
    }
}
