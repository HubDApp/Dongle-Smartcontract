//! Shared authorization helpers used across all mutating contract endpoints.
//!
//! Centralizes auth patterns so every module enforces them consistently.
//! All helpers return `ContractError::Unauthorized` or `ContractError::AdminOnly`
//! on failure, never silently succeed or return `None`.

use crate::admin_manager::AdminManager;
use crate::errors::ContractError;
use soroban_sdk::{Address, Env};

/// Require that `caller` has signed this invocation AND is a registered admin.
///
/// Used by: `set_fee`, `approve_verification`, `reject_verification`,
///          `add_admin`, `remove_admin`.
pub fn require_admin_auth(env: &Env, caller: &Address) -> Result<(), ContractError> {
    caller.require_auth();
    AdminManager::require_admin(env, caller)
}

/// Require that `caller` has signed this invocation AND matches `expected_owner`.
///
/// Used by: `update_project`, `request_verification`.
pub fn require_owner_auth(caller: &Address, expected_owner: &Address) -> Result<(), ContractError> {
    caller.require_auth();
    if caller != expected_owner {
        return Err(ContractError::Unauthorized);
    }
    Ok(())
}

/// Require that `caller` has signed this invocation.
///
/// Used by: `register_project`, `add_review`, `update_review`,
///          `delete_review`, `pay_fee`.
pub fn require_self_auth(caller: &Address) {
    caller.require_auth();
}
