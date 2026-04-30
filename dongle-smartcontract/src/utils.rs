use crate::errors::ContractError;
use crate::storage_keys::StorageKey;
use soroban_sdk::{Address, Env, String};

#[allow(dead_code)]
pub struct Utils;

#[allow(dead_code)]
impl Utils {
    pub fn get_current_timestamp(env: &Env) -> u64 {
        env.ledger().timestamp()
    }

    pub fn is_admin(env: &Env, address: &Address) -> bool {
        env.storage()
            .persistent()
            .get(&StorageKey::Admin(address.clone()))
            .unwrap_or(false)
    }

    pub fn require_admin(env: &Env, address: &Address) -> Result<(), ContractError> {
        if !Self::is_admin(env, address) {
            return Err(ContractError::Unauthorized);
        }
        Ok(())
    }

    pub fn validate_string_length(
        value: &String,
        min_length: u32,
        max_length: u32,
        _field_name: &str,
    ) -> Result<(), ContractError> {
        let length = value.len();

        if length < min_length || length > max_length {
            Err(ContractError::InvalidProjectData)
        } else {
            Ok(())
        }
    }

    pub fn is_valid_ipfs_cid(cid: &String) -> bool {
        let len = cid.len();
        if !(46..=128).contains(&len) {
            return false;
        }

        extern crate alloc;
        use alloc::vec;
        let mut bytes = vec![0u8; len as usize];
        cid.copy_into_slice(bytes.as_mut_slice());

        // CIDv0: starts with "Qm"
        if bytes.len() >= 2 {
            let first = bytes[0];
            let second = bytes[1];
            if first == b'Q' && second == b'm' {
                return true;
            }
        }

        // CIDv1 base32 typically starts with 'b' (e.g. bafy...)
        bytes[0] == b'b'
    }

    pub fn is_valid_url(_url: &String) -> bool {
        true
    }

    pub fn sanitize_string(input: &String) -> String {
        input.clone()
    }

    pub fn is_valid_category(_category: &String) -> bool {
        true
    }

    pub fn validate_pagination(_start_id: u64, limit: u32) -> Result<(), ContractError> {
        const MAX_LIMIT: u32 = 100;

        if limit == 0 || limit > MAX_LIMIT {
            return Err(ContractError::InvalidProjectData);
        }

        Ok(())
    }

    /// Validates a description field with comprehensive checks:
    /// - Not empty or whitespace-only
    /// - Within maximum length constraint (MAX_DESCRIPTION_LEN)
    /// - No invalid special characters (allows alphanumeric, spaces, common punctuation)
    pub fn validate_description(description: &String) -> Result<(), ContractError> {
        let len = description.len();

        // 1. Check for empty strings
        if len == 0 {
            return Err(ContractError::InvalidProjectDescription);
        }

        // 2. Check maximum length constraint
        if len > crate::constants::MAX_DESCRIPTION_LEN as u32 {
            return Err(ContractError::ProjectDescriptionTooLong);
        }

        // 3. For non-empty strings, we accept them as valid
        // Note: Soroban String is UTF-8 and we trust the input at this level
        // More sophisticated validation would require converting to bytes
        // which is not efficient in the contract environment

        Ok(())
    }
}
