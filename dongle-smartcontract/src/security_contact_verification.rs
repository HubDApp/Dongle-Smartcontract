//! Security contact email/URL verification via challenge-response (#757).
//!
//! ## Workflow
//!
//! 1. Owner calls `initiate_security_contact_verification` with their security
//!    contact string. A unique token is generated and stored with a 24-hour
//!    expiry. The token is emitted as an on-chain event so the owner (or an
//!    automated relay) can deliver it out-of-band to the contact address.
//! 2. Owner calls `confirm_security_contact_verification` with the same token.
//!    The contract validates the token and marks the contact as verified,
//!    recording the verification timestamp.
//! 3. Verification expires after `SECURITY_CONTACT_VERIFICATION_VALID_SECS`
//!    (one year). `check_security_contact_verification_expiry` returns whether
//!    re-verification is needed. Admins may also call
//!    `admin_revoke_security_contact_verification` to force re-verification.

use crate::errors::ContractError;
use crate::project_registry::ProjectRegistry;
use crate::storage_keys::SecurityContactVerifKey as SCVK;
use crate::storage_manager::StorageManager;
use crate::types::{
    SecurityContactVerificationRecord, SecurityContactVerificationStatus,
};
use soroban_sdk::{symbol_short, Address, Env, String};

/// How long a pending challenge token remains valid (24 hours in seconds).
pub const CHALLENGE_TOKEN_EXPIRY_SECS: u64 = 86_400;

/// How long a completed verification remains valid before annual re-verification
/// is required (365 days in seconds).
pub const SECURITY_CONTACT_VERIFICATION_VALID_SECS: u64 = 365 * 24 * 3600;

pub struct SecurityContactVerificationRegistry;

impl SecurityContactVerificationRegistry {
    // ── Helpers ───────────────────────────────────────────────────────────

    /// Derive a deterministic challenge token from ledger timestamp + project_id.
    /// In production environments the token should be treated as a nonce —
    /// it is emitted in an on-chain event and confirmed by the contact owner.
    fn derive_token(env: &Env, project_id: u64) -> String {
        // Combine timestamp and project_id to produce a simple hex-like token
        // stored as a Soroban String.
        let ts = env.ledger().timestamp();
        let raw: u64 = ts.wrapping_mul(1_000_003).wrapping_add(project_id);
        // Build a 16-char lowercase hex string without std::fmt.
        const HEX: &[u8] = b"0123456789abcdef";
        let mut buf = [0u8; 16];
        let mut n = raw;
        for i in (0..16).rev() {
            buf[i] = HEX[(n & 0xF) as usize];
            n >>= 4;
        }
        // SAFETY: buf is always valid ASCII.
        let s = core::str::from_utf8(&buf).unwrap_or("0000000000000000");
        String::from_str(env, s)
    }

    fn load_record(env: &Env, project_id: u64) -> Option<SecurityContactVerificationRecord> {
        env.storage()
            .persistent()
            .get(&SCVK::SecurityContactVerifRecord(project_id))
    }

    fn save_record(env: &Env, project_id: u64, record: &SecurityContactVerificationRecord) {
        env.storage()
            .persistent()
            .set(&SCVK::SecurityContactVerifRecord(project_id), record);
        StorageManager::extend_if_exists_scvk(env, &SCVK::SecurityContactVerifRecord(project_id));
    }

    // ── Public API ────────────────────────────────────────────────────────

    /// Initiate a challenge-response verification for the project's security contact.
    ///
    /// Generates a unique token, stores it with a 24-hour expiry, and emits a
    /// `SecurityContactChallengeIssued` event. The caller must be the project owner.
    ///
    /// If the contact is currently verified, calling this resets the verification
    /// so the owner can refresh it.
    pub fn initiate_verification(
        env: &Env,
        project_id: u64,
        caller: &Address,
    ) -> Result<SecurityContactVerificationRecord, ContractError> {
        caller.require_auth();

        let project = ProjectRegistry::get_project(env, project_id)
            .ok_or(ContractError::ProjectNotFound)?;
        if project.owner != *caller {
            return Err(ContractError::Unauthorized);
        }

        let contact = project
            .security_contact
            .ok_or(ContractError::InvalidProjectData)?;

        let token = Self::derive_token(env, project_id);
        let now = env.ledger().timestamp();
        let expires_at = now.saturating_add(CHALLENGE_TOKEN_EXPIRY_SECS);

        let record = SecurityContactVerificationRecord {
            project_id,
            contact: contact.clone(),
            token: token.clone(),
            token_issued_at: now,
            token_expires_at: expires_at,
            verified: false,
            verified_at: 0,
            verification_expires_at: 0,
        };
        Self::save_record(env, project_id, &record);

        env.events().publish(
            (symbol_short!("SC_CHALL"), project_id),
            (caller.clone(), contact, token, expires_at),
        );

        Ok(record)
    }

    /// Confirm ownership of the security contact by providing the correct token.
    ///
    /// Marks the contact as verified and sets the verification expiry to one year
    /// from now. The caller must be the project owner.
    pub fn confirm_verification(
        env: &Env,
        project_id: u64,
        caller: &Address,
        token: String,
    ) -> Result<SecurityContactVerificationRecord, ContractError> {
        caller.require_auth();

        let project = ProjectRegistry::get_project(env, project_id)
            .ok_or(ContractError::ProjectNotFound)?;
        if project.owner != *caller {
            return Err(ContractError::Unauthorized);
        }

        let mut record = Self::load_record(env, project_id)
            .ok_or(ContractError::InvalidProjectData)?;

        // Token must not have expired.
        let now = env.ledger().timestamp();
        if now > record.token_expires_at {
            return Err(ContractError::VerificationExpired);
        }

        // Token must match.
        if record.token != token {
            return Err(ContractError::Unauthorized);
        }

        record.verified = true;
        record.verified_at = now;
        record.verification_expires_at = now.saturating_add(SECURITY_CONTACT_VERIFICATION_VALID_SECS);

        Self::save_record(env, project_id, &record);

        env.events().publish(
            (symbol_short!("SC_VERIF"), project_id),
            (caller.clone(), record.contact.clone(), record.verification_expires_at),
        );

        Ok(record)
    }

    /// Return the current verification status for a project's security contact.
    pub fn get_status(env: &Env, project_id: u64) -> SecurityContactVerificationStatus {
        let now = env.ledger().timestamp();
        match Self::load_record(env, project_id) {
            None => SecurityContactVerificationStatus {
                project_id,
                contact: None,
                verified: false,
                verified_at: 0,
                verification_expires_at: 0,
                re_verification_required: false,
                challenge_pending: false,
            },
            Some(r) => {
                let re_verification_required =
                    r.verified && now > r.verification_expires_at;
                let challenge_pending = !r.verified && r.token_expires_at > now;
                SecurityContactVerificationStatus {
                    project_id,
                    contact: Some(r.contact),
                    verified: r.verified && !re_verification_required,
                    verified_at: r.verified_at,
                    verification_expires_at: r.verification_expires_at,
                    re_verification_required,
                    challenge_pending,
                }
            }
        }
    }

    /// Admin-only: revoke a security contact verification and require re-verification.
    pub fn admin_revoke(
        env: &Env,
        project_id: u64,
        admin: &Address,
    ) -> Result<(), ContractError> {
        admin.require_auth();
        crate::admin_manager::AdminManager::require_admin(env, admin)?;

        let mut record = Self::load_record(env, project_id)
            .ok_or(ContractError::InvalidProjectData)?;

        record.verified = false;
        record.verified_at = 0;
        record.verification_expires_at = 0;

        Self::save_record(env, project_id, &record);

        env.events().publish(
            (symbol_short!("SC_REVOK"), project_id),
            (admin.clone(),),
        );

        Ok(())
    }

    /// Check whether re-verification is required (annual expiry check).
    pub fn requires_reverification(env: &Env, project_id: u64) -> bool {
        let now = env.ledger().timestamp();
        match Self::load_record(env, project_id) {
            None => false,
            Some(r) => r.verified && now > r.verification_expires_at,
        }
    }
}
