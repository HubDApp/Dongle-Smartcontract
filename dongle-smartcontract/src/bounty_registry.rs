//! Community bounties for incentivizing contributions (#950).

use crate::errors::ContractError;
use crate::project_registry::ProjectRegistry;
use crate::storage_keys::BountyKey;
use crate::types::{Bounty, BountyPayout, BountyStatus};
use soroban_sdk::{Address, Env, String, Vec};

pub struct BountyRegistry;

impl BountyRegistry {
    fn next_id(env: &Env) -> u64 {
        let id: u64 = env
            .storage()
            .persistent()
            .get(&BountyKey::NextBountyId)
            .unwrap_or(0);
        env.storage()
            .persistent()
            .set(&BountyKey::NextBountyId, &(id + 1));
        id
    }

    /// Post a bounty for a project. Only the project owner may post.
    pub fn post_bounty(
        env: &Env,
        project_id: u64,
        owner: Address,
        title: String,
        amount: u128,
        deadline: u64,
        acceptance_criteria: String,
    ) -> Result<u64, ContractError> {
        owner.require_auth();
        let project =
            ProjectRegistry::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;
        if project.owner != owner {
            return Err(ContractError::NotBountyOwner);
        }
        if title.len() == 0 || acceptance_criteria.len() == 0 || amount == 0 {
            return Err(ContractError::InvalidInput);
        }
        if amount > i128::MAX as u128 {
            return Err(ContractError::InvalidInput);
        }
        if deadline <= env.ledger().timestamp() {
            return Err(ContractError::InvalidInput);
        }
        let id = Self::next_id(env);
        let bounty = Bounty {
            id,
            project_id,
            owner: owner.clone(),
            title,
            amount,
            deadline,
            acceptance_criteria,
            status: BountyStatus::Open,
            claimant: None,
            work_cid: None,
            created_at: env.ledger().timestamp(),
        };
        env.storage()
            .persistent()
            .set(&BountyKey::Bounty(id), &bounty);
        let mut list: Vec<u64> = env
            .storage()
            .persistent()
            .get(&BountyKey::ProjectBounties(project_id))
            .unwrap_or_else(|| Vec::new(env));
        list.push_back(id);
        env.storage()
            .persistent()
            .set(&BountyKey::ProjectBounties(project_id), &list);
        Ok(id)
    }

    /// Claim an open bounty with a work CID.
    pub fn claim_bounty(
        env: &Env,
        bounty_id: u64,
        claimant: Address,
        work_cid: String,
    ) -> Result<(), ContractError> {
        claimant.require_auth();
        let mut bounty: Bounty = env
            .storage()
            .persistent()
            .get(&BountyKey::Bounty(bounty_id))
            .ok_or(ContractError::BountyNotFound)?;
        if bounty.status != BountyStatus::Open {
            return Err(ContractError::BountyNotOpen);
        }
        if env.ledger().timestamp() > bounty.deadline {
            bounty.status = BountyStatus::Expired;
            env.storage()
                .persistent()
                .set(&BountyKey::Bounty(bounty_id), &bounty);
            return Err(ContractError::BountyExpired);
        }
        if work_cid.len() == 0 {
            return Err(ContractError::InvalidInput);
        }
        bounty.status = BountyStatus::Claimed;
        bounty.claimant = Some(claimant);
        bounty.work_cid = Some(work_cid);
        env.storage()
            .persistent()
            .set(&BountyKey::Bounty(bounty_id), &bounty);
        Ok(())
    }

    /// Owner approves a claimed bounty: records payout history and closes it.
    pub fn approve_bounty(
        env: &Env,
        bounty_id: u64,
        owner: Address,
    ) -> Result<(), ContractError> {
        owner.require_auth();
        let mut bounty: Bounty = env
            .storage()
            .persistent()
            .get(&BountyKey::Bounty(bounty_id))
            .ok_or(ContractError::BountyNotFound)?;
        if bounty.owner != owner {
            return Err(ContractError::NotBountyOwner);
        }
        if bounty.status != BountyStatus::Claimed {
            return Err(ContractError::BountyNotOpen);
        }
        let recipient = bounty.claimant.clone().ok_or(ContractError::BountyClaimNotFound)?;
        bounty.status = BountyStatus::Approved;
        env.storage()
            .persistent()
            .set(&BountyKey::Bounty(bounty_id), &bounty);
        let payout = BountyPayout {
            bounty_id,
            recipient,
            amount: bounty.amount,
            paid_at: env.ledger().timestamp(),
        };
        let mut history: Vec<BountyPayout> = env
            .storage()
            .persistent()
            .get(&BountyKey::BountyPayouts(bounty_id))
            .unwrap_or_else(|| Vec::new(env));
        history.push_back(payout);
        env.storage()
            .persistent()
            .set(&BountyKey::BountyPayouts(bounty_id), &history);
        Ok(())
    }

    pub fn get_bounty(env: &Env, bounty_id: u64) -> Option<Bounty> {
        env.storage().persistent().get(&BountyKey::Bounty(bounty_id))
    }

    pub fn list_project_bounties(env: &Env, project_id: u64) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&BountyKey::ProjectBounties(project_id))
            .unwrap_or_else(|| Vec::new(env))
    }

    pub fn payout_history(env: &Env, bounty_id: u64) -> Vec<BountyPayout> {
        env.storage()
            .persistent()
            .get(&BountyKey::BountyPayouts(bounty_id))
            .unwrap_or_else(|| Vec::new(env))
    }
}
