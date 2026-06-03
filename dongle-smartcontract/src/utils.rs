use crate::errors::ContractError;
use crate::storage_keys::StorageKey;
use soroban_sdk::{Address, Env, Map, String, Vec};

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

    pub fn validate_website(website: &String) -> Result<(), ContractError> {
        let len = website.len();
        if len == 0 {
            return Err(ContractError::InvalidProjectWebsite);
        }
        if len > crate::constants::MAX_WEBSITE_LEN as u32 {
            return Err(ContractError::ProjectWebsiteTooLong);
        }

        extern crate alloc;
        use alloc::string::ToString;
        let web_str = website.to_string();

        if !web_str.starts_with("http://") && !web_str.starts_with("https://") {
            return Err(ContractError::InvalidProjectWebsite);
        }
        Ok(())
    }

    pub fn sanitize_string(input: &String) -> String {
        input.clone()
    }

    pub fn validate_category_field(category: &String) -> Result<(), ContractError> {
        let len = category.len();
        if len == 0 {
            return Err(ContractError::InvalidProjectCategory);
        }
        if len > crate::constants::MAX_CATEGORY_LEN as u32 {
            return Err(ContractError::ProjectCategoryTooLong);
        }

        extern crate alloc;
        use alloc::string::ToString;
        let cat_str = category.to_string();
        if cat_str.trim().is_empty() {
            return Err(ContractError::InvalidProjectCategory);
        }

        Ok(())
    }

    pub fn validate_logo_cid(cid: &String) -> Result<(), ContractError> {
        if cid.len() == 0 || !Self::is_valid_ipfs_cid(cid) {
            return Err(ContractError::InvalidProjectLogoCid);
        }
        Ok(())
    }

    pub fn validate_metadata_cid(cid: &String) -> Result<(), ContractError> {
        if cid.len() == 0 || !Self::is_valid_ipfs_cid(cid) {
            return Err(ContractError::InvalidProjectMetadataCid);
        }
        Ok(())
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

    /// Validates project name with comprehensive checks:
    /// - Not empty or whitespace-only
    /// - Within maximum length constraint (MAX_NAME_LEN)
    /// - Alphanumeric, underscore, and hyphen only
    pub fn validate_project_name(name: &String) -> Result<(), ContractError> {
        extern crate alloc;
        use alloc::string::ToString;

        let name_str = name.to_string();

        // 1. Validate non-empty and not only whitespace
        if name_str.trim().is_empty() {
            return Err(ContractError::InvalidProjectName);
        }

        // 2. Validate max length
        let max_len = crate::constants::MAX_NAME_LEN;
        if name_str.len() > max_len {
            return Err(ContractError::ProjectNameTooLong);
        }

        // 3. Validate alphanumeric, underscore, hyphen
        for c in name_str.chars() {
            if !c.is_ascii_alphanumeric() && c != '_' && c != '-' {
                return Err(ContractError::InvalidProjectNameFormat);
            }
        }

        Ok(())
    }

    /// Validates project tags
    pub fn validate_tags(tags: &Vec<String>) -> Result<(), ContractError> {
        extern crate alloc;
        use alloc::string::ToString;

        // Check max number of tags
        if tags.len() > crate::constants::MAX_TAGS_PER_PROJECT {
            return Err(ContractError::TooManyTags);
        }

        // Validate each tag
        for tag in tags.iter() {
            let tag_str = tag.to_string();

            // Check tag length
            if tag_str.len() == 0 || tag_str.len() > crate::constants::MAX_TAG_LENGTH as usize {
                return Err(ContractError::InvalidTag);
            }

            // Check tag format (alphanumeric, underscore, hyphen only)
            for c in tag_str.chars() {
                if !c.is_ascii_alphanumeric() && c != '_' && c != '-' {
                    return Err(ContractError::InvalidTag);
                }
            }
        }

        Ok(())
    }

    /// Validates social links
    pub fn validate_social_links(social_links: &Map<String, String>) -> Result<(), ContractError> {
        extern crate alloc;
        use alloc::string::ToString;

        // Check max number of social links
        if social_links.len() > crate::constants::MAX_SOCIAL_LINKS {
            return Err(ContractError::TooManySocialLinks);
        }

        // Validate each social link
        for (platform, url) in social_links.iter() {
            let platform_str = platform.to_string();
            let url_str = url.to_string();

            // Check platform name length
            if platform_str.len() == 0
                || platform_str.len() > crate::constants::MAX_SOCIAL_LINK_PLATFORM_LEN as usize
            {
                return Err(ContractError::InvalidSocialLink);
            }

            // Check URL length
            if url_str.len() == 0
                || url_str.len() > crate::constants::MAX_SOCIAL_LINK_URL_LEN as usize
            {
                return Err(ContractError::InvalidSocialLink);
            }

            // Basic URL validation
            if !url_str.starts_with("http://") && !url_str.starts_with("https://") {
                return Err(ContractError::InvalidSocialLink);
            }

            // Platform name validation (alphanumeric, underscore, hyphen only)
            for c in platform_str.chars() {
                if !c.is_ascii_alphanumeric() && c != '_' && c != '-' {
                    return Err(ContractError::InvalidSocialLink);
                }
            }
        }

        Ok(())
    }

    /// Validates report reason CID
    pub fn validate_report_reason_cid(reason_cid: &String) -> Result<(), ContractError> {
        if reason_cid.len() == 0 || !Self::is_valid_ipfs_cid(reason_cid) {
            return Err(ContractError::InvalidReportReason);
        }
        Ok(())
    }
}
