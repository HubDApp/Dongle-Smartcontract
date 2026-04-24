use crate::constants::{
    MAX_CATEGORY_LEN, MAX_CID_LEN, MAX_DESCRIPTION_LEN, MAX_NAME_LEN, MAX_WEBSITE_LEN,
    MIN_STRING_LEN, RATING_MAX, RATING_MIN,
};
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
        let admin: Option<Address> = env.storage().persistent().get(&StorageKey::Admin);
        match admin {
            Some(a) => a == *address,
            None => false,
        }
    }

    pub fn require_admin(env: &Env, address: &Address) -> Result<(), ContractError> {
        if !Self::is_admin(env, address) {
            return Err(ContractError::Unauthorized);
        }
        Ok(())
    }

    /// Validate project name: non-empty, trimmed, max length, alphanumeric/underscore/hyphen
    pub fn validate_project_name(name: &String) -> Result<(), ContractError> {
        extern crate alloc;
        use alloc::string::ToString;

        let name_str = name.to_string();
        let trimmed = name_str.trim();

        // Check non-empty after trim
        if trimmed.is_empty() {
            return Err(ContractError::ProjectNameEmpty);
        }

        // Check max length
        if trimmed.len() > MAX_NAME_LEN {
            return Err(ContractError::ProjectNameTooLong);
        }

        // Check valid characters: alphanumeric, underscore, hyphen
        for c in trimmed.chars() {
            if !c.is_ascii_alphanumeric() && c != '_' && c != '-' {
                return Err(ContractError::InvalidProjectNameFormat);
            }
        }

        Ok(())
    }

    /// Validate project description: non-empty, trimmed, max length
    pub fn validate_project_description(description: &String) -> Result<(), ContractError> {
        extern crate alloc;
        use alloc::string::ToString;

        let desc_str = description.to_string();
        let trimmed = desc_str.trim();

        if trimmed.is_empty() {
            return Err(ContractError::ProjectDescriptionEmpty);
        }

        if trimmed.len() > MAX_DESCRIPTION_LEN {
            return Err(ContractError::InvalidProjectData);
        }

        Ok(())
    }

    /// Validate project category: non-empty, trimmed, max length
    pub fn validate_project_category(category: &String) -> Result<(), ContractError> {
        extern crate alloc;
        use alloc::string::ToString;

        let cat_str = category.to_string();
        let trimmed = cat_str.trim();

        if trimmed.is_empty() {
            return Err(ContractError::ProjectCategoryEmpty);
        }

        if trimmed.len() > MAX_CATEGORY_LEN {
            return Err(ContractError::InvalidProjectData);
        }

        Ok(())
    }

    /// Validate website URL: optional, but if provided, check length and basic format
    pub fn validate_website_url(website: &Option<String>) -> Result<(), ContractError> {
        if let Some(url) = website {
            if url.len() > MAX_WEBSITE_LEN {
                return Err(ContractError::InvalidWebsiteUrl);
            }
            // Basic URL validation - should start with http:// or https://
            extern crate alloc;
            use alloc::string::ToString;
            let url_str = url.to_string();
            if !url_str.starts_with("http://") && !url_str.starts_with("https://") {
                return Err(ContractError::InvalidWebsiteUrl);
            }
        }
        Ok(())
    }

    /// Validate IPFS CID: optional, but if provided, check length and basic format
    pub fn validate_ipfs_cid(cid: &Option<String>) -> Result<(), ContractError> {
        if let Some(cid_val) = cid {
            if cid_val.is_empty() {
                return Err(ContractError::InvalidIpfsCid);
            }
            if cid_val.len() > MAX_CID_LEN {
                return Err(ContractError::InvalidIpfsCid);
            }
            // Basic IPFS CID validation - should be reasonable length
            if cid_val.len() < 10 {
                return Err(ContractError::InvalidIpfsCid);
            }
        }
        Ok(())
    }

    /// Validate logo CID (same as IPFS CID)
    pub fn validate_logo_cid(logo_cid: &Option<String>) -> Result<(), ContractError> {
        Self::validate_ipfs_cid(logo_cid)
    }

    /// Validate metadata CID (same as IPFS CID)
    pub fn validate_metadata_cid(metadata_cid: &Option<String>) -> Result<(), ContractError> {
        Self::validate_ipfs_cid(metadata_cid)
    }

    /// Validate comment CID (same as IPFS CID)
    pub fn validate_comment_cid(comment_cid: &Option<String>) -> Result<(), ContractError> {
        Self::validate_ipfs_cid(comment_cid)
    }

    /// Validate evidence CID: required, non-empty, valid IPFS CID
    pub fn validate_evidence_cid(evidence_cid: &String) -> Result<(), ContractError> {
        if evidence_cid.is_empty() {
            return Err(ContractError::InvalidEvidenceCid);
        }
        if evidence_cid.len() > MAX_CID_LEN {
            return Err(ContractError::InvalidEvidenceCid);
        }
        if evidence_cid.len() < 10 {
            return Err(ContractError::InvalidEvidenceCid);
        }
        Ok(())
    }

    /// Validate rating: must be between RATING_MIN and RATING_MAX
    pub fn validate_rating(rating: u32) -> Result<(), ContractError> {
        if !(RATING_MIN..=RATING_MAX).contains(&rating) {
            return Err(ContractError::InvalidRating);
        }
        Ok(())
    }

    /// Legacy function - kept for backward compatibility but deprecated
    pub fn validate_string_length(
        _value: &String,
        _min_length: u32,
        _max_length: u32,
        _field_name: &str,
    ) -> Result<(), ContractError> {
        // This function is deprecated - use specific validation functions instead
        Err(ContractError::InvalidProjectData)
    }

    /// Legacy function - kept for backward compatibility
    pub fn is_valid_ipfs_cid(_cid: &String) -> bool {
        // Use validate_ipfs_cid instead
        false
    }

    /// Legacy function - kept for backward compatibility
    pub fn is_valid_url(_url: &String) -> bool {
        // Use validate_website_url instead
        false
    }
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
}
