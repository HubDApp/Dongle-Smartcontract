//! Utility functions and the `Utils` struct used throughout the contract.

use soroban_sdk::{Address, Env, String, Vec};

use crate::constants::{
    MAX_CATEGORY_LEN, MAX_CID_LEN, MAX_DESCRIPTION_LEN, MAX_LICENSE_LEN, MAX_NAME_LEN,
    MAX_SECURITY_CONTACT_LEN, MAX_SLUG_LEN, MAX_WEBSITE_LEN,
};
use crate::errors::ContractError;
use crate::types::Project;

/// Check if address is a maintainer of the project (free function).
pub fn is_maintainer(_env: &Env, project: &Project, address: &Address) -> bool {
use soroban_sdk::{Address, Env, Map, String, Vec};

/// Check if address is a maintainer of the project (free function).
pub fn is_maintainer(project: &Project, address: &Address) -> bool {
    if let Some(ref maintainers) = project.maintainers {
        maintainers.contains(address)
    } else {
        false
    }
}

#[allow(dead_code)]
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
    pub fn normalize_project_name(env: &Env, name: &String) -> String {
        let len = name.len() as usize;
        if len == 0 {
            return String::from_str(env, "");
        }

        let mut raw_buf = [0u8; 128];
        let actual_len = core::cmp::min(len, raw_buf.len());
        name.copy_into_slice(&mut raw_buf[..actual_len]);

        let mut out_buf = [0u8; 128];
        let mut out_len: usize = 0;
        let mut last_was_space = true; // treat start as "space" to strip leading

        for i in 0..actual_len {
            let b = raw_buf[i];
            let normalized = if b.is_ascii_uppercase() {
                b + 32
            } else if b == b' ' || b == b'\t' || b == b'\n' || b == b'\r' {
                b' '
            } else if b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_' || b == b'-' {
                b
            } else {
                b' '
            };

            if normalized == b' ' {
                if !last_was_space && out_len < out_buf.len() {
                    out_buf[out_len] = b' ';
                    out_len += 1;
                }
                last_was_space = true;
            } else {
                if out_len < out_buf.len() {
                    out_buf[out_len] = normalized;
                    out_len += 1;
                }
                last_was_space = false;
            }
        }

        // Trim trailing space
        while out_len > 0 && out_buf[out_len - 1] == b' ' {
            out_len -= 1;
        }

        let s = core::str::from_utf8(&out_buf[..out_len]).unwrap_or("");
        String::from_str(env, s)
    }

    /// Convert a Soroban `String` to lowercase (ASCII only).
    /// Used by the reserved-name checker and other case-insensitive comparisons.
    pub fn to_lowercase(env: &Env, s: &String) -> String {
        let len = s.len() as usize;
        if len == 0 {
            return s.clone();
        }
        let mut buf = [0u8; 256];
        let cap = core::cmp::min(len, buf.len());
        s.copy_into_slice(&mut buf[..cap]);
        for i in 0..cap {
            if buf[i].is_ascii_uppercase() {
                buf[i] += 32;
            }
        }
        let res = core::str::from_utf8(&buf[..cap]).unwrap_or("");
        String::from_str(env, res)
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
        let len = name.len() as usize;
        if len == 0 {
            return Err(ContractError::InvalidProjectName);
        }
        if len > MAX_NAME_LEN {
            return Err(ContractError::ProjectNameTooLong);
        }
        let mut buf = [0u8; MAX_NAME_LEN];
        name.copy_into_slice(&mut buf[..len]);
        let mut all_ws = true;
        for &b in &buf[..len] {
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
        let len = slug.len() as usize;
        if len == 0 {
            return Err(ContractError::InvalidProjectSlug);
        }
        if len > MAX_SLUG_LEN {
            return Err(ContractError::InvalidProjectSlugLen);
        }
        let mut buf = [0u8; MAX_SLUG_LEN];
        slug.copy_into_slice(&mut buf[..len]);
        let bytes = &buf[..len];
        for (i, &b) in bytes.iter().enumerate() {
            if !b.is_ascii_alphanumeric() && b != b'-' && b != b'_' {
                return Err(ContractError::InvalidProjectSlug);
            }
            if b == b'-' && (i == 0 || i == len - 1) {
                return Err(ContractError::InvalidProjectSlug);
            }
        }
        Ok(())
    }

    /// Validate a project description (non-empty, within byte limit).
    pub fn validate_description(desc: &String) -> Result<(), ContractError> {
        let len = desc.len() as usize;
        if len == 0 {
            return Err(ContractError::InvalidProjectDesc);
        }
        if len > MAX_DESCRIPTION_LEN {
            return Err(ContractError::ProjectDescTooLong);
        }
        let mut buf = [0u8; 2048];
        let actual_len = core::cmp::min(len, buf.len());
        desc.copy_into_slice(&mut buf[..actual_len]);
        let all_ws = buf[..actual_len].iter().all(|b| b.is_ascii_whitespace());
        if all_ws {
            return Err(ContractError::InvalidProjectDesc);
        }
        Ok(())
    }

    /// Validate a category field (non-empty, within byte limit, non-whitespace-only).
    pub fn validate_category_field(cat: &String) -> Result<(), ContractError> {
        let len = cat.len() as usize;
        if len == 0 {
            return Err(ContractError::InvalidCategory);
        }
        if len > MAX_CATEGORY_LEN {
            return Err(ContractError::InvalidCategory);
        }
        let mut buf = [0u8; MAX_CATEGORY_LEN];
        cat.copy_into_slice(&mut buf[..len]);
        let all_ws = buf[..len].iter().all(|b| b.is_ascii_whitespace());
        if all_ws {
            return Err(ContractError::InvalidCategory);
        }
        Ok(())
    }

    /// Validate a website URL (must start with `http://` or `https://`, within byte limit).
    pub fn validate_website(url: &String) -> Result<(), ContractError> {
        let len = url.len() as usize;
        if len == 0 || len > MAX_WEBSITE_LEN {
            return Err(ContractError::InvalidWebsite);
        }
        let mut buf = [0u8; MAX_WEBSITE_LEN];
        url.copy_into_slice(&mut buf[..len]);
        let bytes = &buf[..len];
        let starts_with_http = bytes.starts_with(b"http://");
        let starts_with_https = bytes.starts_with(b"https://");
        if !starts_with_http && !starts_with_https {
            return Err(ContractError::InvalidWebsite);
        }
        Ok(())
    }

    /// Validate a license identifier (SPDX-style: alphanumeric, `-`, `.`, `+`).
    pub fn validate_license(license: &String) -> Result<(), ContractError> {
        let len = license.len() as usize;
        if len == 0 || len > MAX_LICENSE_LEN {
            return Err(ContractError::InvalidProjectData);
        }
        let mut buf = [0u8; MAX_LICENSE_LEN];
        license.copy_into_slice(&mut buf[..len]);
        for &b in &buf[..len] {
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
        let len = contact.len() as usize;
        if len == 0 || len > MAX_SECURITY_CONTACT_LEN {
            return Err(ContractError::SecurityContactInvalid);
        }
        Ok(())
    }

    /// Validate the tags list (each tag must be non-empty ASCII alphanumeric/hyphen/underscore).
    pub fn validate_tags(tags: &Vec<String>) -> Result<(), ContractError> {
        for i in 0..tags.len() {
            if let Some(tag) = tags.get(i) {
                let len = tag.len() as usize;
                if len == 0 || len > 64 {
                    return Err(ContractError::InvalidTags);
                }
                let mut buf = [0u8; 64];
                tag.copy_into_slice(&mut buf[..len]);
                for &b in &buf[..len] {
                    if !b.is_ascii_alphanumeric() && b != b'-' && b != b'_' {
                        return Err(ContractError::InvalidTags);
                    }
                }
            }
        }
        Ok(())
    }

    /// Validate the social links list (each link must be a valid URL).
    pub fn validate_social_links(links: &Vec<String>) -> Result<(), ContractError> {
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
        let len = cid.len() as usize;
        if len < 46 || len > MAX_CID_LEN {
            return false;
        }
        let mut buf = [0u8; MAX_CID_LEN];
        cid.copy_into_slice(&mut buf[..len]);
        if buf[0] == b'Q' && buf[1] == b'm' {
            // CIDv0
            len == 46
        } else if buf[0] == b'b' {
            // CIDv1
            true
        } else {
            false
        }
    }

    // ────────────────────────────────────────────────────────────────────
    // Report reason CID validation
    // ────────────────────────────────────────────────────────────────────

    /// Validate a report reason CID.
    pub fn validate_report_reason_cid(cid: &String) -> Result<(), ContractError> {
        if cid.is_empty() || !Self::is_valid_ipfs_cid(cid) {
            return Err(ContractError::InvalidCid);
        }
        Ok(())
    }

    // ────────────────────────────────────────────────────────────────────
    // Verified-project field freeze guard
    // ────────────────────────────────────────────────────────────────────

    /// For verified projects, certain identity-critical fields are frozen.
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
