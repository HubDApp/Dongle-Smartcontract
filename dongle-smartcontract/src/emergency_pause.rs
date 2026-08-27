//! Contract Pause / Emergency Stop module.
//!
//! Allows an admin to pause all mutating operations during an incident.
//! When paused:
//! - Mutating calls (registration, reviews, fees, verification, etc.) fail with
//!   `ContractError::ContractPaused`.
//! - Read-only calls continue to work normally.
//! - Admin recovery functions (pause, unpause, admin management, fee config,
//!   verification approval/rejection/revocation, review moderation, TTL extensions)
//!   are still allowed.
//!
//! Pause/unpause emit `ContractPaused` / `ContractUnpaused` events.

use crate::errors::ContractError;
use crate::events::{publish_contract_paused_event, publish_contract_unpaused_event};
use crate::storage_keys::StorageKey;
use soroban_sdk::{Address, Env};

pub struct EmergencyPause;

impl EmergencyPause {
    /// Check whether the contract is currently paused.
    pub fn is_paused(env: &Env) -> bool {
        env.storage()
            .persistent()
            .get(&StorageKey::ContractPaused)
            .unwrap_or(false)
    }

    /// Guard: return `ContractError::ContractPaused` if the contract is paused.
    ///
    /// Call this at the top of every mutating function that should be blocked
    /// during an emergency.
    pub fn require_not_paused(env: &Env) -> Result<(), ContractError> {
        if Self::is_paused(env) {
            Err(ContractError::ContractPaused)
        } else {
            Ok(())
        }
    }

    /// Pause the contract (admin-only).
    ///
    /// After this call, all non-admin mutating operations will be rejected.
    /// Emits a `ContractPaused` event.
    pub fn pause(env: &Env, admin: &Address) -> Result<(), ContractError> {
        admin.require_auth();
        crate::admin_manager::AdminManager::require_admin(env, admin)?;

        env.storage()
            .persistent()
            .set(&StorageKey::ContractPaused, &true);

        publish_contract_paused_event(env, admin.clone());

        Ok(())
    }

    /// Unpause the contract (admin-only).
    ///
    /// Restores normal operation. Emits a `ContractUnpaused` event.
    pub fn unpause(env: &Env, admin: &Address) -> Result<(), ContractError> {
        admin.require_auth();
        crate::admin_manager::AdminManager::require_admin(env, admin)?;

        env.storage()
            .persistent()
            .set(&StorageKey::ContractPaused, &false);

        publish_contract_unpaused_event(env, admin.clone());

        Ok(())
    }
}
