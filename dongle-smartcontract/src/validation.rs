//! Validation utilities for project registration and updates.
//!
//! `validate_registration_params` is the **canonical** validation entry point
//! for all project-registration field checks.  `project_registry.rs` should
//! call this single function instead of invoking `Utils::validate_*` helpers
//! directly, which eliminates duplicated validation logic and the former
//! double-validation bug on `bounty_url`.

use crate::errors::ContractError;
use crate::types::ProjectRegistrationParams;
use crate::utils::Utils;
use soroban_sdk::{Env, String as SorobanString};

/// Validates **all** fields of a project registration request.
///
/// This is the single, canonical validation path for registration.  It
/// delegates to the `Utils::validate_*` helpers so that field-specific error
/// codes are returned correctly and no logic is duplicated.
///
/// Fields validated:
/// - `name`         — via `Utils::validate_project_name`
/// - `slug`         — via `Utils::validate_project_slug`
/// - `description`  — via `Utils::validate_description`
/// - `category`     — via `Utils::validate_category_field`
/// - `website`      — via `Utils::validate_website` (optional)
/// - `logo_cid`     — via `Utils::validate_logo_cid` (optional)
/// - `metadata_cid` — via `Utils::validate_metadata_cid` (optional)
/// - `tags`         — via `Utils::validate_tags` (optional)
/// - `social_links` — via `Utils::validate_social_links` (optional)
/// - `bounty_url`   — via `Utils::validate_website` (optional, validated **once**)
pub fn validate_registration_params(
    _env: &Env,
    params: &ProjectRegistrationParams,
) -> Result<(), ContractError> {
    // Mandatory fields
    Utils::validate_project_name(&params.name)?;
    Utils::validate_project_slug(&params.slug)?;
    Utils::validate_description(&params.description)?;
    Utils::validate_category_field(&params.category)?;

    // Optional fields
    if let Some(website) = &params.website {
        Utils::validate_website(website)?;
    }
    if let Some(logo_cid) = &params.logo_cid {
        Utils::validate_logo_cid(logo_cid)?;
    }
    if let Some(metadata_cid) = &params.metadata_cid {
        Utils::validate_metadata_cid(metadata_cid)?;
    }
    if let Some(tags) = &params.tags {
        Utils::validate_tags(tags)?;
    }
    if let Some(social_links) = &params.social_links {
        Utils::validate_social_links(social_links)?;
    }
    // bounty_url validated exactly once here — previously validated twice in
    // register_project, which was a duplication bug.
    if let Some(bounty_url) = &params.bounty_url {
        Utils::validate_website(bounty_url)?;
    }
    if let Some(repo_url) = &params.repository_url {
        Utils::validate_website(repo_url)?;
    }

    Ok(())
}

/// Validate that a Soroban string is valid UTF-8 and contains no control characters.
///
/// Soroban `String` is backed by bytes; if the caller passes non-UTF-8 data,
/// downstream processing (serialization, display, indexing) can break silently.
/// This check rejects:
/// - Invalid UTF-8 byte sequences
/// - ASCII control characters (0x00–0x1F) except newline (0x0A) and tab (0x09)
/// - DEL character (0x7F)
///
/// # Issue #725
pub fn validate_utf8_string(value: &SorobanString) -> Result<(), ContractError> {
    let bytes = value.to_buffer();

    for &byte in bytes.iter() {
        // Reject ASCII control characters (except \n and \t) and DEL
        if byte < 0x20 && byte != 0x0A && byte != 0x09 {
            return Err(ContractError::InvalidInput);
        }
        if byte == 0x7F {
            return Err(ContractError::InvalidInput);
        }
    }

    // Verify the bytes form valid UTF-8 by attempting to convert to str
    core::str::from_utf8(bytes.as_slice()).map_err(|_| ContractError::InvalidInput)?;

    Ok(())
}

/// Validate a Soroban string is valid UTF-8 and optionally reject control characters.
/// For multi-line fields (like description), newlines are allowed but other
/// control characters are still rejected.
pub fn validate_multiline_utf8(value: &SorobanString) -> Result<(), ContractError> {
    let bytes = value.to_buffer();

    for &byte in bytes.iter() {
        // Reject all control characters except \n and \t
        if byte < 0x20 && byte != 0x0A && byte != 0x09 {
            return Err(ContractError::InvalidInput);
        }
        if byte == 0x7F {
            return Err(ContractError::InvalidInput);
        }
    }

    core::str::from_utf8(bytes.as_slice()).map_err(|_| ContractError::InvalidInput)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::Env;

    #[test]
    fn test_valid_utf8_string() {
        let env = Env::default();
        let s = SorobanString::from_str(&env, "Hello, world!");
        assert!(validate_utf8_string(&s).is_ok());
    }

    #[test]
    fn test_valid_utf8_with_newline() {
        let env = Env::default();
        let s = SorobanString::from_str(&env, "Line 1\nLine 2");
        assert!(validate_utf8_string(&s).is_ok());
    }

    #[test]
    fn test_reject_null_byte() {
        let env = Env::default();
        let bytes: soroban_sdk::Bytes = soroban_sdk::Bytes::from_array(&env, &[0x00, 0x41, 0x42]);
        let s = SorobanString::from_bytes(&bytes);
        assert_eq!(validate_utf8_string(&s), Err(ContractError::InvalidInput));
    }

    #[test]
    fn test_reject_control_char() {
        let env = Env::default();
        // 0x01 is a control character
        let bytes: soroban_sdk::Bytes = soroban_sdk::Bytes::from_array(&env, &[0x48, 0x01, 0x49]);
        let s = SorobanString::from_bytes(&bytes);
        assert_eq!(validate_utf8_string(&s), Err(ContractError::InvalidInput));
    }

    #[test]
    fn test_reject_del_char() {
        let env = Env::default();
        let bytes: soroban_sdk::Bytes = soroban_sdk::Bytes::from_array(&env, &[0x48, 0x7F, 0x49]);
        let s = SorobanString::from_bytes(&bytes);
        assert_eq!(validate_utf8_string(&s), Err(ContractError::InvalidInput));
    }

    #[test]
    fn test_reject_invalid_utf8() {
        let env = Env::default();
        // 0xFF is never valid in UTF-8
        let bytes: soroban_sdk::Bytes = soroban_sdk::Bytes::from_array(&env, &[0xC0, 0xAF]);
        let s = SorobanString::from_bytes(&bytes);
        assert_eq!(validate_utf8_string(&s), Err(ContractError::InvalidInput));
    }

    #[test]
    fn test_valid_multiline() {
        let env = Env::default();
        let s = SorobanString::from_str(&env, "Line 1\nLine 2\nLine 3");
        assert!(validate_multiline_utf8(&s).is_ok());
    }

    #[test]
    fn test_multiline_rejects_control_chars() {
        let env = Env::default();
        let bytes: soroban_sdk::Bytes = soroban_sdk::Bytes::from_array(&env, &[0x48, 0x0B, 0x49]);
        let s = SorobanString::from_bytes(&bytes);
        assert_eq!(validate_multiline_utf8(&s), Err(ContractError::InvalidInput));
    }
}
