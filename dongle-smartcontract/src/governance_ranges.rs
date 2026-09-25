//! Configurable and enforced ranges for governance parameters (issue #740).
//!
//! Governance parameters (approval threshold, fees, durations, index limits)
//! are the knobs an admin can turn. Before #740 nothing bounded them beyond
//! a few per-callsite checks, so an admin could set a threshold above the
//! admin count, a fee large enough to lock every user out, or a verification
//! duration of zero. This module gives each parameter a stored
//! `[min, max]` range, validates every write against it, and requires
//! multi-signature approval before a range itself can be changed.
//!
//! ## Layering
//!
//! ```text
//! set_param_range (single-sig)      →  ProposalPayload::SetParamRange
//!         │                                        │
//!         └──────────────► apply_range ◄────────────┘
//!                              │
//!                              ▼
//!                     validate(param, value)
//! ```
//!
//! Ranges are read with a compile-time default fallback, so a deployment
//! that predates this module keeps working with the documented defaults and
//! existing parameter values stay valid.
//!
//! See `docs/GOVERNANCE_RANGES.md`.

use crate::admin_action_log::AdminActionLog;
use crate::admin_manager::AdminManager;
use crate::auth::require_admin_auth;
use crate::constants::{
    MAX_APPROVAL_THRESHOLD, MAX_MAX_REVIEWS_PER_PROJECT, MAX_REGISTRATION_FEE,
    MAX_REVIEW_FEE, MAX_VERIFICATION_DURATION_SECS, MAX_VERIFICATION_FEE, MIN_APPROVAL_THRESHOLD,
    MIN_MAX_REVIEWS_PER_PROJECT, MIN_VERIFICATION_DURATION_SECS,
};
use crate::errors::ContractError;
use crate::events::publish_param_range_changed_event;
use crate::storage_keys::GovernanceKey;
use crate::types::{AdminActionType, GovernanceParam, ParamRange};
use soroban_sdk::{Address, Env, Map};

/// Read/write helper for the `[min, max]` range of every [`GovernanceParam`].
pub struct GovernanceRanges;

impl GovernanceRanges {
    /// Compile-time default range for `param`.
    ///
    /// These defaults are what a deployment that has never called
    /// `set_param_range` uses, and every default comfortably contains the
    /// contract's own default parameter values.
    pub fn default_range(param: GovernanceParam) -> ParamRange {
        match param {
            GovernanceParam::ApprovalThreshold => ParamRange {
                min: MIN_APPROVAL_THRESHOLD,
                max: MAX_APPROVAL_THRESHOLD,
            },
            GovernanceParam::VerificationFee => ParamRange {
                min: 0,
                max: MAX_VERIFICATION_FEE,
            },
            GovernanceParam::RegistrationFee => ParamRange {
                min: 0,
                max: MAX_REGISTRATION_FEE,
            },
            GovernanceParam::ReviewFee => ParamRange {
                min: 0,
                max: MAX_REVIEW_FEE,
            },
            GovernanceParam::VerificationDuration => ParamRange {
                min: MIN_VERIFICATION_DURATION_SECS,
                max: MAX_VERIFICATION_DURATION_SECS,
            },
            GovernanceParam::MaxReviewsPerProject => ParamRange {
                min: MIN_MAX_REVIEWS_PER_PROJECT,
                max: MAX_MAX_REVIEWS_PER_PROJECT,
            },
        }
    }

    /// The range currently in force for `param`.
    ///
    /// Falls back to [`GovernanceRanges::default_range`] when no range has
    /// been stored, so a read is always defined.
    pub fn get_range(env: &Env, param: GovernanceParam) -> ParamRange {
        let key = GovernanceKey::ParamRange(param);
        let stored: Option<ParamRange> = env.storage().persistent().get(&key);
        match stored {
            Some(range) => {
                crate::storage_manager::StorageManager::extend_critical_config_ttl(env);
                range
            }
            None => Self::default_range(param),
        }
    }

    /// Every parameter with its current range, in enum order.
    pub fn get_all_ranges(env: &Env) -> Map<GovernanceParam, ParamRange> {
        let mut map = Map::new(env);
        for param in Self::all_params() {
            map.set(param, Self::get_range(env, param));
        }
        map
    }

    /// All governed parameters, in declaration order.
    pub fn all_params() -> Vec<GovernanceParam> {
        alloc::vec![
            GovernanceParam::ApprovalThreshold,
            GovernanceParam::VerificationFee,
            GovernanceParam::RegistrationFee,
            GovernanceParam::ReviewFee,
            GovernanceParam::VerificationDuration,
            GovernanceParam::MaxReviewsPerProject,
        ]
    }

    /// Current on-chain value of `param`, in the parameter's natural unit.
    ///
    /// Used by [`GovernanceRanges::apply_range`] to refuse a range that
    /// would strand the contract with an out-of-range value, and exposed to
    /// frontends so a settings UI can show value + bounds together.
    pub fn current_value(env: &Env, param: GovernanceParam) -> u128 {
        match param {
            GovernanceParam::ApprovalThreshold => {
                AdminManager::get_admin_approval_threshold(env) as u128
            }
            GovernanceParam::VerificationFee => crate::fee_manager::FeeManager::get_fee_config(env)
                .map(|c| c.verification_fee)
                .unwrap_or(0),
            GovernanceParam::RegistrationFee => crate::fee_manager::FeeManager::get_fee_config(env)
                .map(|c| c.registration_fee)
                .unwrap_or(0),
            GovernanceParam::ReviewFee => {
                crate::review_registry::ReviewRegistry::get_review_eligibility_config(env).review_fee
            }
            GovernanceParam::VerificationDuration => {
                AdminManager::get_verification_duration(env) as u128
            }
            GovernanceParam::MaxReviewsPerProject => {
                crate::config_registry::ConfigRegistry::get_max_reviews_per_project(env) as u128
            }
        }
    }

    /// Assert that `value` is inside the range currently in force.
    ///
    /// Every governance setter calls this **before** it writes, so an
    /// out-of-range value never reaches storage. Callers get
    /// [`ContractError::ParameterOutOfRange`] and can read the effective
    /// bounds via [`GovernanceRanges::get_range`] to build a useful message.
    pub fn validate(env: &Env, param: GovernanceParam, value: u128) -> Result<(), ContractError> {
        let range = Self::get_range(env, param);
        if range.contains(value) {
            Ok(())
        } else {
            Err(ContractError::ParameterOutOfRange)
        }
    }

    /// Admin-only: set the range for one parameter.
    ///
    /// # Multi-sig requirement (#740)
    ///
    /// Returns `MultiSigRequired` when the admin approval threshold is above
    /// 1. Widening or narrowing a range is itself a governance decision, so
    /// in a multi-sig deployment it must be submitted as a
    /// `ProposalPayload::SetParamRange` proposal and reach the quorum through
    /// `execute_proposal`. This mirrors how `add_admin` / `set_fee` behave.
    ///
    /// # Rejections
    /// - `InvalidParamRange` — `min > max`, or the range excludes the
    ///   parameter's current value (which would leave the contract with a
    ///   value that the contract's own validation rejects).
    pub fn set_range(
        env: &Env,
        admin: Address,
        param: GovernanceParam,
        range: ParamRange,
    ) -> Result<(), ContractError> {
        require_admin_auth(env, &admin)?;

        if AdminManager::get_admin_approval_threshold(env) > 1 {
            return Err(ContractError::MultiSigRequired);
        }

        Self::apply_range(env, admin, param, range)
    }

    /// Write a range, recording the admin action and emitting an event.
    ///
    /// Called by [`GovernanceRanges::set_range`] (single-sig path) and by
    /// `AdminManager::execute_proposal` for `ProposalPayload::SetParamRange`
    /// (multi-sig path). The multi-sig gate therefore lives in the caller,
    /// not here.
    pub fn apply_range(
        env: &Env,
        admin: Address,
        param: GovernanceParam,
        range: ParamRange,
    ) -> Result<(), ContractError> {
        if range.min > range.max {
            return Err(ContractError::InvalidParamRange);
        }

        // A range that excludes the live value would make the current
        // configuration invalid on its own terms (e.g. shrinking the fee
        // range below the fee already collected). Reject instead of
        // stranding the contract in an unfixable state.
        let current = Self::current_value(env, param);
        if !range.contains(current) {
            return Err(ContractError::InvalidParamRange);
        }

        let key = GovernanceKey::ParamRange(param);
        env.storage().persistent().set(&key, &range);
        crate::storage_manager::StorageManager::extend_critical_config_ttl(env);

        publish_param_range_changed_event(env, admin.clone(), param, range.min, range.max);

        AdminActionLog::record_action(
            env,
            admin,
            AdminActionType::ParamRangeChanged,
            None,
            None,
            None,
        );

        Ok(())
    }
}
