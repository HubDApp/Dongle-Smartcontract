//! Storage TTL (Time To Live) management for Soroban persistent storage.
//!
//! This module provides utilities to extend TTL for contract data, ensuring
//! critical information persists and doesn't expire unexpectedly.

use crate::constants::*;
use crate::storage_keys::{ExtensionKey, StorageKey};
use soroban_sdk::{Address, Env, String, Vec};

/// Storage manager for TTL operations
pub struct StorageManager;

impl StorageManager {
    // ── Critical Data TTL Management ──────────────────────────────────────

    /// Extend TTL for admin-related storage (admin list, individual admin entries)
    pub fn extend_admin_ttl(env: &Env, admin: &Address) {
        if env
            .storage()
            .persistent()
            .has(&StorageKey::Admin(admin.clone()))
        {
            env.storage().persistent().extend_ttl(
                &StorageKey::Admin(admin.clone()),
                LEDGER_THRESHOLD_CRITICAL,
                LEDGER_BUMP_CRITICAL,
            );
        }
    }

    /// Extend TTL for the admin list
    pub fn extend_admin_list_ttl(env: &Env) {
        if env.storage().persistent().has(&StorageKey::AdminList) {
            env.storage().persistent().extend_ttl(
                &StorageKey::AdminList,
                LEDGER_THRESHOLD_CRITICAL,
                LEDGER_BUMP_CRITICAL,
            );
        }
    }

    /// Extend TTL for fee configuration
    pub fn extend_fee_config_ttl(env: &Env) {
        if env.storage().persistent().has(&StorageKey::FeeConfig) {
            env.storage().persistent().extend_ttl(
                &StorageKey::FeeConfig,
                LEDGER_THRESHOLD_CRITICAL,
                LEDGER_BUMP_CRITICAL,
            );
        }
    }

    /// Extend TTL for treasury address
    pub fn extend_treasury_ttl(env: &Env) {
        if env.storage().persistent().has(&StorageKey::Treasury) {
            env.storage().persistent().extend_ttl(
                &StorageKey::Treasury,
                LEDGER_THRESHOLD_CRITICAL,
                LEDGER_BUMP_CRITICAL,
            );
        }
    }

    // ── Project Data TTL Management ───────────────────────────────────────

    /// Extend TTL for a specific project
    pub fn extend_project_ttl(env: &Env, project_id: u64) {
        if env
            .storage()
            .persistent()
            .has(&StorageKey::Project(project_id))
        {
            env.storage().persistent().extend_ttl(
                &StorageKey::Project(project_id),
                LEDGER_THRESHOLD_PROJECT,
                LEDGER_BUMP_PROJECT,
            );
        }
    }

    /// Extend TTL for project count
    pub fn extend_project_count_ttl(env: &Env) {
        if env.storage().persistent().has(&StorageKey::ProjectCount) {
            env.storage().persistent().extend_ttl(
                &StorageKey::ProjectCount,
                LEDGER_THRESHOLD_PROJECT,
                LEDGER_BUMP_PROJECT,
            );
        }
    }

    /// Extend TTL for project by name mapping
    pub fn extend_project_by_name_ttl(env: &Env, name: &String) {
        if env
            .storage()
            .persistent()
            .has(&StorageKey::ProjectByName(name.clone()))
        {
            env.storage().persistent().extend_ttl(
                &StorageKey::ProjectByName(name.clone()),
                LEDGER_THRESHOLD_PROJECT,
                LEDGER_BUMP_PROJECT,
            );
        }
    }

    /// Extend TTL for category projects index
    pub fn extend_category_projects_ttl(env: &Env, category: &String) {
        if env
            .storage()
            .persistent()
            .has(&StorageKey::CategoryProjects(category.clone()))
        {
            env.storage().persistent().extend_ttl(
                &StorageKey::CategoryProjects(category.clone()),
                LEDGER_THRESHOLD_PROJECT,
                LEDGER_BUMP_PROJECT,
            );
        }
    }

    /// Extend TTL for project stats
    pub fn extend_project_stats_ttl(env: &Env, project_id: u64) {
        if env
            .storage()
            .persistent()
            .has(&StorageKey::ProjectStats(project_id))
        {
            env.storage().persistent().extend_ttl(
                &StorageKey::ProjectStats(project_id),
                LEDGER_THRESHOLD_PROJECT,
                LEDGER_BUMP_PROJECT,
            );
        }
    }

    /// Extend TTL for a project's dependency index + dependency records.
    pub fn extend_project_dependency_ttl(env: &Env, project_id: u64) {
        if env
            .storage()
            .persistent()
            .has(&ExtensionKey::ProjectDependencyKeys(project_id))
        {
            env.storage().persistent().extend_ttl(
                &ExtensionKey::ProjectDependencyKeys(project_id),
                LEDGER_THRESHOLD_PROJECT,
                LEDGER_BUMP_PROJECT,
            );
        }

        if let Some(keys) = env
            .storage()
            .persistent()
            .get::<_, soroban_sdk::Vec<String>>(&ExtensionKey::ProjectDependencyKeys(project_id))
        {
            for i in 0..keys.len() {
                if let Some(k) = keys.get(i) {
                    if env
                        .storage()
                        .persistent()
                        .has(&ExtensionKey::ProjectDependency(project_id, k.clone()))
                    {
                        env.storage().persistent().extend_ttl(
                            &ExtensionKey::ProjectDependency(project_id, k.clone()),
                            LEDGER_THRESHOLD_PROJECT,
                            LEDGER_BUMP_PROJECT,
                        );
                    }
                }
            }
        }
    }

    // ── Review Data TTL Management ────────────────────────────────────────

    /// Extend TTL for a specific review
    pub fn extend_review_ttl(env: &Env, project_id: u64, reviewer: &Address) {
        if env
            .storage()
            .persistent()
            .has(&StorageKey::Review(project_id, reviewer.clone()))
        {
            env.storage().persistent().extend_ttl(
                &StorageKey::Review(project_id, reviewer.clone()),
                LEDGER_THRESHOLD_REVIEW,
                LEDGER_BUMP_REVIEW,
            );
        }
    }

    /// Extend TTL for project reviews list
    pub fn extend_project_reviews_ttl(env: &Env, project_id: u64) {
        if env
            .storage()
            .persistent()
            .has(&StorageKey::ProjectReviews(project_id))
        {
            env.storage().persistent().extend_ttl(
                &StorageKey::ProjectReviews(project_id),
                LEDGER_THRESHOLD_REVIEW,
                LEDGER_BUMP_REVIEW,
            );
        }
    }

    // ── Verification Data TTL Management ──────────────────────────────────

    /// Extend TTL for verification record
    pub fn extend_verification_ttl(env: &Env, project_id: u64) {
        // 1. Extend current verification record TTL
        if env
            .storage()
            .persistent()
            .has(&StorageKey::Verification(project_id))
        {
            env.storage().persistent().extend_ttl(
                &StorageKey::Verification(project_id),
                LEDGER_THRESHOLD_VERIFICATION,
                LEDGER_BUMP_VERIFICATION,
            );
        }

        // 2. Extend history vector TTL
        if env
            .storage()
            .persistent()
            .has(&StorageKey::ProjectVerificationHistory(project_id))
        {
            env.storage().persistent().extend_ttl(
                &StorageKey::ProjectVerificationHistory(project_id),
                LEDGER_THRESHOLD_VERIFICATION,
                LEDGER_BUMP_VERIFICATION,
            );
            // 3. Extend all historical record TTLs
            if let Some(history) = env.storage().persistent().get::<_, soroban_sdk::Vec<u64>>(
                &StorageKey::ProjectVerificationHistory(project_id),
            ) {
                for i in 0..history.len() {
                    if let Some(req_id) = history.get(i) {
                        if env
                            .storage()
                            .persistent()
                            .has(&StorageKey::VerificationRecord(req_id))
                        {
                            env.storage().persistent().extend_ttl(
                                &StorageKey::VerificationRecord(req_id),
                                LEDGER_THRESHOLD_VERIFICATION,
                                LEDGER_BUMP_VERIFICATION,
                            );
                        }
                    }
                }
            }
        }
    }

    /// Extend TTL for fee payment record
    pub fn extend_fee_paid_ttl(env: &Env, project_id: u64) {
        if env
            .storage()
            .persistent()
            .has(&StorageKey::FeePaidForProject(project_id))
        {
            env.storage().persistent().extend_ttl(
                &StorageKey::FeePaidForProject(project_id),
                LEDGER_THRESHOLD_VERIFICATION,
                LEDGER_BUMP_VERIFICATION,
            );
        }
    }

    /// Extend TTL for project bounty url (removed - not part of core storage)
    pub fn extend_project_bounty_url_ttl(env: &Env, project_id: u64) {
        // Bounty URL storage removed - not part of core implementation
    }

    // ── User Data TTL Management ──────────────────────────────────────────

    /// Extend TTL for owner projects list
    pub fn extend_owner_projects_ttl(env: &Env, owner: &Address) {
        if env
            .storage()
            .persistent()
            .has(&StorageKey::OwnerProjects(owner.clone()))
        {
            env.storage().persistent().extend_ttl(
                &StorageKey::OwnerProjects(owner.clone()),
                LEDGER_THRESHOLD_USER,
                LEDGER_BUMP_USER,
            );
        }
    }

    /// Extend TTL for user reviews list
    pub fn extend_user_reviews_ttl(env: &Env, user: &Address) {
        if env
            .storage()
            .persistent()
            .has(&StorageKey::UserReviews(user.clone()))
        {
            env.storage().persistent().extend_ttl(
                &StorageKey::UserReviews(user.clone()),
                LEDGER_THRESHOLD_USER,
                LEDGER_BUMP_USER,
            );
        }
    }

    /// Extend TTL for owner project count
    pub fn extend_owner_project_count_ttl(env: &Env, owner: &Address) {
        if env
            .storage()
            .persistent()
            .has(&StorageKey::OwnerProjectCount(owner.clone()))
        {
            env.storage().persistent().extend_ttl(
                &StorageKey::OwnerProjectCount(owner.clone()),
                LEDGER_THRESHOLD_USER,
                LEDGER_BUMP_USER,
            );
        }
    }

    // ── Composite Operations ──────────────────────────────────────────────

    /// Extend TTL for project followers list and count
    pub fn extend_followers_ttl(env: &Env, project_id: u64) {
        if env
            .storage()
            .persistent()
            .has(&ExtensionKey::ProjectFollowers(project_id))
        {
            env.storage().persistent().extend_ttl(
                &ExtensionKey::ProjectFollowers(project_id),
                LEDGER_THRESHOLD_USER,
                LEDGER_BUMP_USER,
            );
        }
        if env
            .storage()
            .persistent()
            .has(&ExtensionKey::FollowerCount(project_id))
        {
            env.storage().persistent().extend_ttl(
                &ExtensionKey::FollowerCount(project_id),
                LEDGER_THRESHOLD_USER,
                LEDGER_BUMP_USER,
            );
        }
    }

    /// Extend TTL for user bookmarks list
    pub fn extend_user_bookmarks_ttl(env: &Env, user: &Address) {
        if env
            .storage()
            .persistent()
            .has(&ExtensionKey::UserBookmarks(user.clone()))
        {
            env.storage().persistent().extend_ttl(
                &ExtensionKey::UserBookmarks(user.clone()),
                LEDGER_THRESHOLD_USER,
                LEDGER_BUMP_USER,
            );
        }
    }

    /// Extend TTL for project endorsements list and count
    pub fn extend_endorsements_ttl(env: &Env, project_id: u64) {
        if env
            .storage()
            .persistent()
            .has(&ExtensionKey::ProjectEndorsements(project_id))
        {
            env.storage().persistent().extend_ttl(
                &ExtensionKey::ProjectEndorsements(project_id),
                LEDGER_THRESHOLD_USER,
                LEDGER_BUMP_USER,
            );
        }
        if env
            .storage()
            .persistent()
            .has(&ExtensionKey::EndorsementCount(project_id))
        {
            env.storage().persistent().extend_ttl(
                &ExtensionKey::EndorsementCount(project_id),
                LEDGER_THRESHOLD_USER,
                LEDGER_BUMP_USER,
            );
        }
    }

    /// Extend TTL for user subscriptions list
    pub fn extend_user_subscriptions_ttl(env: &Env, user: &Address) {
        if env
            .storage()
            .persistent()
            .has(&ExtensionKey::UserSubscriptions(user.clone()))
        {
            env.storage().persistent().extend_ttl(
                &ExtensionKey::UserSubscriptions(user.clone()),
                LEDGER_THRESHOLD_USER,
                LEDGER_BUMP_USER,
            );
        }
    }

    /// Extend TTL for a project's maintainer list
    pub fn extend_project_maintainers_ttl(env: &Env, project_id: u64) {
        if env
            .storage()
            .persistent()
            .has(&StorageKey::ProjectMaintainers(project_id))
        {
            env.storage().persistent().extend_ttl(
                &StorageKey::ProjectMaintainers(project_id),
                LEDGER_THRESHOLD_PROJECT,
                LEDGER_BUMP_PROJECT,
            );
        }
    }

    /// Extend TTL for all project-related data (project + stats + name mapping + maintainers)
    pub fn extend_project_full_ttl(env: &Env, project_id: u64, name: &String) {
        Self::extend_project_ttl(env, project_id);
        Self::extend_project_stats_ttl(env, project_id);
        Self::extend_project_by_name_ttl(env, name);
        Self::extend_project_maintainers_ttl(env, project_id);
    }

    /// Extend TTL for all admin-related data
    pub fn extend_all_admin_ttl(env: &Env, admin: &Address) {
        Self::extend_admin_ttl(env, admin);
        Self::extend_admin_list_ttl(env);
    }

    /// Extend TTL for all critical contract configuration
    pub fn extend_critical_config_ttl(env: &Env) {
        Self::extend_admin_list_ttl(env);
        Self::extend_fee_config_ttl(env);
        Self::extend_treasury_ttl(env);
    }

    /// Extend TTL for a claim request
    pub fn extend_claim_request_ttl(env: &Env, claim_request_id: u64) {
        if env
            .storage()
            .persistent()
            .has(&ExtensionKey::ClaimRequest(claim_request_id))
        {
            env.storage().persistent().extend_ttl(
                &ExtensionKey::ClaimRequest(claim_request_id),
                LEDGER_THRESHOLD_PROJECT,
                LEDGER_BUMP_PROJECT,
            );
        }
    }

    /// Extend TTL for all claim-related data for a project
    pub fn extend_project_claims_ttl(env: &Env, project_id: u64) {
        if env
            .storage()
            .persistent()
            .has(&ExtensionKey::ProjectClaimRequests(project_id))
        {
            env.storage().persistent().extend_ttl(
                &ExtensionKey::ProjectClaimRequests(project_id),
                LEDGER_THRESHOLD_PROJECT,
                LEDGER_BUMP_PROJECT,
            );
            // Extend all individual claim request TTLs
            if let Some(request_ids) = env
                .storage()
                .persistent()
                .get::<_, Vec<u64>>(&ExtensionKey::ProjectClaimRequests(project_id))
            {
                for i in 0..request_ids.len() {
                    if let Some(request_id) = request_ids.get(i) {
                        Self::extend_claim_request_ttl(env, request_id);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DongleContract;
    use soroban_sdk::{testutils::Address as _, Env};

    #[test]
    fn test_extend_admin_ttl() {
        let env = Env::default();
        let contract_id = env.register(DongleContract, ());
        let admin = Address::generate(&env);

        // Initialize contract with admin
        env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .set(&StorageKey::Admin(admin.clone()), &true);

            let mut admins = soroban_sdk::Vec::new(&env);
            admins.push_back(admin.clone());
            env.storage()
                .persistent()
                .set(&StorageKey::AdminList, &admins);

            // Extend TTL should not panic
            StorageManager::extend_admin_ttl(&env, &admin);
            StorageManager::extend_admin_list_ttl(&env);
        });
    }

    #[test]
    fn test_extend_project_ttl() {
        let env = Env::default();
        let contract_id = env.register(DongleContract, ());
        let project_id = 1u64;

        env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .set(&StorageKey::Project(project_id), &true);

            // Extend TTL should not panic
            StorageManager::extend_project_ttl(&env, project_id);
        });
    }

    #[test]
    fn test_extend_critical_config_ttl() {
        let env = Env::default();
        let contract_id = env.register(DongleContract, ());

        env.as_contract(&contract_id, || {
            let admins: soroban_sdk::Vec<Address> = soroban_sdk::Vec::new(&env);
            env.storage()
                .persistent()
                .set(&StorageKey::AdminList, &admins);

            // Should not panic
            StorageManager::extend_critical_config_ttl(&env);
        });
    }

    #[test]
    fn test_extend_project_full_ttl() {
        let env = Env::default();
        let contract_id = env.register(DongleContract, ());
        let project_id = 1u64;
        let name = String::from_str(&env, "TestProject");

        env.as_contract(&contract_id, || {
            env.storage()
                .persistent()
                .set(&StorageKey::Project(project_id), &true);
            env.storage()
                .persistent()
                .set(&StorageKey::ProjectStats(project_id), &true);
            env.storage()
                .persistent()
                .set(&StorageKey::ProjectByName(name.clone()), &project_id);

            // Should not panic
            StorageManager::extend_project_full_ttl(&env, project_id, &name);
        });
    }
}
