//! Grant / funding announcement features (issue #949).
//!
//! Projects can post grant opportunities (`post_grant_opportunity`), which are
//! retrievable per project set (`get_grants_for_projects`). Applications and
//! winners are tracked on-chain, and posting emits `GrantOpportunityPosted`.

use crate::constants::*;
use crate::errors::ContractError;
use crate::events::publish_grant_opportunity_posted_event;
use crate::project_registry::ProjectRegistry;
use crate::storage_keys::ExtensionKey2 as EK2;
use crate::storage_manager::StorageManager;
use crate::types::Grant;
use soroban_sdk::{Address, Env, String, Vec};

pub struct GrantRegistry;

impl GrantRegistry {
    fn next_id(env: &Env) -> u64 {
        let current: u64 = env
            .storage()
            .persistent()
            .get(&EK2::NextGrantId)
            .unwrap_or(0);
        let next = current.saturating_add(1);
        env.storage().persistent().set(&EK2::NextGrantId, &next);
        next
    }

    /// Post a grant/funding opportunity for `project_id`.
    ///
    /// The poster authenticates and must own the project. `deadline` must be
    /// in the future; `title` and `criteria` must be non-empty and bounded.
    pub fn post_grant_opportunity(
        env: &Env,
        poster: Address,
        project_id: u64,
        title: String,
        amount: u128,
        deadline: u64,
        criteria: String,
    ) -> Result<u64, ContractError> {
        poster.require_auth();
        let project =
            ProjectRegistry::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;
        if project.owner != poster {
            return Err(ContractError::Unauthorized);
        }
        if title.len() == 0
            || title.len() as usize > MAX_NAME_LEN
            || criteria.len() == 0
            || criteria.len() as usize > MAX_DESCRIPTION_LEN
        {
            return Err(ContractError::InvalidInput);
        }
        if deadline <= env.ledger().timestamp() {
            return Err(ContractError::InvalidInput);
        }

        let id = Self::next_id(env);
        let grant = Grant {
            id,
            project_id,
            title: title.clone(),
            amount,
            deadline,
            criteria,
            created_at: env.ledger().timestamp(),
        };
        env.storage().persistent().set(&EK2::GrantOpp(id), &grant);

        let mut for_project: Vec<u64> = env
            .storage()
            .persistent()
            .get(&EK2::ProjectGrants(project_id))
            .unwrap_or_else(|| Vec::new(env));
        for_project.push_back(id);
        env.storage()
            .persistent()
            .set(&EK2::ProjectGrants(project_id), &for_project);

        StorageManager::extend_project_ttl(env, project_id);
        publish_grant_opportunity_posted_event(env, id, project_id, title, amount, deadline);
        Ok(id)
    }

    pub fn get_grant(env: &Env, grant_id: u64) -> Option<Grant> {
        env.storage().persistent().get(&EK2::GrantOpp(grant_id))
    }

    /// Return all grants posted by any of `project_ids`, oldest-first.
    pub fn get_grants_for_projects(env: &Env, project_ids: Vec<u64>) -> Vec<Grant> {
        let mut out = Vec::new(env);
        for project_id in project_ids.iter() {
            let ids: Vec<u64> = env
                .storage()
                .persistent()
                .get(&EK2::ProjectGrants(project_id))
                .unwrap_or_else(|| Vec::new(env));
            for id in ids.iter() {
                if let Some(grant) = Self::get_grant(env, id) {
                    out.push_back(grant);
                }
            }
        }
        out
    }

    /// Apply for a grant before its deadline. One application per address.
    pub fn apply_for_grant(
        env: &Env,
        applicant: Address,
        grant_id: u64,
    ) -> Result<(), ContractError> {
        applicant.require_auth();
        let grant = Self::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;
        if env.ledger().timestamp() > grant.deadline {
            return Err(ContractError::InvalidInput);
        }
        let mut apps: Vec<Address> = env
            .storage()
            .persistent()
            .get(&EK2::GrantApplications(grant_id))
            .unwrap_or_else(|| Vec::new(env));
        if apps.contains(&applicant) {
            return Err(ContractError::DuplicateGrantApplication);
        }
        apps.push_back(applicant);
        env.storage()
            .persistent()
            .set(&EK2::GrantApplications(grant_id), &apps);
        Ok(())
    }

    /// Record the winning applicant. Only the grant's project owner may call.
    pub fn select_grant_winner(
        env: &Env,
        caller: Address,
        grant_id: u64,
        winner: Address,
    ) -> Result<(), ContractError> {
        caller.require_auth();
        let grant = Self::get_grant(env, grant_id).ok_or(ContractError::GrantNotFound)?;
        let project =
            ProjectRegistry::get_project(env, grant.project_id).ok_or(ContractError::ProjectNotFound)?;
        if project.owner != caller {
            return Err(ContractError::Unauthorized);
        }
        let apps: Vec<Address> = env
            .storage()
            .persistent()
            .get(&EK2::GrantApplications(grant_id))
            .unwrap_or_else(|| Vec::new(env));
        if !apps.contains(&winner) {
            return Err(ContractError::InvalidInput);
        }
        env.storage()
            .persistent()
            .set(&EK2::GrantWinner(grant_id), &winner);
        Ok(())
    }

    /// Tracked applications for a grant.
    pub fn get_grant_applications(env: &Env, grant_id: u64) -> Vec<Address> {
        env.storage()
            .persistent()
            .get(&EK2::GrantApplications(grant_id))
            .unwrap_or_else(|| Vec::new(env))
    }

    /// Tracked winner for a grant, if one has been selected.
    pub fn get_grant_winner(env: &Env, grant_id: u64) -> Option<Address> {
        env.storage().persistent().get(&EK2::GrantWinner(grant_id))
    }
}
