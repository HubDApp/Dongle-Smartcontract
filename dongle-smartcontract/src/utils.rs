//! Utility functions and the `Utils` struct used throughout the contract.

use soroban_sdk::{Address, Env, String};

use crate::constants::{
    MAX_CATEGORY_LEN, MAX_CID_LEN, MAX_DESCRIPTION_LEN, MAX_LICENSE_LEN, MAX_NAME_LEN,
    MAX_SECURITY_CONTACT_LEN, MAX_SLUG_LEN, MAX_WEBSITE_LEN,
};
use crate::errors::ContractError;
use crate::storage_keys::StorageKey;
use crate::types::Project;

/// Check if contract is initialized
pub fn is_initialized(env: &Env) -> bool {
    env.storage().instance().has(&StorageKey::Initialized)
}

/// Check if address is a maintainer of the project (free function).
pub fn is_maintainer(env: &Env, project: &Project, address: &Address) -> bool {
    if let Some(ref maintainers) = project.maintainers {
        maintainers.contains(address)
    } else {
        false
    }
}

/// Utility struct — all methods are associated functions (no instance needed).
pub struct Utils;

impl Utils {
    // ────────────────────────────────────────────────────────────────────
    // Name normalization
    // ────────────────────────────────────────────────────────────────────

    /// Normalize a project name for duplicate-detection purposes.
    ///
    /// Rules applied (in order):
    /// 1. ASCII-lowercase all letters.
    /// 2. Collapse all whitespace sequences to a single space.
    /// 3. Strip leading and trailing whitespace.
    /// 4. Remove all punctuation characters (retaining only `[a-z0-9 _-]`).
    ///
    /// Two names that produce the same normalized form are considered
    /// duplicates regardless of their original casing, spacing, or punctuation.
    ///
    /// Examples:
    ///   "MyProject"   → "myproject"
    ///   "MY PROJECT"  → "my project"
    ///   "my-project"  → "my-project"
    ///   "My.Project!" → "my project"   (dots and ! removed)
    ///   "  My  Project  " → "my project"
    pub fn normalize_project_name(env: &Env, name: &String) -> String {
        let bytes = name.as_bytes();
        let len = bytes.len();

        // Allocate a buffer of the same size (normalization can only shrink or
        // preserve length when working on ASCII bytes).
        let mut buf = [0u8; 64]; // MAX_NAME_LEN is 50, safe upper bound
        let cap = if len < buf.len() { len } else { buf.len() };

        let mut out_len: usize = 0;
        let mut last_was_space = true; // treat start as "space" to strip leading

        for i in 0..cap {
            let b = bytes[i];
            let normalized = if b.is_ascii_uppercase() {
                // lowercase
                b + 32
            } else if b == b' ' || b == b'\t' || b == b'\n' || b == b'\r' {
                // whitespace → single space (collapsed)
                b' '
            } else if b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-' {
                // keep as-is
                b
            } else {
                // punctuation / special chars → strip (replace with space to
                // avoid merging adjacent words: "foo.bar" → "foo bar")
                b' '
            };

            if normalized == b' ' {
                if !last_was_space && out_len < cap {
                    buf[out_len] = b' ';
                    out_len += 1;
                }
                last_was_space = true;
            } else {
                buf[out_len] = normalized;
                out_len += 1;
                last_was_space = false;
            }
        }

        // Trim trailing space
        while out_len > 0 && buf[out_len - 1] == b' ' {
            out_len -= 1;
        }

        // Convert back to a Soroban String
        // SAFETY: all bytes are valid ASCII (subset of UTF-8).
        let s = core::str::from_utf8(&buf[..out_len]).unwrap_or("");
        String::from_str(env, s)
    }

    /// Convert a Soroban `String` to lowercase (ASCII only).
    /// Used by the reserved-name checker and other case-insensitive comparisons.
    pub fn to_lowercase(env: &Env, s: &String) -> String {
        let bytes = s.as_bytes();
        let len = bytes.len();
        let mut buf = [0u8; 256];
        let cap = if len < buf.len() { len } else { buf.len() };
        for i in 0..cap {
            buf[i] = if bytes[i].is_ascii_uppercase() {
                bytes[i] + 32
            } else {
                bytes[i]
            };
        }
        let s = core::str::from_utf8(&buf[..cap]).unwrap_or("");
        String::from_str(env, s)
    }

    // ────────────────────────────────────────────────────────────────────
    // Name / slug / field validation
    // ────────────────────────────────────────────────────────────────────

    /// Validate a project name.
    ///
    /// Rules:
    /// - Non-empty.
    /// - At most `MAX_NAME_LEN` bytes.
    /// - Only ASCII alphanumeric, `-`, or `_` characters (no spaces, no punctuation).
    /// - Not purely whitespace.
    pub fn validate_project_name(name: &String) -> Result<(), ContractError> {
        let bytes = name.as_bytes();
        if bytes.is_empty() {
            return Err(ContractError::InvalidProjectName);
        }
        if bytes.len() > MAX_NAME_LEN {
            return Err(ContractError::ProjectNameTooLong);
        }
        let mut all_ws = true;
        for &b in bytes.iter() {
            if !b.is_ascii_alphanumeric() && b != b'-' && b != b'_' {
                return Err(ContractError::InvalidProjectName);
            }
            if !b.is_ascii_whitespace() {
                all_ws = false;
            }
        }
        if all_ws {
            return Err(ContractError::InvalidProjectName);
        }
        Ok(())
    }

    /// Validate a project slug.
    ///
    /// Rules:
    /// - Non-empty, at most `MAX_SLUG_LEN` bytes.
    /// - Lowercase alphanumeric plus `-` or `_`.
    /// - No leading or trailing `-`.
    pub fn validate_project_slug(slug: &String) -> Result<(), ContractError> {
        let bytes = slug.as_bytes();
        if bytes.is_empty() {
            return Err(ContractError::InvalidProjectSlug);
        }
        if bytes.len() > MAX_SLUG_LEN {
            return Err(ContractError::InvalidProjectSlugLen);
        }
        for (i, &b) in bytes.iter().enumerate() {
            if !b.is_ascii_alphanumeric() && b != b'-' && b != b'_' {
                return Err(ContractError::InvalidProjectSlug);
            }
            if b == b'-' && (i == 0 || i == bytes.len() - 1) {
                return Err(ContractError::InvalidProjectSlug);
            }
        }
        Ok(())
    }

    /// Validate a project description (non-empty, within byte limit).
    pub fn validate_description(desc: &String) -> Result<(), ContractError> {
        let bytes = desc.as_bytes();
        if bytes.is_empty() {
            return Err(ContractError::InvalidProjectDesc);
        }
        // Reject whitespace-only descriptions
        let all_ws = bytes.iter().all(|b| b.is_ascii_whitespace());
        if all_ws {
            return Err(ContractError::InvalidProjectDesc);
        }
        if bytes.len() > MAX_DESCRIPTION_LEN {
            return Err(ContractError::ProjectDescTooLong);
        }
        Ok(())
    }

    /// Validate a category field (non-empty, within byte limit, non-whitespace-only).
    pub fn validate_category_field(cat: &String) -> Result<(), ContractError> {
        let bytes = cat.as_bytes();
        if bytes.is_empty() {
            return Err(ContractError::InvalidCategory);
        }
        let all_ws = bytes.iter().all(|b| b.is_ascii_whitespace());
        if all_ws {
            return Err(ContractError::InvalidCategory);
        }
        if bytes.len() > MAX_CATEGORY_LEN {
            return Err(ContractError::InvalidCategory);
        }
        Ok(())
    }

    /// Validate a website URL (must start with `http://` or `https://`, within byte limit).
    pub fn validate_website(url: &String) -> Result<(), ContractError> {
        let bytes = url.as_bytes();
        if bytes.is_empty() {
            return Err(ContractError::InvalidWebsite);
        }
        if bytes.len() > MAX_WEBSITE_LEN {
            return Err(ContractError::InvalidWebsite);
        }
        let s = url.as_str();
        if !s.starts_with("http://") && !s.starts_with("https://") {
            return Err(ContractError::InvalidWebsite);
        }
        Ok(())
    }

    /// Validate a license identifier (SPDX-style: alphanumeric, `-`, `.`, `+`).
    pub fn validate_license(license: &String) -> Result<(), ContractError> {
        let bytes = license.as_bytes();
        if bytes.is_empty() {
            return Err(ContractError::InvalidProjectData);
        }
        if bytes.len() > MAX_LICENSE_LEN {
            return Err(ContractError::InvalidProjectData);
        }
        for &b in bytes.iter() {
            if !b.is_ascii_alphanumeric() && b != b'-' && b != b'.' && b != b'+' {
                return Err(ContractError::InvalidProjectData);
            }
        }
        Ok(())
    }

    /// Validate a logo CID.
    pub fn validate_logo_cid(cid: &String) -> Result<(), ContractError> {
        if cid.is_empty() || !Self::is_valid_ipfs_cid(cid) {
            return Err(ContractError::InvalidLogoCid);
        }
        Ok(())
    }

    /// Validate a metadata CID.
    pub fn validate_metadata_cid(cid: &String) -> Result<(), ContractError> {
        if cid.is_empty() || !Self::is_valid_ipfs_cid(cid) {
            return Err(ContractError::InvalidMetaCid);
        }
        Ok(())
    }

    /// Validate a security contact value (non-empty, within byte limit).
    pub fn validate_security_contact(contact: &String) -> Result<(), ContractError> {
        let bytes = contact.as_bytes();
        if bytes.is_empty() || bytes.len() > MAX_SECURITY_CONTACT_LEN {
            return Err(ContractError::SecurityContactInvalid);
        }
        Ok(())
    }

    /// Validate the tags list (each tag must be non-empty ASCII alphanumeric/hyphen/underscore).
    pub fn validate_tags(tags: &soroban_sdk::Vec<String>) -> Result<(), ContractError> {
        for i in 0..tags.len() {
            if let Some(tag) = tags.get(i) {
                let bytes = tag.as_bytes();
                if bytes.is_empty() {
                    return Err(ContractError::InvalidTags);
                }
                for &b in bytes.iter() {
                    if !b.is_ascii_alphanumeric() && b != b'-' && b != b'_' {
                        return Err(ContractError::InvalidTags);
                    }
                }
            }
        }
        Ok(())
    }

    /// Validate the social links list (each link must be a valid URL).
    pub fn validate_social_links(links: &soroban_sdk::Vec<String>) -> Result<(), ContractError> {
        for i in 0..links.len() {
            if let Some(link) = links.get(i) {
                Self::validate_website(&link)?;
            }
        }
        Ok(())
    }

    // ────────────────────────────────────────────────────────────────────
    // CID helpers
    // ────────────────────────────────────────────────────────────────────

    /// Returns `true` if the string is a plausible IPFS CID (v0 or v1).
    ///
    /// - CIDv0: starts with `Qm`, total length 46.
    /// - CIDv1: starts with `b`, length 46–128.
    pub fn is_valid_ipfs_cid(cid: &String) -> bool {
        let bytes = cid.as_bytes();
        let len = bytes.len();
        if len < 46 || len > MAX_CID_LEN {
            return false;
        }
        if bytes[0] == b'Q' && bytes[1] == b'm' {
            // CIDv0
            len == 46
        } else if bytes[0] == b'b' {
            // CIDv1
            true
        } else {
            false
        }
    }

    // ────────────────────────────────────────────────────────────────────
    // Verified-project field freeze guard
    // ────────────────────────────────────────────────────────────────────

    /// For verified projects, certain identity-critical fields are frozen.
    ///
    /// Frozen fields: `slug`, `category`, `logo_cid`.
    /// `name` is NOT frozen (rename triggers verification reset instead).
    /// `metadata_cid` is NOT frozen.
    pub fn check_frozen_fields(
        is_verified: bool,
        _name_differs: bool,
        slug_differs: bool,
        category_differs: bool,
        logo_differs: bool,
        _meta_differs: bool,
    ) -> Result<(), ContractError> {
        if is_verified && (slug_differs || category_differs || logo_differs) {
            return Err(ContractError::VerifiedFieldFrozen);
        }
        Ok(())
    }
}
