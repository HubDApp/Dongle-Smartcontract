use crate::types::VerificationRecord;
use soroban_sdk::{Env, Address, String};

pub struct VerificationRegistry;

impl VerificationRegistry {
    pub fn request_verification(_env: &Env, _project_id: u64, _requester: Address, _evidence_cid: String) {
        // Validate project ownership
        // Require fee paid via FeeManager
        // Store VerificationRecord with Pending
    }

    pub fn approve_verification(_env: &Env, _project_id: u64, _verifier: Address) {
        // Only admin/verifier
        // Set status to Verified
    }

    pub fn reject_verification(_env: &Env, _project_id: u64, _verifier: Address) {
        // Only admin/verifier
        // Set status to Rejected
    }

    pub fn get_verification(_env: &Env, _project_id: u64) -> Option<VerificationRecord> {
        None
    }
}
