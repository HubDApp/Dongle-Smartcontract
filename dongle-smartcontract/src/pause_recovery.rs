//! Staged resumption of operations after an emergency pause — issue #903.
//!
//! A global pause is all-or-nothing: while the contract is paused every
//! mutating entry point is rejected. During an incident recovery that is
//! usually too coarse — the operator needs to bring the system back **one
//! area at a time** and be able to fall back to the previous area when
//! something still misbehaves.
//!
//! # The recovery ladder
//!
//! Recovery is expressed as an ordered *plan* of phases. The default plan is
//!
//! | Phase | Name           | What it re-enables                       |
//! |-------|----------------|------------------------------------------|
//! | 1     | `READ_OPS`     | read-only traffic (reads are never gated) |
//! | 2     | `CORE_WRITES`  | project register / update / archive       |
//! | 3     | `REVIEWS`      | review create / update / delete           |
//! | 4     | `VERIFICATION` | verification request / approve / reject   |
//! | 5     | `FEES`         | fee payment / refund / cancellation        |
//! | 6     | `ALL`          | everything (equivalent to a full resume)  |
//!
//! The plan is *not* cumulative: resuming the `REVIEWS` phase only re-enables
//! review mutations, the other areas stay halted. This is what "recover one
//! area at a time" means. Operators that want a different ladder can install
//! their own with [`PauseRecovery::set_pause_recovery_plan`].
//!
//! # Rollback
//!
//! Every phase change is pushed onto a stack. [`PauseRecovery::rollback_recovery_phase`]
//! pops the most recent phase and halts that area again, which lets an
//! operator back out of a phase that turned out to be unsafe.
//!
//! # Observability
//!
//! Every phase change emits a `PausePhaseChanged` event carrying the current
//! phase, the previously active phase, the enabled phases and the completed
//! phases.
//!
//! The operator guide lives in `docs/PAUSE_RECOVERY_PHASES.md`.

use crate::auth::require_admin_auth;
use crate::constants::{
    PAUSE_PHASE_ALL, PAUSE_PHASE_CORE_WRITES, PAUSE_PHASE_FEES, PAUSE_PHASE_FULL_PAUSE,
    PAUSE_PHASE_READ_OPS, PAUSE_PHASE_REVIEWS, PAUSE_PHASE_VERIFICATION,
};
use crate::emergency_pause::EmergencyPause;
use crate::errors::ContractError;
use crate::events::publish_pause_phase_changed_event;
use crate::storage_keys::PauseRecoveryKey;
use crate::types::PauseRecoveryState;
use soroban_sdk::{Address, Env, Vec};

pub struct PauseRecovery;

impl PauseRecovery {
    // ── Plan ─────────────────────────────────────────────────────────────

    /// The default recovery ladder.
    pub fn default_plan(env: &Env) -> Vec<u32> {
        let mut plan: Vec<u32> = Vec::new(env);
        plan.push_back(PAUSE_PHASE_READ_OPS);
        plan.push_back(PAUSE_PHASE_CORE_WRITES);
        plan.push_back(PAUSE_PHASE_REVIEWS);
        plan.push_back(PAUSE_PHASE_VERIFICATION);
        plan.push_back(PAUSE_PHASE_FEES);
        plan.push_back(PAUSE_PHASE_ALL);
        plan
    }

    /// The active recovery plan (defaults to [`PauseRecovery::default_plan`]).
    pub fn get_plan(env: &Env) -> Vec<u32> {
        env.storage()
            .persistent()
            .get(&PauseRecoveryKey::Plan)
            .unwrap_or_else(|| Self::default_plan(env))
    }

    /// Install a custom recovery plan (admin-only).
    ///
    /// Every phase code must be one of the known phase codes and the plan may
    /// not contain duplicates. The plan is only meaningful while the contract
    /// is paused.
    pub fn set_pause_recovery_plan(
        env: &Env,
        admin: Address,
        phases: Vec<u32>,
    ) -> Result<PauseRecoveryState, ContractError> {
        require_admin_auth(env, &admin)?;

        if phases.len() == 0 {
            return Err(ContractError::InvalidPausePhase);
        }
        for phase in phases.iter() {
            if !Self::is_known_phase(phase) {
                return Err(ContractError::InvalidPausePhase);
            }
        }
        // Reject duplicate phases so the plan stays a strict ladder.
        let mut seen: Vec<u32> = Vec::new(env);
        for phase in phases.iter() {
            if seen.iter().any(|p| p == phase) {
                return Err(ContractError::InvalidPausePhase);
            }
            seen.push_back(phase);
        }

        env.storage()
            .persistent()
            .set(&PauseRecoveryKey::Plan, &phases);

        Self::record_change(env, &admin, PAUSE_PHASE_FULL_PAUSE, false);
        Ok(Self::get_state(env))
    }

    // ── Resumption ───────────────────────────────────────────────────────

    /// Resume a single recovery phase (admin-only).
    ///
    /// The phase must be part of the active plan. Resuming a phase that is
    /// already live is a no-op. Every other area stays halted, so the operator
    /// can validate one area before moving on.
    pub fn resume_operations_phase(
        env: &Env,
        admin: Address,
        phase: u32,
    ) -> Result<PauseRecoveryState, ContractError> {
        require_admin_auth(env, &admin)?;

        if !EmergencyPause::is_paused(env) {
            return Err(ContractError::ContractNotPaused);
        }
        if !Self::is_known_phase(phase) {
            return Err(ContractError::InvalidPausePhase);
        }

        let plan = Self::get_plan(env);
        if !plan.iter().any(|p| p == phase) {
            return Err(ContractError::InvalidPausePhase);
        }

        let mut enabled = Self::get_enabled_phases(env);
        let previous_phase = Self::current_phase(&enabled);

        if !enabled.iter().any(|p| p == phase) {
            enabled.push_back(phase);
            env.storage()
                .persistent()
                .set(&PauseRecoveryKey::EnabledPhases, &enabled);
        }

        Self::record_change(env, &admin, previous_phase, false);
        Ok(Self::get_state(env))
    }

    /// Roll back the most recently resumed phase (admin-only).
    ///
    /// The area is halted again and the operator can resume a different
    /// phase. Used when a phase turns out to be unsafe to keep live.
    pub fn rollback_recovery_phase(
        env: &Env,
        admin: Address,
    ) -> Result<PauseRecoveryState, ContractError> {
        require_admin_auth(env, &admin)?;

        let mut enabled = Self::get_enabled_phases(env);
        if enabled.len() == 0 {
            return Err(ContractError::NoPhaseToRollBack);
        }

        let previous_phase = Self::current_phase(&enabled);
        let _rolled_back = enabled.pop_back().unwrap_or(PAUSE_PHASE_FULL_PAUSE);
        env.storage()
            .persistent()
            .set(&PauseRecoveryKey::EnabledPhases, &enabled);

        Self::record_change(env, &admin, previous_phase, true);
        Ok(Self::get_state(env))
    }

    // ── Gating ───────────────────────────────────────────────────────────

    /// Whether a phase is currently live.
    ///
    /// The `ALL` phase is a wildcard: once it is resumed every phase is live.
    pub fn is_phase_enabled(env: &Env, phase: u32) -> bool {
        let enabled = Self::get_enabled_phases(env);
        if enabled.iter().any(|p| p == PAUSE_PHASE_ALL) {
            return true;
        }
        enabled.iter().any(|p| p == phase)
    }

    /// Guard for operations that belong to `phase`.
    ///
    /// * Not paused → the operation is allowed.
    /// * Paused and the phase is live → the operation is allowed.
    /// * Paused and the phase is halted → `ContractError::ContractPaused`.
    pub fn require_phase(env: &Env, phase: u32) -> Result<(), ContractError> {
        if !EmergencyPause::is_paused(env) {
            return Ok(());
        }
        if Self::is_phase_enabled(env, phase) {
            Ok(())
        } else {
            Err(ContractError::ContractPaused)
        }
    }

    // ── State ────────────────────────────────────────────────────────────

    /// The current recovery state.
    pub fn get_state(env: &Env) -> PauseRecoveryState {
        let enabled = Self::get_enabled_phases(env);
        let plan = Self::get_plan(env);
        let frontier = Self::current_phase(&enabled);

        // Phases of the plan that the operator already moved past are
        // reported as completed — this is the "phase progress" tracking.
        let mut completed: Vec<u32> = Vec::new(env);
        for phase in plan.iter() {
            if phase < frontier && !enabled.iter().any(|p| p == phase) {
                completed.push_back(phase);
            }
        }

        PauseRecoveryState {
            is_paused: EmergencyPause::is_paused(env),
            enabled_phases: enabled,
            completed_phases: completed,
            plan,
            last_changed_at: env
                .storage()
                .persistent()
                .get(&PauseRecoveryKey::LastChangedAt)
                .unwrap_or(0),
            last_changed_by: env
                .storage()
                .persistent()
                .get(&PauseRecoveryKey::LastChangedBy),
        }
    }

    /// Reset the recovery state (called when the contract is paused or fully
    /// resumed). The plan itself is preserved so operators keep their ladder.
    pub fn reset(env: &Env) {
        env.storage()
            .persistent()
            .set(&PauseRecoveryKey::EnabledPhases, &Vec::<u32>::new(env));
        env.storage()
            .persistent()
            .set(&PauseRecoveryKey::CompletedPhases, &Vec::<u32>::new(env));
    }

    // ── Internal helpers ─────────────────────────────────────────────────

    fn get_enabled_phases(env: &Env) -> Vec<u32> {
        env.storage()
            .persistent()
            .get(&PauseRecoveryKey::EnabledPhases)
            .unwrap_or_else(|| Vec::new(env))
    }

    /// The most recently resumed phase (`0` = nothing resumed yet).
    fn current_phase(enabled: &Vec<u32>) -> u32 {
        enabled.last().unwrap_or(PAUSE_PHASE_FULL_PAUSE)
    }

    /// Whether the phase code is one of the known recovery phases.
    pub fn is_known_phase(phase: u32) -> bool {
        phase == PAUSE_PHASE_READ_OPS
            || phase == PAUSE_PHASE_CORE_WRITES
            || phase == PAUSE_PHASE_REVIEWS
            || phase == PAUSE_PHASE_VERIFICATION
            || phase == PAUSE_PHASE_FEES
            || phase == PAUSE_PHASE_ALL
    }

    /// Record a phase change and emit the `PausePhaseChanged` event.
    fn record_change(env: &Env, admin: &Address, previous_phase: u32, rolled_back: bool) {
        let enabled = Self::get_enabled_phases(env);
        let plan = Self::get_plan(env);
        let phase = Self::current_phase(&enabled);
        let frontier = phase;

        let mut completed: Vec<u32> = Vec::new(env);
        for p in plan.iter() {
            if p < frontier && !enabled.iter().any(|e| e == p) {
                completed.push_back(p);
            }
        }

        let now = env.ledger().timestamp();
        env.storage()
            .persistent()
            .set(&PauseRecoveryKey::LastChangedAt, &now);
        env.storage()
            .persistent()
            .set(&PauseRecoveryKey::LastChangedBy, &admin.clone());
        env.storage()
            .persistent()
            .set(&PauseRecoveryKey::CompletedPhases, &completed);

        let count: u32 = env
            .storage()
            .persistent()
            .get(&PauseRecoveryKey::PhaseHistoryCount)
            .unwrap_or(0);
        env.storage()
            .persistent()
            .set(&PauseRecoveryKey::PhaseHistory(count), &phase);
        env.storage()
            .persistent()
            .set(&PauseRecoveryKey::PhaseHistoryCount, &(count + 1));

        publish_pause_phase_changed_event(
            env,
            phase,
            previous_phase,
            enabled,
            completed,
            admin.clone(),
            rolled_back,
        );
    }
}
