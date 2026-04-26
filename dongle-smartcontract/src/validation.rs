//! Input validation module for all user-provided data.
//! Ensures data integrity and prevents abuse through strict validation rules.

use crate::constants::*;
use crate::errors::ContractError;
use soroban_sdk::String;

/// Validates project name according to defined rules:
/// - Non-empty after trimming whitespace
/// - Length between MIN_STRING_LEN and MAX_NAME_LEN
/// - Contains only alphanumeric characters, underscores, and hyphens
pub fn validate_project_name(name: &String) -> Result<(), ContractError> {
    extern crate alloc;
    use alloc::string::ToString;

    let name_str = name.to_string();
    let trimmed = name_str.trim();

    // Check non-empty
    if trimmed.is_empty() {
        return Err(ContractError::InvalidProjectName);
    }

    // Check length
    if trimmed.len() < MIN_STRING_LEN || trimmed.len() > MAX_NAME_LEN {
        return Err(ContractError::ProjectNameTooLong);
    }

    // Check format: alphanumeric, underscore, hyphen, space
    for c in trimmed.chars() {
        if !c.is_ascii_alphanumeric() && c != '_' && c != '-' && c != ' ' {
            return Err(ContractError::InvalidProjectNameFormat);
        }
    }

    Ok(())
}

/// Validates project description:
/// - Non-empty after trimming
/// - Length between MIN_STRING_LEN and MAX_DESCRIPTION_LEN
pub fn validate_description(description: &String) -> Result<(), ContractError> {
    extern crate alloc;
    use alloc::string::ToString;

    let desc_str = description.to_string();
    let trimmed = desc_str.trim();

    if trimmed.is_empty() {
        return Err(ContractError::InvalidDescription);
    }

    if trimmed.len() < MIN_STRING_LEN || trimmed.len() > MAX_DESCRIPTION_LEN {
        return Err(ContractError::DescriptionTooLong);
    }

    Ok(())
}

/// Validates project category:
/// - Non-empty after trimming
/// - Length between MIN_STRING_LEN and MAX_CATEGORY_LEN
/// - Contains only alphanumeric characters, underscores, hyphens, and spaces
pub fn validate_category(category: &String) -> Result<(), ContractError> {
    extern crate alloc;
    use alloc::string::ToString;

    let cat_str = category.to_string();
    let trimmed = cat_str.trim();

    if trimmed.is_empty() {
        return Err(ContractError::InvalidCategory);
    }

    if trimmed.len() < MIN_STRING_LEN || trimmed.len() > MAX_CATEGORY_LEN {
        return Err(ContractError::CategoryTooLong);
    }

    // Allow alphanumeric, underscore, hyphen, space
    for c in trimmed.chars() {
        if !c.is_ascii_alphanumeric() && c != '_' && c != '-' && c != ' ' {
            return Err(ContractError::InvalidCategoryFormat);
        }
    }

    Ok(())
}

/// Validates website URL:
/// - If provided, must be non-empty
/// - Length must not exceed MAX_WEBSITE_LEN
/// - Must start with http:// or https://
pub fn validate_website(website: &Option<String>) -> Result<(), ContractError> {
    extern crate alloc;
    use alloc::string::ToString;

    if let Some(url) = website {
        let url_str = url.to_string();
        let trimmed = url_str.trim();

        if trimmed.is_empty() {
            return Err(ContractError::InvalidWebsiteUrl);
        }

        if trimmed.len() > MAX_WEBSITE_LEN {
            return Err(ContractError::WebsiteUrlTooLong);
        }

        // Basic URL format check
        if !trimmed.starts_with("http://") && !trimmed.starts_with("https://") {
            return Err(ContractError::InvalidWebsiteUrlFormat);
        }
    }

    Ok(())
}

/// Validates CID (Content Identifier) for IPFS/similar systems:
/// - If provided, must be non-empty
/// - Length must be between MIN_CID_LEN and MAX_CID_LEN
/// - Contains only alphanumeric characters (base58/base32 encoding)
pub fn validate_cid(cid: &Option<String>) -> Result<(), ContractError> {
    extern crate alloc;
    use alloc::string::ToString;

    if let Some(cid_val) = cid {
        let cid_str = cid_val.to_string();
        let trimmed = cid_str.trim();

        if trimmed.is_empty() {
            return Err(ContractError::InvalidCid);
        }

        if trimmed.len() < MIN_CID_LEN || trimmed.len() > MAX_CID_LEN {
            return Err(ContractError::CidInvalidLength);
        }

        // CIDs are typically base58 or base32 encoded (alphanumeric)
        for c in trimmed.chars() {
            if !c.is_ascii_alphanumeric() {
                return Err(ContractError::InvalidCidFormat);
            }
        }
    }

    Ok(())
}

/// Validates rating value:
/// - Must be between RATING_MIN and RATING_MAX (inclusive)
pub fn validate_rating(rating: u32) -> Result<(), ContractError> {
    if !(RATING_MIN..=RATING_MAX).contains(&rating) {
        return Err(ContractError::InvalidRating);
    }
    Ok(())
}

/// Validates pagination parameters:
/// - Limit must be greater than 0
/// - Limit must not exceed MAX_PAGINATION_LIMIT
pub fn validate_pagination(limit: u32) -> Result<(), ContractError> {
    if limit == 0 {
        return Err(ContractError::InvalidPaginationLimit);
    }

    if limit > MAX_PAGINATION_LIMIT {
        return Err(ContractError::PaginationLimitTooLarge);
    }

    Ok(())
}

/// Validates evidence CID for verification requests:
/// - Must be non-empty
/// - Length must be between MIN_CID_LEN and MAX_CID_LEN
/// - Contains only alphanumeric characters
pub fn validate_evidence_cid(evidence_cid: &String) -> Result<(), ContractError> {
    extern crate alloc;
    use alloc::string::ToString;

    let cid_str = evidence_cid.to_string();
    let trimmed = cid_str.trim();

    if trimmed.is_empty() {
        return Err(ContractError::InvalidCid);
    }

    if trimmed.len() < MIN_CID_LEN || trimmed.len() > MAX_CID_LEN {
        return Err(ContractError::CidInvalidLength);
    }

    for c in trimmed.chars() {
        if !c.is_ascii_alphanumeric() {
            return Err(ContractError::InvalidCidFormat);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{Env, String};

    // ── Project Name Tests ──

    #[test]
    fn test_valid_project_name() {
        let env = Env::default();
        let name = String::from_str(&env, "Valid-Project_Name123");
        assert!(validate_project_name(&name).is_ok());
    }

    #[test]
    fn test_project_name_with_spaces() {
        let env = Env::default();
        let name = String::from_str(&env, "My Project Name");
        assert!(validate_project_name(&name).is_ok());
    }

    #[test]
    fn test_empty_project_name() {
        let env = Env::default();
        let name = String::from_str(&env, "");
        assert_eq!(
            validate_project_name(&name),
            Err(ContractError::InvalidProjectName)
        );
    }

    #[test]
    fn test_whitespace_only_project_name() {
        let env = Env::default();
        let name = String::from_str(&env, "   ");
        assert_eq!(
            validate_project_name(&name),
            Err(ContractError::InvalidProjectName)
        );
    }

    #[test]
    fn test_project_name_too_long() {
        let env = Env::default();
        // 51 characters
        let name = String::from_str(&env, "ThisProjectNameIsWayTooLongAndExceedsTheFiftyCharL1");
        assert_eq!(
            validate_project_name(&name),
            Err(ContractError::ProjectNameTooLong)
        );
    }

    #[test]
    fn test_project_name_invalid_characters() {
        let env = Env::default();
        let name = String::from_str(&env, "Project@Name!");
        assert_eq!(
            validate_project_name(&name),
            Err(ContractError::InvalidProjectNameFormat)
        );
    }

    // ── Description Tests ──

    #[test]
    fn test_valid_description() {
        let env = Env::default();
        let desc = String::from_str(&env, "This is a valid project description.");
        assert!(validate_description(&desc).is_ok());
    }

    #[test]
    fn test_empty_description() {
        let env = Env::default();
        let desc = String::from_str(&env, "");
        assert_eq!(
            validate_description(&desc),
            Err(ContractError::InvalidDescription)
        );
    }

    #[test]
    fn test_description_too_long() {
        let env = Env::default();
        let long_desc = "a".repeat(MAX_DESCRIPTION_LEN + 1);
        let desc = String::from_str(&env, &long_desc);
        assert_eq!(
            validate_description(&desc),
            Err(ContractError::DescriptionTooLong)
        );
    }

    // ── Category Tests ──

    #[test]
    fn test_valid_category() {
        let env = Env::default();
        let cat = String::from_str(&env, "DeFi");
        assert!(validate_category(&cat).is_ok());
    }

    #[test]
    fn test_category_with_spaces() {
        let env = Env::default();
        let cat = String::from_str(&env, "Decentralized Finance");
        assert!(validate_category(&cat).is_ok());
    }

    #[test]
    fn test_empty_category() {
        let env = Env::default();
        let cat = String::from_str(&env, "");
        assert_eq!(
            validate_category(&cat),
            Err(ContractError::InvalidCategory)
        );
    }

    #[test]
    fn test_category_too_long() {
        let env = Env::default();
        let long_cat = "a".repeat(MAX_CATEGORY_LEN + 1);
        let cat = String::from_str(&env, &long_cat);
        assert_eq!(
            validate_category(&cat),
            Err(ContractError::CategoryTooLong)
        );
    }

    // ── Website Tests ──

    #[test]
    fn test_valid_website() {
        let env = Env::default();
        let url = Some(String::from_str(&env, "https://example.com"));
        assert!(validate_website(&url).is_ok());
    }

    #[test]
    fn test_website_http() {
        let env = Env::default();
        let url = Some(String::from_str(&env, "http://example.com"));
        assert!(validate_website(&url).is_ok());
    }

    #[test]
    fn test_website_none() {
        assert!(validate_website(&None).is_ok());
    }

    #[test]
    fn test_website_invalid_protocol() {
        let env = Env::default();
        let url = Some(String::from_str(&env, "ftp://example.com"));
        assert_eq!(
            validate_website(&url),
            Err(ContractError::InvalidWebsiteUrlFormat)
        );
    }

    #[test]
    fn test_website_too_long() {
        let env = Env::default();
        let long_url = format!("https://{}.com", "a".repeat(MAX_WEBSITE_LEN));
        let url = Some(String::from_str(&env, &long_url));
        assert_eq!(
            validate_website(&url),
            Err(ContractError::WebsiteUrlTooLong)
        );
    }

    // ── CID Tests ──

    #[test]
    fn test_valid_cid() {
        let env = Env::default();
        let cid = Some(String::from_str(
            &env,
            "QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG",
        ));
        assert!(validate_cid(&cid).is_ok());
    }

    #[test]
    fn test_cid_none() {
        assert!(validate_cid(&None).is_ok());
    }

    #[test]
    fn test_cid_too_short() {
        let env = Env::default();
        let cid = Some(String::from_str(&env, "short"));
        assert_eq!(validate_cid(&cid), Err(ContractError::CidInvalidLength));
    }

    #[test]
    fn test_cid_too_long() {
        let env = Env::default();
        let long_cid = "a".repeat(MAX_CID_LEN + 1);
        let cid = Some(String::from_str(&env, &long_cid));
        assert_eq!(validate_cid(&cid), Err(ContractError::CidInvalidLength));
    }

    #[test]
    fn test_cid_invalid_characters() {
        let env = Env::default();
        let cid = Some(String::from_str(
            &env,
            "Qm@#$%^&*()_+{}|:<>?~`-=[]\\;',./",
        ));
        assert_eq!(validate_cid(&cid), Err(ContractError::InvalidCidFormat));
    }

    // ── Rating Tests ──

    #[test]
    fn test_valid_ratings() {
        for rating in RATING_MIN..=RATING_MAX {
            assert!(validate_rating(rating).is_ok());
        }
    }

    #[test]
    fn test_rating_too_low() {
        assert_eq!(
            validate_rating(RATING_MIN - 1),
            Err(ContractError::InvalidRating)
        );
    }

    #[test]
    fn test_rating_too_high() {
        assert_eq!(
            validate_rating(RATING_MAX + 1),
            Err(ContractError::InvalidRating)
        );
    }

    // ── Pagination Tests ──

    #[test]
    fn test_valid_pagination() {
        assert!(validate_pagination(10).is_ok());
        assert!(validate_pagination(MAX_PAGINATION_LIMIT).is_ok());
    }

    #[test]
    fn test_pagination_zero() {
        assert_eq!(
            validate_pagination(0),
            Err(ContractError::InvalidPaginationLimit)
        );
    }

    #[test]
    fn test_pagination_too_large() {
        assert_eq!(
            validate_pagination(MAX_PAGINATION_LIMIT + 1),
            Err(ContractError::PaginationLimitTooLarge)
        );
    }

    // ── Evidence CID Tests ──

    #[test]
    fn test_valid_evidence_cid() {
        let env = Env::default();
        let cid = String::from_str(&env, "QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG");
        assert!(validate_evidence_cid(&cid).is_ok());
    }

    #[test]
    fn test_evidence_cid_empty() {
        let env = Env::default();
        let cid = String::from_str(&env, "");
        assert_eq!(
            validate_evidence_cid(&cid),
            Err(ContractError::InvalidCid)
        );
    }
}
