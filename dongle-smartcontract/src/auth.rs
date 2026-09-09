//! Shared authorization helpers used across all mutating contract endpoints.
//!
//! Centralizes auth patterns so every module enforces them consistently.
//! All helpers return `ContractError::Unauthorized` or `ContractError::AdminOnly`
//! on failure, never silently succeed or return `None`.
//!
//! # Reentrancy Analysis (#621)
//!
//! ## Soroban's Re-entrancy Model
//!
//! Soroban's WASM contract execution model is fundamentally different from the
//! EVM. In the current Stellar protocol:
//!
//! - **Each contract invocation is atomic and isolated.** The runtime does not
//!   support callbacks or mid-execution host-function calls that re-enter the
//!   same contract instance.
//! - **Cross-contract calls are synchronous sub-invocations** dispatched
//!   through the host environment. The calling contract is suspended while the
//!   callee executes; it cannot be re-entered during that suspension because
//!   there is no call-stack mechanism that permits the callee to call back into
//!   the caller before the original frame returns.
//! - The Soroban host explicitly rejects self-re-entrancy in its call-stack
//!   management; any attempt to invoke a contract that is already on the call
//!   stack results in a host-level error, not a second entry.
//!
//! ## Cross-Contract Call Sites in This Contract
//!
//! The only cross-contract calls in this contract are token transfers:
//!
//! | Function | Call | State written before call? |
//! |---|---|---|
//! | `execute_fee_payment` | `token::transfer(payer → treasury)` | No (flag written after) |
//! | `claim_fee_refund` | `token::transfer(treasury → payer)` | Yes ✅ (claimed_at set first) |
//! | `cancel_fee_payment` | `token::transfer(treasury → payer)` | Yes ✅ (flags removed first) |
//!
//! `execute_fee_payment` writes the paid flag *after* the transfer, but since
//! Soroban prevents re-entrancy this ordering is safe today. For defence-in-
//! depth the refund/cancel paths follow checks-effects-interactions explicitly.
//!
//! ## Conclusion
//!
//! Traditional EVM-style reentrancy attacks are **not possible** in this
//! contract under the current Soroban host. The platform-level isolation
//! eliminates the attack surface. No additional reentrancy guard (e.g. a
//! mutex flag) is needed. This analysis should be revisited if Soroban ever
//! introduces asynchronous cross-contract messaging or callback patterns.

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
