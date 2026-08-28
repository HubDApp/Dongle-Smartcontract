//! Contract Pause / Emergency Stop module (closes #664).
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
//!
//! ## State Machine
//!
//! ```text
//!               pause(admin)
//!   RUNNING ─────────────────► PAUSED
//!      ▲                          │
//!      └──────────────────────────┘
//!           unpause(admin)
//! ```
//!
//! | State   | `ContractPaused` storage value | Allowed mutations |
//! |---------|--------------------------------|-------------------|
//! | RUNNING | absent or `false`              | All |
//! | PAUSED  | `true`                         | Admin-only recovery functions |
//!
//! Transitions are **idempotent**: pausing an already-paused contract and
//! unpausing an already-running contract both succeed without error.
//!
//! ## Recovery checklist (for operations team)
//!
//! 1. Identify admin address(es) authorised to call `unpause`.
//! 2. Call `is_paused()` to confirm the contract is currently paused.
//! 3. Investigate the incident root cause before unpausing.
//! 4. Call `unpause(admin)` with admin auth.
//! 5. Call `is_paused()` again — must return `false`.
//! 6. Spot-check state integrity: call `get_project`, `get_admin_list`,
//!    `get_fee_config`.  The pause flag is the **only** thing changed by
//!    pause/unpause; all other state is unaffected.
//! 7. Monitor the ledger for a `ContractUnpaused` event (topics:
//!    `["CONTRACT", "UNPAUSED"]`).
//!
//! ## State validation after unpause
//!
//! After calling `unpause`:
//! - `is_paused()` returns `false`.
//! - `get_config()` succeeds and reflects the unpaused state.
//! - All project, review, admin, fee, and verification data is identical to
//!   what it was before the pause — no data is modified by pause/unpause.
//! - All mutating entry points accept calls again.
//!
//! See `tests::pause_state_machine` for automated verification of these guarantees.

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
