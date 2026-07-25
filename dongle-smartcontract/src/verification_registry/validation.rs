//! Verification input validation (evidence CIDs, etc.).

use crate::constants::MAX_CID_LEN;
use crate::errors::ContractError;
use crate::utils::Utils;
use soroban_sdk::String;

/// Validation helpers for verification request/renewal paths.
pub struct VerificationValidation;

impl VerificationValidation {
    pub fn validate_evidence_cid(evidence_cid: &String) -> Result<(), ContractError> {
        if evidence_cid.is_empty() {
            return Err(ContractError::InvalidProjectData);
        }
        if !Utils::is_valid_ipfs_cid(evidence_cid) || evidence_cid.len() as usize > MAX_CID_LEN {
            return Err(ContractError::InvalidProjectData);
        }
        Ok(())
    }
}
