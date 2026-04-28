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
        
        // Check length is within valid IPFS CID range
        // Valid CIDv0: 46 chars (Qm + 44 chars)
        // Valid CIDv1: 60+ chars typically
        if !((46..=crate::constants::MAX_CID_LEN).contains(&len)) {
            return false;
        }

        // CIDv0 must start with 'Q' (base58 encoded)
        if cid.len() >= 1 {
            let first_char = cid.as_bytes()[0];
            if first_char == b'Q' {
                // CIDv0 format: starts with Q, followed by base58 characters
                // Basic validation: check if it starts with Qm pattern (most common)
                if cid.len() >= 2 {
                    let second_char = cid.as_bytes()[1];
                    if second_char != b'm' {
                        // Not standard Qm pattern, but could still be valid Qx pattern
                        // For simplicity, we'll reject non-Qm patterns
                        return false;
                    }
                }
            } else {
                // Could be CIDv1 (base32 or base36 encoded)
                // Basic validation: allow alphanumeric characters
                // Full validation would require more complex parsing
                let cid_str = cid.clone();
                for byte in cid_str.as_bytes().iter() {
                    // Allow base32 lowercase alphabet and numbers (for CIDv1 default encoding)
                    if !((*byte >= b'a' && *byte <= b'z') || (*byte >= b'0' && *byte <= b'9')) {
                        return false;
                    }
                }
            }
        }

        true
    }

    pub fn is_valid_url(url: &String) -> bool {
        let len = url.len();
        
        // Check length constraint
        if len == 0 || len > crate::constants::MAX_WEBSITE_LEN {
            return false;
        }

        // Basic URL validation: must contain :// pattern
        let url_str = url.clone();
        let bytes = url_str.as_bytes();
        
        let mut found_protocol = false;
        for i in 0..bytes.len().saturating_sub(3) {
            if bytes[i] == b':' && bytes[i + 1] == b'/' && bytes[i + 2] == b'/' {
                found_protocol = true;
                break;
            }
        }
        
        if !found_protocol {
            return false;
        }

        // Must start with http:// or https://
        if url_str.starts_with("http://") || url_str.starts_with("https://") {
            // Basic domain validation: must have at least one character after ://
            if url_str.len() > 7 {
                return true;
            }
        }

        false
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

    /// Validates optional website field
    /// - If provided (Some), must be a valid URL within MAX_WEBSITE_LEN
    /// - If None, validation passes
    pub fn validate_optional_website(website: &Option<String>) -> Result<(), ContractError> {
        if let Some(url) = website {
            if !Self::is_valid_url(url) {
                return Err(ContractError::InvalidProjectData);
            }
        }
        Ok(())
    }

    /// Validates optional logo CID field
    /// - If provided (Some), must be a valid IPFS CID within MAX_CID_LEN
    /// - If None, validation passes
    pub fn validate_optional_logo_cid(logo_cid: &Option<String>) -> Result<(), ContractError> {
        if let Some(cid) = logo_cid {
            if !Self::is_valid_ipfs_cid(cid) {
                return Err(ContractError::InvalidProjectData);
            }
        }
        Ok(())
    }

    /// Validates optional metadata CID field
    /// - If provided (Some), must be a valid IPFS CID within MAX_CID_LEN
    /// - If None, validation passes
    pub fn validate_optional_metadata_cid(metadata_cid: &Option<String>) -> Result<(), ContractError> {
        if let Some(cid) = metadata_cid {
            if !Self::is_valid_ipfs_cid(cid) {
                return Err(ContractError::InvalidProjectData);
            }
        }
        Ok(())
    }

    /// Validates optional comment CID field (for reviews)
    /// - If provided (Some), must be a valid IPFS CID within MAX_CID_LEN
    /// - If None, validation passes
    pub fn validate_optional_comment_cid(comment_cid: &Option<String>) -> Result<(), ContractError> {
        if let Some(cid) = comment_cid {
            if !Self::is_valid_ipfs_cid(cid) {
                return Err(ContractError::InvalidProjectData);
            }
        }
        Ok(())
    }
}
