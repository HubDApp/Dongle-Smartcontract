//! Contract Pause / Emergency Stop module (closes #664, extends #901, #902, #904).
//!
//! Allows an admin to halt a defined set of mutating operations during an
//! incident.  The pause state is **fine-grained** (issue #902): instead of a
//! single boolean the contract stores a [`PauseState`] describing which
//! *family* of operations is blocked (verification, fees, reviews, or
//! everything).
//!
//! When paused:
//! - The entry points guarded by [`EmergencyPause::require_not_paused`] fail with
//!   `ContractError::ContractPaused`.  The gate is applied to the
//!   project lifecycle / links / transfer / region calls, `cancel_fee_payment`,
//!   and the changelog write calls in `lib.rs`.
//! - Read-only calls continue to work normally — a read is never blocked by
//!   the pause gate ([`PauseOperation::Read`] is always allowed).
//! - Admin-only recovery calls (pause, unpause, phase resumption, admin
//!   management, fee config, verification and review moderation, TTL
//!   extensions, …) are never gated.
//!
//! ## Pause reasons (issue #901)
//!
//! A pause can record a human-readable reason and an estimated resume time.
//! Both are surfaced by the `get_contract_status` endpoint, included in the
//! `CONTRACT/PAUSED` event, and pushed to the addresses that subscribed to
//! pause notifications.
//!
//! ## Staged recovery (issue #904)
//!
//! `resume_operations_phase` walks the contract back to life one area at a
//! time following the plan *read ops → reviews → verification → fees*.
//! The current phase is stored on the pause record, transitions emit a
//! `CONTRACT/PHASE` event, and a phase can be rolled back with
//! `rollback_operations_phase`.  See `PAUSE_RECOVERY_PHASES.md`.
//!
//! ## Pause matrix (issue #902)
//!
//! See `PAUSE_MATRIX.md` for the full table of which operation families are
//! blocked by which [`PauseState`].

use crate::admin_manager::AdminManager;
use crate::constants::{MAX_PAUSE_REASON_LEN, MAX_PHASE_HISTORY_LEN, MAX_RESUME_PHASE};
use crate::errors::ContractError;
use crate::events::{
    publish_contract_paused_event, publish_contract_unpaused_event,
    publish_pause_phase_changed_event,
};
use crate::notification_registry::NotificationRegistry;
use crate::storage_keys::{PauseKey, StorageKey};
use crate::types::{ContractStatus, EmergencyPauseState, PauseOperation, PauseState};
use soroban_sdk::{Address, Env, String, Vec};

pub struct EmergencyPause;

impl EmergencyPause {
    // ── State accessors ────────────────────────────────────────────────────

    /// Read the full emergency-pause record.
    ///
    /// Deployments that predate the fine-grained pause (#902) only have the
    /// legacy boolean flag under `StorageKey::ContractPaused`; for those the
    /// record is derived on the fly (`PausedAll` / `Running`).
    pub fn get_state(env: &Env) -> EmergencyPauseState {
        let stored: Option<EmergencyPauseState> =
            env.storage().persistent().get(&PauseKey::PauseState);
        match stored {
            Some(state) => state,
            None => {
                let legacy: bool = env
                    .storage()
                    .persistent()
                    .get(&StorageKey::ContractPaused)
                    .unwrap_or(false);
                EmergencyPauseState {
                    state: if legacy {
                        PauseState::PausedAll
                    } else {
                        PauseState::Running
                    },
                    pause_reason: None,
                    estimated_resume: None,
                    paused_at: 0,
                    resume_phase: 0,
                }
            }
        }
    }

    /// Persist the emergency-pause record and keep the legacy boolean flag in
    /// sync so that `get_config` keeps reporting the pause state.
    fn save_state(env: &Env, state: &EmergencyPauseState) {
        env.storage().persistent().set(&PauseKey::PauseState, state);
        env.storage()
            .persistent()
            .set(&StorageKey::ContractPaused, &(state.state != PauseState::Running));
    }

    /// Current fine-grained pause state.
    pub fn get_pause_state(env: &Env) -> PauseState {
        Self::get_state(env).state
    }

    /// Check whether the contract is currently paused (in any capacity).
    pub fn is_paused(env: &Env) -> bool {
        Self::get_pause_state(env) != PauseState::Running
    }

    /// Reason recorded for the current pause (issue #901).
    pub fn get_pause_reason(env: &Env) -> Option<String> {
        Self::get_state(env).pause_reason
    }

    /// Estimated resume time (Unix timestamp) of the current pause (issue #901).
    pub fn get_estimated_resume(env: &Env) -> Option<u64> {
        Self::get_state(env).estimated_resume
    }

    /// Current staged-recovery phase (issue #904).
    pub fn get_resume_phase(env: &Env) -> u32 {
        Self::get_state(env).resume_phase
    }

    /// Ordered history of staged-recovery phase transitions (issue #904).
    pub fn get_phase_history(env: &Env) -> Vec<u32> {
        env.storage()
            .persistent()
            .get(&PauseKey::PhaseHistory)
            .unwrap_or_else(|| Vec::new(env))
    }

    // ── Pause checks ───────────────────────────────────────────────────────

    /// Returns `true` when the given operation family is currently blocked.
    ///
    /// * `PauseOperation::Read` is **never** blocked.
    /// * `PauseOperation::All` (generic mutations) is blocked by
    ///   `PausedAll` and, during a staged recovery, until the final phase is
    ///   reached.
    pub fn is_operation_paused(env: &Env, operation: PauseOperation) -> bool {
        // Read operations are always available, even in an emergency.
        if operation == PauseOperation::Read {
            return false;
        }

        let state = Self::get_state(env);

        let blocked_by_state = match state.state {
            PauseState::Running => false,
            PauseState::PausedAll => true,
            PauseState::PausedVerification => operation == PauseOperation::Verification,
            PauseState::PausedFees => operation == PauseOperation::Fees,
            PauseState::PausedReviews => operation == PauseOperation::Reviews,
            PauseState::PausedVerificationAndFees => {
                operation == PauseOperation::Verification || operation == PauseOperation::Fees
            }
        };
        if blocked_by_state {
            return true;
        }

        // During a staged recovery generic mutations stay closed until the
        // final phase ("fees resumed") is reached.
        if operation == PauseOperation::All
            && state.resume_phase > 0
            && state.resume_phase < MAX_RESUME_PHASE
        {
            return true;
        }

        false
    }

    /// Guard: return `ContractError::ContractPaused` if the given operation
    /// family is blocked by the current pause state.
    ///
    /// Call this at the top of every mutating function that should be blocked
    /// during an emergency, passing the family the call belongs to.
    pub fn require_not_paused(
        env: &Env,
        operation: PauseOperation,
    ) -> Result<(), ContractError> {
        if Self::is_operation_paused(env, operation) {
            Err(ContractError::ContractPaused)
        } else {
            Ok(())
        }
    }

    /// The operation families that are currently blocked.
    pub fn paused_operations(env: &Env) -> Vec<PauseOperation> {
        let mut blocked = Vec::new(env);
        if Self::is_operation_paused(env, PauseOperation::All) {
            blocked.push_back(PauseOperation::All);
        }
        if Self::is_operation_paused(env, PauseOperation::Verification) {
            blocked.push_back(PauseOperation::Verification);
        }
        if Self::is_operation_paused(env, PauseOperation::Fees) {
            blocked.push_back(PauseOperation::Fees);
        }
        if Self::is_operation_paused(env, PauseOperation::Reviews) {
            blocked.push_back(PauseOperation::Reviews);
        }
        blocked
    }

    /// Full operational status of the contract (issue #901).
    pub fn get_contract_status(env: &Env) -> ContractStatus {
        let state = Self::get_state(env);
        ContractStatus {
            is_paused: state.state != PauseState::Running,
            pause_state: state.state,
            pause_reason: state.pause_reason,
            estimated_resume: state.estimated_resume,
            paused_at: state.paused_at,
            resume_phase: state.resume_phase,
            paused_operations: Self::paused_operations(env),
        }
    }

    // ── Transitions ────────────────────────────────────────────────────────

    /// Pause the contract (admin-only).
    ///
    /// After this call, all non-admin mutating operations will be rejected.
    /// Emits a `ContractPaused` event and notifies pause subscribers.
    pub fn pause(env: &Env, admin: &Address) -> Result<(), ContractError> {
        admin.require_auth();
        AdminManager::require_admin(env, admin)?;

        Self::apply_state(
            env,
            PauseState::PausedAll,
            None,
            None,
            0,
            admin,
        )
    }

    /// Pause the contract with a human-readable reason and an estimated
    /// resume time (issue #901).
    ///
    /// * `reason` — operator note explaining why the contract was paused
    ///   (1..=`MAX_PAUSE_REASON_LEN` bytes).
    /// * `estimated_resume` — expected resume time as a Unix timestamp.
    ///   `0` means "no estimate" and is stored as `None`.
    pub fn set_pause_with_reason(
        env: &Env,
        admin: &Address,
        reason: String,
        estimated_resume: u64,
    ) -> Result<(), ContractError> {
        admin.require_auth();
        AdminManager::require_admin(env, admin)?;

        Self::validate_reason(&reason)?;

        let estimate = if estimated_resume == 0 {
            None
        } else {
            Some(estimated_resume)
        };

        Self::apply_state(
            env,
            PauseState::PausedAll,
            Some(reason),
            estimate,
            0,
            admin,
        )
    }

    /// Pause only a specific family of operations (issue #902).
    ///
    /// `state` selects which operations are blocked; `reason` and
    /// `estimated_resume` are optional and behave as in
    /// [`EmergencyPause::set_pause_with_reason`].  Passing
    /// [`PauseState::Running`] resumes the contract.
    pub fn set_pause_state(
        env: &Env,
        admin: &Address,
        state: PauseState,
        reason: Option<String>,
        estimated_resume: Option<u64>,
    ) -> Result<(), ContractError> {
        admin.require_auth();
        AdminManager::require_admin(env, admin)?;

        if let Some(ref r) = reason {
            Self::validate_reason(r)?;
        }

        let estimate = match estimated_resume {
            Some(0) => None,
            other => other,
        };

        if state == PauseState::Running {
            return Self::unpause(env, admin);
        }

        Self::apply_state(env, state, reason, estimate, 0, admin)
    }

    /// Unpause the contract (admin-only).
    ///
    /// Restores normal operation, clears the pause reason and resets the
    /// staged-recovery phase.  Emits a `ContractUnpaused` event.
    pub fn unpause(env: &Env, admin: &Address) -> Result<(), ContractError> {
        admin.require_auth();
        AdminManager::require_admin(env, admin)?;

        let state = EmergencyPauseState {
            state: PauseState::Running,
            pause_reason: None,
            estimated_resume: None,
            paused_at: env.ledger().timestamp(),
            resume_phase: 0,
        };
        Self::save_state(env, &state);

        publish_contract_unpaused_event(env, admin.clone());
        Ok(())
    }

    /// Resume the contract up to a given staged-recovery phase (issue #904).
    ///
    /// The recovery plan is *read ops → reviews → verification → fees*:
    ///
    /// | Phase | Resumed |
    /// |-------|---------|
    /// | 0 | nothing (fully paused) |
    /// | 1 | read operations |
    /// | 2 | reviews |
    /// | 3 | verification |
    /// | 4 | fees — the contract is fully running again |
    ///
    /// Calling with a phase lower than the current one performs a rollback;
    /// [`EmergencyPause::rollback_operations_phase`] is the explicit form.
    pub fn resume_operations_phase(
        env: &Env,
        admin: &Address,
        phase: u32,
    ) -> Result<(), ContractError> {
        admin.require_auth();
        AdminManager::require_admin(env, admin)?;

        if phase > MAX_RESUME_PHASE {
            return Err(ContractError::InvalidPausePhase);
        }

        let current = Self::get_state(env);
        let previous_phase = current.resume_phase;

        let state = match phase {
            0 | 1 => PauseState::PausedAll,
            2 => PauseState::PausedVerificationAndFees,
            3 => PauseState::PausedFees,
            _ => PauseState::Running,
        };

        let fully_resumed = state == PauseState::Running;

        let updated = EmergencyPauseState {
            state,
            // The reason stays visible until the contract is fully running
            // again so that operators can see why the incident started.
            pause_reason: if fully_resumed {
                None
            } else {
                current.pause_reason.clone()
            },
            estimated_resume: if fully_resumed {
                None
            } else {
                current.estimated_resume
            },
            paused_at: current.paused_at,
            resume_phase: phase,
        };
        Self::save_state(env, &updated);

        // Record the transition for the incident review.
        let mut history = Self::get_phase_history(env);
        if history.len() >= MAX_PHASE_HISTORY_LEN {
            history.remove(0);
        }
        history.push_back(phase);
        env.storage()
            .persistent()
            .set(&PauseKey::PhaseHistory, &history);

        publish_pause_phase_changed_event(env, phase, previous_phase, admin.clone());

        if fully_resumed {
            publish_contract_unpaused_event(env, admin.clone());
        }

        Ok(())
    }

    /// Roll the staged recovery back by one phase (issue #904).
    ///
    /// Returns `ContractError::InvalidPhaseRollback` when no phase has been
    /// resumed yet.
    pub fn rollback_operations_phase(env: &Env, admin: &Address) -> Result<(), ContractError> {
        admin.require_auth();
        AdminManager::require_admin(env, admin)?;

        let current = Self::get_resume_phase(env);
        if current == 0 {
            return Err(ContractError::InvalidPhaseRollback);
        }

        Self::resume_operations_phase(env, admin, current - 1)
    }

    // ── Internal helpers ───────────────────────────────────────────────────

    fn validate_reason(reason: &String) -> Result<(), ContractError> {
        let len = reason.len();
        if len == 0 || len as usize > MAX_PAUSE_REASON_LEN {
            return Err(ContractError::InvalidPauseReason);
        }
        Ok(())
    }

    /// Shared implementation of every "pause the contract" transition.
    fn apply_state(
        env: &Env,
        state: PauseState,
        reason: Option<String>,
        estimated_resume: Option<u64>,
        resume_phase: u32,
        admin: &Address,
    ) -> Result<(), ContractError> {
        let record = EmergencyPauseState {
            state,
            pause_reason: reason.clone(),
            estimated_resume,
            paused_at: env.ledger().timestamp(),
            resume_phase,
        };
        Self::save_state(env, &record);

        publish_contract_paused_event(env, admin.clone(), reason.clone(), estimated_resume);

        // Push the pause (with its reason) to every subscriber (issue #901).
        NotificationRegistry::emit_pause_notification(env, reason, estimated_resume);

        Ok(())
    }
}
