//! Delegated endorsement authority (#919).
//!
//! An address holding an active endorsement of a project (a *root
//! endorser*) can delegate its endorsement authority to other addresses,
//! optionally limited to a set of expertise areas.  Delegates can endorse the
//! project on the root's behalf and can further sub-delegate within their own
//! scope, forming a delegation chain.
//!
//! ## Chain rules
//!
//! * Each delegate holds at most one delegation per project, so a chain is a
//!   simple list `delegate → delegator → … → root`.
//! * Granting a delegation walks the delegator's chain upward; if it reaches
//!   the delegate the grant is rejected as a cycle.  Chains are capped at
//!   `MAX_DELEGATION_CHAIN_DEPTH`.
//! * Authority is validated dynamically: a delegation is effective only while
//!   its chain still ends at an address with an active endorsement.  Revoking
//!   (or the root unendorsing / expiring) therefore suspends every
//!   sub-delegation below that link without touching their records.

use crate::constants::{MAX_DELEGATES_PER_ENDORSER, MAX_DELEGATION_CHAIN_DEPTH};
use crate::endorsement_registry::EndorsementRegistry;
use crate::errors::ContractError;
use crate::events::{
    publish_endorsement_delegated_event, publish_endorsement_delegation_revoked_event,
};
use crate::project_registry::ProjectRegistry;
use crate::storage_keys::{EndorsementDelegationKey, EndorsementRecordKey};
use crate::types::EndorsementDelegation;
use crate::utils::Utils;
use soroban_sdk::{Address, Env, String, Vec};

pub struct EndorsementDelegationRegistry;

impl EndorsementDelegationRegistry {
    // ── Granting & revoking ───────────────────────────────────────────────

    /// Delegate `delegator`'s endorsement authority on `project_id` to
    /// `delegate`, limited to `scope` (empty = the delegator's full scope).
    pub fn delegate_endorsement_authority(
        env: &Env,
        project_id: u64,
        delegator: Address,
        delegate: Address,
        scope: Vec<String>,
    ) -> Result<(), ContractError> {
        delegator.require_auth();

        ProjectRegistry::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;
        if delegator == delegate {
            return Err(ContractError::SelfDelegation);
        }
        EndorsementRegistry::validate_areas(&scope)?;

        let allowed = Self::effective_scope(env, project_id, &delegator)
            .ok_or(ContractError::NoEndorsementAuthority)?;
        if !Self::within_scope(&scope, &allowed) {
            return Err(ContractError::DelegationScopeExceeded);
        }
        let scope = if scope.is_empty() { allowed } else { scope };

        if Self::get_delegation(env, project_id, delegate.clone()).is_some() {
            return Err(ContractError::AlreadyDelegated);
        }

        // Walk up from the delegator: reaching the delegate means a cycle.
        let chain = Self::get_delegation_chain(env, project_id, delegator.clone());
        if chain.contains(&delegate) {
            return Err(ContractError::DelegationCycleDetected);
        }
        if chain.len() > MAX_DELEGATION_CHAIN_DEPTH {
            return Err(ContractError::DelegationChainTooDeep);
        }

        let mut delegates = Self::get_delegates(env, project_id, delegator.clone());
        if delegates.len() >= MAX_DELEGATES_PER_ENDORSER {
            return Err(ContractError::MaxDelegatesExceeded);
        }
        delegates.push_back(delegate.clone());
        Self::save_delegates(env, project_id, &delegator, &delegates);

        let delegation = EndorsementDelegation {
            project_id,
            delegator: delegator.clone(),
            delegate: delegate.clone(),
            scope: scope.clone(),
            created_at: env.ledger().timestamp(),
        };
        env.storage().persistent().set(
            &EndorsementDelegationKey::Delegation(project_id, delegate.clone()),
            &delegation,
        );

        publish_endorsement_delegated_event(env, project_id, delegator, delegate, scope);
        Ok(())
    }

    /// Revoke the authority `delegator` granted to `delegate`.  Any
    /// sub-delegations made by `delegate` become ineffective immediately.
    pub fn revoke_endorsement_delegation(
        env: &Env,
        project_id: u64,
        delegator: Address,
        delegate: Address,
    ) -> Result<(), ContractError> {
        delegator.require_auth();

        let delegation = Self::get_delegation(env, project_id, delegate.clone())
            .ok_or(ContractError::DelegationNotFound)?;
        if delegation.delegator != delegator {
            return Err(ContractError::DelegationNotFound);
        }

        env.storage()
            .persistent()
            .remove(&EndorsementDelegationKey::Delegation(project_id, delegate.clone()));
        let delegates = Self::get_delegates(env, project_id, delegator.clone());
        let remaining = Utils::remove_item_from_vec(env, &delegates, &delegate);
        Self::save_delegates(env, project_id, &delegator, &remaining);

        publish_endorsement_delegation_revoked_event(env, project_id, delegator, delegate);
        Ok(())
    }

    /// Endorse `project_id` as `delegate`, using delegated authority.  The
    /// requested `areas` must fall within the delegation scope.  Returns the
    /// endorsement ID.
    pub fn endorse_project_on_behalf(
        env: &Env,
        project_id: u64,
        delegate: Address,
        areas: Vec<String>,
        expires_at: Option<u64>,
    ) -> Result<u64, ContractError> {
        delegate.require_auth();

        let delegation = Self::get_delegation(env, project_id, delegate.clone())
            .ok_or(ContractError::NoEndorsementAuthority)?;
        let root = Self::resolve_root(env, project_id, &delegate)
            .ok_or(ContractError::NoEndorsementAuthority)?;
        // A general endorsement would exceed a restricted scope, so default
        // to the full delegated scope instead.
        let areas = if areas.is_empty() {
            delegation.scope.clone()
        } else {
            areas
        };
        if !Self::within_scope(&areas, &delegation.scope) {
            return Err(ContractError::DelegationScopeExceeded);
        }

        EndorsementRegistry::endorse_internal(
            env,
            project_id,
            delegate,
            areas,
            expires_at,
            Some(root),
        )
    }

    // ── Queries ───────────────────────────────────────────────────────────

    pub fn get_delegation(
        env: &Env,
        project_id: u64,
        delegate: Address,
    ) -> Option<EndorsementDelegation> {
        env.storage()
            .persistent()
            .get(&EndorsementDelegationKey::Delegation(project_id, delegate))
    }

    /// Delegates appointed by `delegator` for `project_id`.
    pub fn get_delegates(env: &Env, project_id: u64, delegator: Address) -> Vec<Address> {
        env.storage()
            .persistent()
            .get(&EndorsementDelegationKey::Delegates(project_id, delegator))
            .unwrap_or_else(|| Vec::new(env))
    }

    /// The chain of delegators above `address`, starting with `address`
    /// itself and ending at the topmost delegator found (the root endorser
    /// when the chain is valid).
    pub fn get_delegation_chain(env: &Env, project_id: u64, address: Address) -> Vec<Address> {
        let mut chain = Vec::new(env);
        chain.push_back(address.clone());
        let mut cur = address;
        while chain.len() <= MAX_DELEGATION_CHAIN_DEPTH {
            let Some(delegation) = Self::get_delegation(env, project_id, cur) else {
                break;
            };
            if chain.contains(&delegation.delegator) {
                break;
            }
            chain.push_back(delegation.delegator.clone());
            cur = delegation.delegator;
        }
        chain
    }

    /// Whether `address` may endorse `project_id`, either in its own right or
    /// through a valid delegation chain.
    pub fn has_endorsement_authority(env: &Env, project_id: u64, address: Address) -> bool {
        EndorsementRegistry::has_active_endorsement(env, project_id, &address)
            || Self::resolve_root(env, project_id, &address).is_some()
    }

    // ── Internal helpers ──────────────────────────────────────────────────

    /// Root endorser at the top of `address`'s delegation chain, provided it
    /// still holds an active endorsement.
    fn resolve_root(env: &Env, project_id: u64, address: &Address) -> Option<Address> {
        let chain = Self::get_delegation_chain(env, project_id, address.clone());
        let mut i = 1;
        while i < chain.len() {
            let candidate = chain.get(i)?;
            if EndorsementRegistry::has_active_endorsement(env, project_id, &candidate) {
                return Some(candidate);
            }
            i += 1;
        }
        None
    }

    /// Areas `address` may endorse or delegate for (empty = unrestricted), or
    /// `None` when it holds no authority.
    fn effective_scope(env: &Env, project_id: u64, address: &Address) -> Option<Vec<String>> {
        if EndorsementRegistry::has_active_endorsement(env, project_id, address) {
            return Some(
                EndorsementRegistry::get_user_project_endorsement(env, project_id, address)
                    .map(|record| record.areas)
                    .unwrap_or_else(|| Vec::new(env)),
            );
        }
        Self::resolve_root(env, project_id, address)?;
        Self::get_delegation(env, project_id, address.clone()).map(|d| d.scope)
    }

    /// `requested ⊆ allowed`, where an empty `allowed` means unrestricted and
    /// an empty `requested` means "everything allowed".
    fn within_scope(requested: &Vec<String>, allowed: &Vec<String>) -> bool {
        if allowed.is_empty() || requested.is_empty() {
            return true;
        }
        requested.iter().all(|area| allowed.contains(&area))
    }

    /// Persist the delegate list and mirror it onto the delegator's
    /// endorsement record (`Endorsement::delegated_to`).
    fn save_delegates(env: &Env, project_id: u64, delegator: &Address, delegates: &Vec<Address>) {
        env.storage().persistent().set(
            &EndorsementDelegationKey::Delegates(project_id, delegator.clone()),
            delegates,
        );
        if let Some(mut record) =
            EndorsementRegistry::get_user_project_endorsement(env, project_id, delegator)
        {
            record.delegated_to = if delegates.is_empty() {
                None
            } else {
                Some(delegates.clone())
            };
            env.storage()
                .persistent()
                .set(&EndorsementRecordKey::Record(record.id), &record);
        }
    }
}
