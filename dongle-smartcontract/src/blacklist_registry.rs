//! Address blacklisting registry — Issue #987
//!
//! Allows admins to blacklist Stellar addresses that violate terms of service.
//! Blacklisted addresses are blocked from all mutating contract operations until
//! the entry expires or an admin removes them.
//!
//! # Features
//!
//! - **Temporary blacklisting**: entry expires at a caller-supplied Unix timestamp.
//! - **Permanent blacklisting**: entry never expires.
//! - **Blacklist check**: O(1) lookup used as a guard in every mutating operation.
//! - **Appeal process**: blacklisted address submits `request_blacklist_removal`
//!   with an IPFS appeal CID; admin reviews with `review_blacklist_appeal`.
//! - **Statistics**: aggregate counters (total, active, appeals, grants, denials).
//! - **Events**: `AddressBlacklisted`, `AddressWhitelisted`,
//!   `BlacklistAppealSubmitted`, `BlacklistAppealReviewed`.
//!
//! # Storage layout
//!
//! All keys live in [`BlacklistKey`] to avoid hitting the 50-variant
//! Soroban `#[contracttype]` cap on the main enums.

use soroban_sdk::{Address, Env, String, Vec};

use crate::admin_manager::AdminManager;
use crate::errors::ContractError;
use crate::events::{
    publish_address_blacklisted_event, publish_address_whitelisted_event,
    publish_blacklist_appeal_reviewed_event, publish_blacklist_appeal_submitted_event,
};
use crate::storage_keys::BlacklistKey;
use crate::types::{
    AppealStatus, BlacklistAppeal, BlacklistEntry, BlacklistKind, BlacklistStats, BlacklistStatus,
};

/// Maximum byte length for the blacklist reason string.
const MAX_REASON_LEN: u32 = 256;

pub struct BlacklistRegistry;

impl BlacklistRegistry {
    // ── Internal helpers ──────────────────────────────────────────────────────

    fn get_entry(env: &Env, address: &Address) -> Option<BlacklistEntry> {
        env.storage()
            .persistent()
            .get(&BlacklistKey::BlacklistEntry(address.clone()))
    }

    fn save_entry(env: &Env, address: &Address, entry: &BlacklistEntry) {
        env.storage()
            .persistent()
            .set(&BlacklistKey::BlacklistEntry(address.clone()), entry);
    }

    fn get_stats(env: &Env) -> BlacklistStats {
        env.storage()
            .persistent()
            .get(&BlacklistKey::BlacklistStats)
            .unwrap_or(BlacklistStats {
                total_blacklisted: 0,
                currently_active: 0,
                total_appeals: 0,
                appeals_granted: 0,
                appeals_denied: 0,
            })
    }

    fn save_stats(env: &Env, stats: &BlacklistStats) {
        env.storage()
            .persistent()
            .set(&BlacklistKey::BlacklistStats, stats);
    }

    fn get_addresses(env: &Env) -> Vec<Address> {
        env.storage()
            .persistent()
            .get(&BlacklistKey::BlacklistedAddresses)
            .unwrap_or(Vec::new(env))
    }

    fn save_addresses(env: &Env, list: &Vec<Address>) {
        env.storage()
            .persistent()
            .set(&BlacklistKey::BlacklistedAddresses, list);
    }

    fn validate_reason(reason: &String) -> Result<(), ContractError> {
        let len = reason.len();
        if len == 0 || len > MAX_REASON_LEN {
            return Err(ContractError::InvalidBlacklistReason);
        }
        Ok(())
    }

    // ── Public API ────────────────────────────────────────────────────────────

    /// Blacklist an address (admin-only).
    ///
    /// # Parameters
    /// - `admin`      — caller; must be a registered admin.
    /// - `address`    — address to blacklist.
    /// - `reason`     — human-readable reason (1–256 bytes).
    /// - `expires_at` — Unix timestamp for expiry; pass `0` for a permanent ban.
    ///
    /// Calling this on an already-active entry **updates** the existing entry
    /// (extends or changes the expiry and reason) rather than returning an error,
    /// so admins can tighten or extend a ban without first removing it.
    pub fn blacklist_address(
        env: &Env,
        admin: &Address,
        address: &Address,
        reason: String,
        expires_at: u64,
    ) -> Result<(), ContractError> {
        admin.require_auth();

        if !AdminManager::is_admin(env, admin) {
            return Err(ContractError::AdminOnly);
        }

        Self::validate_reason(&reason)?;

        let now = env.ledger().timestamp();
        let kind = if expires_at == 0 {
            BlacklistKind::Permanent
        } else {
            BlacklistKind::Temporary
        };
        let permanent = kind == BlacklistKind::Permanent;

        // Determine whether this is a new entry or an update.
        let is_new = match Self::get_entry(env, address) {
            Some(e) => e.status != BlacklistStatus::Active,
            None => true,
        };

        let entry = BlacklistEntry {
            address: address.clone(),
            reason: reason.clone(),
            kind,
            expires_at,
            blacklisted_by: admin.clone(),
            blacklisted_at: now,
            status: BlacklistStatus::Active,
        };
        Self::save_entry(env, address, &entry);

        // Update the global address list (only add if new).
        if is_new {
            let mut list = Self::get_addresses(env);
            list.push_back(address.clone());
            Self::save_addresses(env, &list);

            let mut stats = Self::get_stats(env);
            stats.total_blacklisted = stats.total_blacklisted.saturating_add(1);
            stats.currently_active = stats.currently_active.saturating_add(1);
            Self::save_stats(env, &stats);
        }

        publish_address_blacklisted_event(
            env,
            address.clone(),
            reason,
            permanent,
            expires_at,
            admin.clone(),
        );

        Ok(())
    }

    /// Remove (whitelist) an address from the blacklist (admin-only).
    ///
    /// Returns [`ContractError::AddressNotBlacklisted`] if the address is not
    /// currently active.
    pub fn remove_blacklist(
        env: &Env,
        admin: &Address,
        address: &Address,
    ) -> Result<(), ContractError> {
        admin.require_auth();

        if !AdminManager::is_admin(env, admin) {
            return Err(ContractError::AdminOnly);
        }

        let mut entry = Self::get_entry(env, address)
            .ok_or(ContractError::AddressNotBlacklisted)?;

        if entry.status != BlacklistStatus::Active {
            return Err(ContractError::AddressNotBlacklisted);
        }

        entry.status = BlacklistStatus::Removed;
        Self::save_entry(env, address, &entry);

        let mut stats = Self::get_stats(env);
        stats.currently_active = stats.currently_active.saturating_sub(1);
        Self::save_stats(env, &stats);

        publish_address_whitelisted_event(env, address.clone(), admin.clone());

        Ok(())
    }

    /// Check whether an address is currently blacklisted and its ban has not expired.
    ///
    /// Returns `true` when the address should be blocked.
    pub fn is_blacklisted(env: &Env, address: &Address) -> bool {
        match Self::get_entry(env, address) {
            None => false,
            Some(entry) => {
                if entry.status != BlacklistStatus::Active {
                    return false;
                }
                // Temporary entries expire when the ledger timestamp passes expires_at.
                if entry.kind == BlacklistKind::Temporary {
                    let now = env.ledger().timestamp();
                    if now >= entry.expires_at {
                        return false; // Expired — treat as not blacklisted
                    }
                }
                true
            }
        }
    }

    /// Assert the caller is not blacklisted; return an error if they are.
    ///
    /// This is the guard used at the top of every mutating contract function.
    pub fn assert_not_blacklisted(env: &Env, address: &Address) -> Result<(), ContractError> {
        if Self::is_blacklisted(env, address) {
            Err(ContractError::AddressBlacklisted)
        } else {
            Ok(())
        }
    }

    /// Submit a blacklist removal appeal (called by the blacklisted address itself).
    ///
    /// # Parameters
    /// - `appellant`  — the blacklisted address filing the appeal; must authenticate.
    /// - `appeal_cid` — IPFS CID pointing to off-chain evidence or explanation.
    ///
    /// Returns [`ContractError::AddressNotBlacklisted`] if the caller is not
    /// currently blacklisted, and [`ContractError::AppealAlreadyExists`] if a
    /// pending appeal is already on record.
    pub fn request_blacklist_removal(
        env: &Env,
        appellant: &Address,
        appeal_cid: String,
    ) -> Result<(), ContractError> {
        appellant.require_auth();

        if !Self::is_blacklisted(env, appellant) {
            return Err(ContractError::AddressNotBlacklisted);
        }

        // Validate CID (must be non-empty)
        if appeal_cid.len() == 0 {
            return Err(ContractError::InvalidAppealCid);
        }

        // Reject duplicate pending appeals
        if let Some(existing) = Self::get_appeal(env, appellant) {
            if existing.status == AppealStatus::Pending {
                return Err(ContractError::AppealAlreadyExists);
            }
        }

        let appeal = BlacklistAppeal {
            appellant: appellant.clone(),
            appeal_cid: appeal_cid.clone(),
            submitted_at: env.ledger().timestamp(),
            status: AppealStatus::Pending,
            reviewed_by: None,
            reviewed_at: 0,
            review_note: None,
        };
        Self::save_appeal(env, appellant, &appeal);

        let mut stats = Self::get_stats(env);
        stats.total_appeals = stats.total_appeals.saturating_add(1);
        Self::save_stats(env, &stats);

        publish_blacklist_appeal_submitted_event(env, appellant.clone(), appeal_cid);

        Ok(())
    }

    /// Review (grant or deny) a blacklist appeal (admin-only).
    ///
    /// Granting an appeal removes the blacklist entry and decrements
    /// `currently_active`. Denying leaves it in place.
    pub fn review_blacklist_appeal(
        env: &Env,
        admin: &Address,
        appellant: &Address,
        grant: bool,
        note: Option<String>,
    ) -> Result<(), ContractError> {
        admin.require_auth();

        if !AdminManager::is_admin(env, admin) {
            return Err(ContractError::AdminOnly);
        }

        let mut appeal = Self::get_appeal(env, appellant)
            .ok_or(ContractError::AppealNotExist)?;

        if appeal.status != AppealStatus::Pending {
            return Err(ContractError::AppealAlreadyResolved);
        }

        let now = env.ledger().timestamp();
        let outcome = if grant {
            AppealStatus::Granted
        } else {
            AppealStatus::Denied
        };

        appeal.status = outcome;
        appeal.reviewed_by = Some(admin.clone());
        appeal.reviewed_at = now;
        appeal.review_note = note;
        Self::save_appeal(env, appellant, &appeal);

        let mut stats = Self::get_stats(env);
        if grant {
            // Lift the blacklist
            if let Some(mut entry) = Self::get_entry(env, appellant) {
                if entry.status == BlacklistStatus::Active {
                    entry.status = BlacklistStatus::AppealGranted;
                    Self::save_entry(env, appellant, &entry);
                    stats.currently_active = stats.currently_active.saturating_sub(1);
                }
            }
            stats.appeals_granted = stats.appeals_granted.saturating_add(1);
        } else {
            stats.appeals_denied = stats.appeals_denied.saturating_add(1);
        }
        Self::save_stats(env, &stats);

        publish_blacklist_appeal_reviewed_event(env, appellant.clone(), outcome, admin.clone());

        Ok(())
    }

    // ── Read-only helpers ─────────────────────────────────────────────────────

    /// Retrieve the blacklist entry for an address (if any).
    pub fn get_blacklist_entry(env: &Env, address: &Address) -> Option<BlacklistEntry> {
        Self::get_entry(env, address)
    }

    /// Retrieve the appeal record for an address (if any).
    pub fn get_appeal(env: &Env, address: &Address) -> Option<BlacklistAppeal> {
        env.storage()
            .persistent()
            .get(&BlacklistKey::BlacklistAppeal(address.clone()))
    }

    fn save_appeal(env: &Env, address: &Address, appeal: &BlacklistAppeal) {
        env.storage()
            .persistent()
            .set(&BlacklistKey::BlacklistAppeal(address.clone()), appeal);
    }

    /// Return the aggregate blacklist statistics.
    pub fn get_blacklist_stats(env: &Env) -> BlacklistStats {
        Self::get_stats(env)
    }

    /// Return the list of all blacklisted addresses (paginated).
    ///
    /// `offset` is zero-based; pass `limit = 0` to request the default page
    /// size of 50.
    pub fn list_blacklisted_addresses(
        env: &Env,
        offset: u32,
        limit: u32,
    ) -> Vec<Address> {
        let effective_limit = if limit == 0 || limit > 50 { 50u32 } else { limit };
        let list = Self::get_addresses(env);
        let total = list.len();
        let start = offset.min(total);
        let end = (start + effective_limit).min(total);
        let mut page = Vec::new(env);
        for i in start..end {
            page.push_back(list.get(i).unwrap());
        }
        page
    }
}
