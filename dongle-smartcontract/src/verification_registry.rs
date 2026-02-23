//! Verification requests with ownership and fee checks, and events.

use crate::constants::MAX_CID_LEN;
use crate::errors::Error;
use crate::events::VerificationApproved;
use crate::events::VerificationRejected;
use crate::events::VerificationRequested;
use crate::storage_keys::StorageKey;
use crate::types::{VerificationRecord, VerificationStatus};
use soroban_sdk::{Address, Env, String};

pub struct VerificationRegistry;

impl VerificationRegistry {
    pub fn request_verification(
        env: &Env,
        project_id: u64,
        requester: Address,
        evidence_cid: String,
    ) {
        // Validate project ownership
        // Require fee paid via FeeManager
        // Store VerificationRecord with Pending
    }

    pub fn approve_verification(env: &Env, project_id: u64, verifier: Address) -> Result<(), Error> {
        if !Self::is_admin(env, &verifier) {
            return Err(Error::UnauthorizedVerifier);
        }

        let key = StorageKey::Verification(project_id);
        let mut record: VerificationRecord = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(Error::VerificationNotFound)?;

        if record.status != VerificationStatus::Pending {
            return Err(Error::VerificationNotPending);
        }

        let ledger_timestamp = env.ledger().timestamp();
        record.status = VerificationStatus::Verified;
        record.decided_at = Some(ledger_timestamp);
        env.storage().persistent().set(&key, &record);

        VerificationApproved {
            project_id,
            verifier,
        }
        .publish(env);

        Ok(())
    }

    pub fn reject_verification(env: &Env, project_id: u64, verifier: Address) -> Result<(), Error> {
        if !Self::is_admin(env, &verifier) {
            return Err(Error::UnauthorizedVerifier);
        }

        let key = StorageKey::Verification(project_id);
        let mut record: VerificationRecord = env
            .storage()
            .persistent()
            .get(&key)
            .ok_or(Error::VerificationNotFound)?;

        if record.status != VerificationStatus::Pending {
            return Err(Error::VerificationNotPending);
        }

        let ledger_timestamp = env.ledger().timestamp();
        record.status = VerificationStatus::Rejected;
        record.decided_at = Some(ledger_timestamp);
        env.storage().persistent().set(&key, &record);

        VerificationRejected {
            project_id,
            verifier,
        }
        .publish(env);

        Ok(())
    }

    fn is_admin(env: &Env, addr: &Address) -> bool {
        let admin: Option<Address> = env.storage().persistent().get(&StorageKey::Admin);
        admin.as_ref().map_or(false, |a| a == addr)
    }

    pub fn get_verification(env: &Env, project_id: u64) -> Option<VerificationRecord> {
        env.storage()
            .persistent()
            .get(&StorageKey::Verification(project_id))
    }
}
