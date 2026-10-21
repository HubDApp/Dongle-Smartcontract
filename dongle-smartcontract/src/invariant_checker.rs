//! Runtime invariant checks for critical state (#940).
//!
//! Detects corrupted state early: project count consistency, per-project
//! review counts, admin count, and fee balance. Admin-only debug endpoint
//! `verify_contract_invariants` runs enabled checks; violations are appended
//! to an on-chain audit log. Individual checks can be enabled/disabled.

use soroban_sdk::{contracttype, Address, Env, String, Vec};

use crate::admin_manager::AdminManager;
use crate::errors::ContractError;
use crate::storage_keys::StorageKey;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InvariantCheckId {
    ProjectCount,
    ReviewCountPerProject,
    AdminCount,
    FeeBalance,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InvariantViolation {
    pub check: InvariantCheckId,
    pub detail: String,
    pub recorded_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
enum InvariantKey {
    DisabledChecks,
    Violations,
}

pub struct InvariantChecker;

impl InvariantChecker {
    fn disabled(env: &Env) -> Vec<InvariantCheckId> {
        env.storage()
            .persistent()
            .get(&InvariantKey::DisabledChecks)
            .unwrap_or_else(|| Vec::new(env))
    }

    fn is_enabled(env: &Env, check: &InvariantCheckId) -> bool {
        let d = Self::disabled(env);
        !d.iter().any(|c| c == *check)
    }

    /// Enable or disable a single invariant check (admin-only).
    pub fn set_check_enabled(
        env: &Env,
        admin: Address,
        check: InvariantCheckId,
        enabled: bool,
    ) -> Result<(), ContractError> {
        admin.require_auth();
        if !AdminManager::is_admin(env, &admin) {
            return Err(ContractError::AdminOnly);
        }
        let mut d = Self::disabled(env);
        let present = d.iter().any(|c| c == check);
        if enabled && present {
            let mut kept: Vec<InvariantCheckId> = Vec::new(env);
            for i in 0..d.len() {
                if let Some(c) = d.get(i) {
                    if c != check {
                        kept.push_back(c);
                    }
                }
            }
            d = kept;
        } else if !enabled && !present {
            d.push_back(check);
        }
        env.storage()
            .persistent()
            .set(&InvariantKey::DisabledChecks, &d);
        Ok(())
    }

    fn log_violation(env: &Env, check: InvariantCheckId, detail: String) {
        let mut v: Vec<InvariantViolation> = env
            .storage()
            .persistent()
            .get(&InvariantKey::Violations)
            .unwrap_or_else(|| Vec::new(env));
        v.push_back(InvariantViolation {
            check,
            detail,
            recorded_at: env.ledger().timestamp(),
        });
        env.storage()
            .persistent()
            .set(&InvariantKey::Violations, &v);
    }

    fn check_project_count(env: &Env) -> Option<String> {
        let count: u64 = env
            .storage()
            .persistent()
            .get(&StorageKey::ProjectCount)
            .unwrap_or(0);
        let next: u64 = env
            .storage()
            .persistent()
            .get(&StorageKey::NextProjectId)
            .unwrap_or(1);
        // NextProjectId starts at 1; count must never exceed ids issued.
        if next > 0 && count + 1 > next {
            return Some(String::from_str(env, "project count exceeds issued ids"));
        }
        None
    }

    fn check_admin_count(env: &Env) -> Option<String> {
        let admins = AdminManager::get_admin_list(env);
        if admins.is_empty() {
            return Some(String::from_str(env, "admin list empty"));
        }
        let counted = AdminManager::get_admin_count(env);
        if counted != admins.len() {
            return Some(String::from_str(env, "admin count mismatch"));
        }
        None
    }

    fn check_review_counts(env: &Env) -> Option<String> {
        let count: u64 = env
            .storage()
            .persistent()
            .get(&StorageKey::ProjectCount)
            .unwrap_or(0);
        // Bounded scan to keep the debug endpoint cheap.
        let scan = core::cmp::min(count, 50);
        let mut pid = 1u64;
        while pid <= scan {
            let keys: Vec<Address> = env
                .storage()
                .persistent()
                .get(&StorageKey::ProjectReviews(pid))
                .unwrap_or_else(|| Vec::new(env));
            let stats = crate::review_registry::ReviewRegistry::get_project_stats(env, pid);
            if stats.review_count != keys.len() {
                return Some(String::from_str(env, "review count mismatch"));
            }
            pid += 1;
        }
        None
    }

    fn check_fee_balance(env: &Env) -> Option<String> {
        // Fee balance invariant: if any project has a fee-paid flag, a
        // global FeeConfig must exist (otherwise accounting is inconsistent).
        let count: u64 = env
            .storage()
            .persistent()
            .get(&StorageKey::ProjectCount)
            .unwrap_or(0);
        let scan = core::cmp::min(count, 50);
        let mut pid = 1u64;
        while pid <= scan {
            let paid: bool = env
                .storage()
                .persistent()
                .get(&StorageKey::FeePaidForProject(pid))
                .unwrap_or(false);
            if paid {
                let cfg: Option<crate::types::FeeConfig> = env
                    .storage()
                    .persistent()
                    .get(&StorageKey::FeeConfig);
                if cfg.is_none() {
                    return Some(String::from_str(env, "fee paid but no fee config"));
                }
                break;
            }
            pid += 1;
        }
        None
    }

    /// Admin-only debug endpoint: run all enabled checks, log violations,
    /// and return them.
    pub fn verify_contract_invariants(
        env: &Env,
        admin: Address,
    ) -> Result<Vec<InvariantViolation>, ContractError> {
        admin.require_auth();
        if !AdminManager::is_admin(env, &admin) {
            return Err(ContractError::AdminOnly);
        }
        let mut out: Vec<InvariantViolation> = Vec::new(env);
        let checks = [
            InvariantCheckId::ProjectCount,
            InvariantCheckId::ReviewCountPerProject,
            InvariantCheckId::AdminCount,
            InvariantCheckId::FeeBalance,
        ];
        for check in checks {
            if !Self::is_enabled(env, &check) {
                continue;
            }
            let detail = match check {
                InvariantCheckId::ProjectCount => Self::check_project_count(env),
                InvariantCheckId::ReviewCountPerProject => Self::check_review_counts(env),
                InvariantCheckId::AdminCount => Self::check_admin_count(env),
                InvariantCheckId::FeeBalance => Self::check_fee_balance(env),
            };
            if let Some(d) = detail {
                Self::log_violation(env, check.clone(), d.clone());
                out.push_back(InvariantViolation {
                    check: check.clone(),
                    detail: d,
                    recorded_at: env.ledger().timestamp(),
                });
            }
        }
        Ok(out)
    }

    pub fn get_violations(env: &Env) -> Vec<InvariantViolation> {
        env.storage()
            .persistent()
            .get(&InvariantKey::Violations)
            .unwrap_or_else(|| Vec::new(env))
    }
}
