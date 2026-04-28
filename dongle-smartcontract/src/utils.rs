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
        (46..=100).contains(&len)
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
        extern crate alloc;
        use alloc::string::ToString;

        let desc_str = description.to_string();

        // 1. Check for empty or whitespace-only strings
        if desc_str.trim().is_empty() {
            return Err(ContractError::InvalidProjectDescription);
        }

        // 2. Check maximum length constraint
        if desc_str.len() > crate::constants::MAX_DESCRIPTION_LEN {
            return Err(ContractError::ProjectDescriptionTooLong);
        }

        // 3. Validate characters - allow alphanumeric, spaces, and common punctuation
        // Allowed: letters, digits, spaces, and: . , ! ? - ( ) ' " : ; / &
        for c in desc_str.chars() {
            let is_valid = c.is_ascii_alphanumeric()
                || c.is_whitespace()
                || matches!(
                    c,
                    '.' | ',' | '!' | '?' | '-' | '(' | ')' | '\'' | '"' | ':' | ';' | '/' | '&'
                );

            if !is_valid {
                return Err(ContractError::InvalidProjectDescriptionFormat);
            }
        }

        Ok(())
    }
}
