//! Read-only configuration view + global pause flag.
//!
//! Frontends and indexers previously had to call `get_fee_config`,
//! `get_admin_count`, `get_admin_approval_threshold`, … independently to
//! rebuild the contract configuration. This module exposes the same data
//! through a single `get_config` entry point and a stable on-chain type
//! (`ContractConfigView`).
//!
//! ## Scope: pause flag is *not* enforced
//! [`set_pause`] writes the admin's intent to storage and emits a matching
//! audit-log entry, but **does not block mutating entry points**. Wiring
//! pause enforcement through `register_project`, `pay_fee`, … is a
//! separate, future ticket. Frontends should treat the flag as advisory
//! for now.

use crate::admin_action_log::AdminActionLog;
use crate::auth;
use crate::constants::{
    CONTRACT_VERSION, LEDGER_BUMP_CRITICAL, LEDGER_THRESHOLD_CRITICAL, MAX_DESCRIPTION_LEN,
    MAX_NAME_LEN, MAX_PAGE_LIMIT, MAX_PROJECTS_PER_USER, MAX_REVIEWS_PER_PROJECT,
    VERIFICATION_VALIDITY_PERIOD,
};
use crate::errors::ContractError;
use crate::storage_keys::{ExtensionKey, StorageKey};
use crate::types::{AdminActionType, ContractConfigView, ContractLimits, FeeConfig};
use soroban_sdk::{Address, Env, String};

pub struct ConfigRegistry;

impl ConfigRegistry {
    /// Returns true if the contract has been paused by an admin via
    /// [`ConfigRegistry::set_pause`]. Defaults to `false` when no flag
    /// has ever been set, which keeps post-init reads well-defined.
    ///
    /// Bumps the critical-config TTL **iff** the key has ever been
    /// written. The absent-key case returns `false` and is intentionally
    /// not bumped, since there is nothing to preserve yet — `set_pause`
    /// bumps TTL on the corresponding write.
    pub fn is_paused(env: &Env) -> bool {
        let key = StorageKey::ContractPaused;
        let paused: bool = env.storage().persistent().get(&key).unwrap_or(false);
        if env.storage().persistent().has(&key) {
            env.storage().persistent().extend_ttl(
                &key,
                LEDGER_THRESHOLD_CRITICAL,
                LEDGER_BUMP_CRITICAL,
            );
        }
        paused
    }

    /// Toggle the global pause flag (admin only). Records an
    /// `AdminActionLog` entry for audit parity with every other admin
    /// mutation in this contract.
    ///
    /// # Returns
    /// The pause state **before** the call. Callers that only care about
    /// success should ignore the value; callers that need to react to a
    /// state transition can compare the return value against the new
    /// value passed in.
    pub fn set_pause(env: &Env, admin: Address, paused: bool) -> Result<bool, ContractError> {
        auth::require_admin_auth(env, &admin)?;

        let previous = Self::is_paused(env);
        let key = StorageKey::ContractPaused;
        env.storage().persistent().set(&key, &paused);
        env.storage().persistent().extend_ttl(
            &key,
            LEDGER_THRESHOLD_CRITICAL,
            LEDGER_BUMP_CRITICAL,
        );

        // Audit the transition. Use distinct variants so the admin log
        // is unambiguous (rather than a single "PauseChanged" variant
        // where readers have to compare prev/new to interpret the row).
        if paused {
            AdminActionLog::record_action(
                env,
                admin,
                AdminActionType::ContractPaused,
                None,
                None,
                None,
            );
        } else {
            AdminActionLog::record_action(
                env,
                admin,
                AdminActionType::ContractResumed,
                None,
                None,
                None,
            );
        }

        Ok(previous)
    }

    /// Build and return the full contract configuration snapshot.
    ///
    /// Composed from existing storage: fee config
    /// (`StorageKey::FeeConfig`), treasury address (`StorageKey::Treasury`),
    /// admin count + threshold (`AdminManager`), the pause flag
    /// (`StorageKey::ContractPaused`), and the static `ContractLimits` derived
    /// from `constants.rs`.
    ///
    /// # Behaviour absent `set_fee`
    /// If `set_fee` has never been called the view still returns a fully
    /// populated `ContractConfigView` with **zero-fee defaults**
    /// (`token: None`, both fee amounts zero). Frontends can distinguish
    /// "never configured" from "configured-with-zero-fees" via
    /// `treasury: Option<Address>` — only `set_fee` populates it.
    /// This contract always succeeds so the post-init snapshot in the
    /// acceptance criteria remains readable.
    pub fn get_config(env: &Env) -> Result<ContractConfigView, ContractError> {
        let fees: FeeConfig = env
            .storage()
            .persistent()
            .get(&StorageKey::FeeConfig)
            .unwrap_or(FeeConfig {
                token: None,
                verification_fee: 0,
                registration_fee: 0,
            });

        let treasury: Option<Address> = env.storage().persistent().get(&StorageKey::Treasury);
        let paused = Self::is_paused(env);
        let admin_count = crate::admin_manager::AdminManager::get_admin_count(env);
        let admin_approval_threshold =
            crate::admin_manager::AdminManager::get_admin_approval_threshold(env);

        Ok(ContractConfigView {
            version: String::from_str(env, CONTRACT_VERSION),
            admin_count,
            admin_approval_threshold,
            paused,
            treasury,
            fees,
            limits: ContractLimits {
                max_page_limit: MAX_PAGE_LIMIT,
                max_projects_per_user: MAX_PROJECTS_PER_USER,
                max_reviews_per_project: MAX_REVIEWS_PER_PROJECT,
                max_name_len: MAX_NAME_LEN as u32,
                max_description_len: MAX_DESCRIPTION_LEN as u32,
                verification_validity_period: VERIFICATION_VALIDITY_PERIOD,
            },
        })
    }
}
