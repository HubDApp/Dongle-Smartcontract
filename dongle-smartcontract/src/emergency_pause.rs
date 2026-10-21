//! Contract Pause / Emergency Stop module (closes #664).
//!
//! Allows an admin to halt a defined set of mutating operations during an
//! incident by flipping a single flag (`StorageKey::ContractPaused`).
//!
//! When paused:
//! - The entry points guarded by [`EmergencyPause::require_not_paused`] fail with
//!   `ContractError::ContractPaused`. That gate is currently applied to the
//!   project lifecycle / links / transfer / region calls, `cancel_fee_payment`,
//!   and the changelog write calls in `lib.rs`. It is **not** yet applied to
//!   every mutating path (e.g. `pay_fee`, `add_review`, `request_verification`,
//!   follow/bookmark/endorse) — see the enforcement-surface table in
//!   `docs/EMERGENCY_PAUSE_RECOVERY.md`.
//! - Read-only calls continue to work normally.
//! - Admin-only recovery calls (pause, unpause, admin management, fee config,
//!   verification and review moderation, TTL extensions, …) are never gated.
//!
//! Pause/unpause emit `CONTRACT/PAUSED` / `CONTRACT/UNPAUSED` events and do
//! **not** write an `AdminActionLog` entry.
//!
//! Operational runbook (state machine, recovery checklist, post-unpause state
//! validation): `docs/EMERGENCY_PAUSE_RECOVERY.md`. Not to be confused with the
//! separate, unenforced `ConfigRegistry::set_pause` flag surfaced by
//! `get_config`.
//!
//! Since issue #903 the pause supports **staged recovery**: while the contract
//! is paused, an admin can resume one area at a time (read ops -> core writes
//! -> reviews -> verification -> fees) with
//! `PauseRecovery::resume_operations_phase`, and roll the last resumed area
//! back with `PauseRecovery::rollback_recovery_phase`. The guards below are
//! phase aware: `require_not_paused` covers the core-write phase while
//! `require_phase` covers a specific phase. Operator guide:
//! `docs/PAUSE_RECOVERY_PHASES.md`.

use crate::constants::PAUSE_PHASE_CORE_WRITES;
use crate::errors::ContractError;
use crate::events::{publish_contract_paused_event, publish_contract_unpaused_event};
use crate::pause_recovery::PauseRecovery;
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
    ///
    /// Since issue #903 the contract supports *staged* recovery: while the
    /// contract is paused this guard accepts calls whose recovery phase
    /// (`PAUSE_PHASE_CORE_WRITES`) has been resumed through
    /// `PauseRecovery::resume_operations_phase`.
    pub fn require_not_paused(env: &Env) -> Result<(), ContractError> {
        PauseRecovery::require_phase(env, PAUSE_PHASE_CORE_WRITES)
    }

    /// Guard for operations that belong to a specific recovery phase
    /// (issue #903).
    ///
    /// `phase` is one of the `PAUSE_PHASE_*` constants. While the contract is
    /// paused the operation is only allowed if that phase has been resumed.
    pub fn require_phase(env: &Env, phase: u32) -> Result<(), ContractError> {
        PauseRecovery::require_phase(env, phase)
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

        // A fresh incident starts from a full pause: no recovery phase is live
        // until an operator resumes it explicitly (issue #903).
        PauseRecovery::reset(env);

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

        // The contract is fully operational again — drop any staged recovery
        // state (issue #903).
        PauseRecovery::reset(env);

        publish_contract_unpaused_event(env, admin.clone());

        Ok(())
    }
}
