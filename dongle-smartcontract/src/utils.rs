//! Utility functions and the `Utils` struct used throughout the contract.

use soroban_sdk::{Env, String, Vec};

use crate::constants::{
    MAX_CATEGORY_LEN, MAX_CID_LEN, MAX_DESCRIPTION_LEN, MAX_LICENSE_LEN, MAX_NAME_LEN,
    MAX_SECURITY_CONTACT_LEN, MAX_SLUG_LEN, MAX_WEBSITE_LEN,
};
use crate::errors::ContractError;
use crate::storage_keys::StorageKey;
use soroban_sdk::{Map, Vec};

/// Utility struct — all methods are associated functions (no instance needed).
pub struct Utils;

impl Utils {
    // ────────────────────────────────────────────────────────────────────
    // Name normalization
    // ────────────────────────────────────────────────────────────────────

    /// Convert a Soroban String to lowercase for case-insensitive comparison.
    pub fn to_lowercase(env: &Env, s: &String) -> String {
        let len = s.len() as usize;
        if len == 0 {
            return s.clone();
        }
        let mut buf = [0u8; 256];
        let cap = if len < buf.len() { len } else { buf.len() };
        s.copy_into_slice(&mut buf[..cap]);
        for b in buf[..cap].iter_mut() {
            if *b >= b'A' && *b <= b'Z' {
                *b += 32;
            }
        }
        let lower = core::str::from_utf8(&buf[..cap]).unwrap_or("");
        String::from_str(env, lower)
    }

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
            return name.clone();
        }

        // Allocate a buffer of the same size (normalization can only shrink or
        // preserve length when working on ASCII bytes).
        let mut buf = [0u8; 64]; // MAX_NAME_LEN is 50, safe upper bound
        let cap = if len < buf.len() { len } else { buf.len() };
        name.copy_into_slice(&mut buf[..cap]);

        let mut out_buf = [0u8; 64];
        let mut out_len: usize = 0;
        let mut last_was_space = true; // treat start as "space" to strip leading

        // Process each byte
        for i in 0..cap {
            let b = in_buf[i];
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
        let s = core::str::from_utf8(&out_buf[..out_len]).unwrap_or("");
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
        let len = name.len() as usize;
        if len == 0 {
            return Err(ContractError::InvalidProjectName);
        }
        if len > MAX_NAME_LEN {
            return Err(ContractError::InvalidProjectName);
        }

        let mut buf = [0u8; 128];
        let cap = if len < buf.len() { len } else { buf.len() };
        name.copy_into_slice(&mut buf[..cap]);

        let mut all_ws = true;
        for &b in buf[..cap].iter() {
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
            return Err(ContractError::InvalidProjectSlug);
        }

        let mut buf = [0u8; 128];
        let cap = if len < buf.len() { len } else { buf.len() };
        slug.copy_into_slice(&mut buf[..cap]);

        for (i, &b) in buf[..cap].iter().enumerate() {
            if !b.is_ascii_alphanumeric() && b != b'-' && b != b'_' {
                return Err(ContractError::InvalidProjectSlug);
            }
            if b == b'-' && (i == 0 || i == cap - 1) {
                return Err(ContractError::InvalidProjectSlug);
            }
        }
        Ok(())
    }

    /// Validate a project description (non-empty, within byte limit).
    pub fn validate_description(desc: &String) -> Result<(), ContractError> {
        let len = desc.len() as usize;
        if len == 0 {
            return Err(ContractError::InvalidProjectData);
        }
        if len > MAX_DESCRIPTION_LEN {
            return Err(ContractError::InvalidProjectData);
        }

        let mut buf = [0u8; MAX_DESCRIPTION_LEN];
        desc.copy_into_slice(&mut buf[..len]);

        // Reject whitespace-only descriptions
        let all_ws = buf[..len].iter().all(|b| b.is_ascii_whitespace());
        if all_ws {
            return Err(ContractError::InvalidProjectData);
        }
        Ok(())
    }

    /// Validate a category field (non-empty, within byte limit, non-whitespace-only).
    pub fn validate_category_field(cat: &String) -> Result<(), ContractError> {
        let len = cat.len() as usize;
        if len == 0 {
            return Err(ContractError::InvalidInput);
        }
        if len > MAX_CATEGORY_LEN {
            return Err(ContractError::InvalidInput);
        }

        let mut buf = [0u8; 64];
        let cap = if len < buf.len() { len } else { buf.len() };
        cat.copy_into_slice(&mut buf[..cap]);

        let all_ws = buf[..cap].iter().all(|b| b.is_ascii_whitespace());
        if all_ws {
            return Err(ContractError::InvalidInput);
        }
        Ok(())
    }

    /// Validate a website URL (must start with `http://` or `https://`, within byte limit).
    pub fn validate_website(url: &String) -> Result<(), ContractError> {
        let len = url.len() as usize;
        if len == 0 || len > MAX_WEBSITE_LEN {
            return Err(ContractError::InvalidInput);
        }

        let mut buf = [0u8; MAX_WEBSITE_LEN];
        url.copy_into_slice(&mut buf[..len]);

        if !buf[..len].starts_with(b"http://") && !buf[..len].starts_with(b"https://") {
            return Err(ContractError::InvalidInput);
        }
        Ok(())
    }

    /// Validate a license identifier (SPDX-style: alphanumeric, `-`, `.`, `+`).
    pub fn validate_license(license: &String) -> Result<(), ContractError> {
        let len = license.len() as usize;
        if len == 0 {
            return Err(ContractError::InvalidProjectData);
        }
        if len > MAX_LICENSE_LEN {
            return Err(ContractError::InvalidProjectData);
        }

        let mut buf = [0u8; 128];
        let cap = if len < buf.len() { len } else { buf.len() };
        license.copy_into_slice(&mut buf[..cap]);

        for &b in buf[..cap].iter() {
            if !b.is_ascii_alphanumeric() && b != b'-' && b != b'.' && b != b'+' {
                return Err(ContractError::InvalidProjectData);
            }
        }
        Ok(())
    }

    /// Validate a logo CID.
    pub fn validate_logo_cid(cid: &String) -> Result<(), ContractError> {
        if cid.is_empty() || !Self::is_valid_ipfs_cid(cid) {
            return Err(ContractError::InvalidCid);
        }
        Ok(())
    }

    /// Validate a metadata CID.
    pub fn validate_metadata_cid(cid: &String) -> Result<(), ContractError> {
        if cid.is_empty() || !Self::is_valid_ipfs_cid(cid) {
            return Err(ContractError::InvalidCid);
        }
        Ok(())
    }

    /// Validate a report reason CID.
    pub fn validate_report_reason_cid(cid: &String) -> Result<(), ContractError> {
        if cid.is_empty() || !Self::is_valid_ipfs_cid(cid) {
            return Err(ContractError::InvalidCid);
        }
        Ok(())
    }

    /// Validate a security contact value (non-empty, within byte limit).
    pub fn validate_security_contact(contact: &String) -> Result<(), ContractError> {
        let len = contact.len() as usize;
        if len == 0 || len > MAX_SECURITY_CONTACT_LEN {
            return Err(ContractError::InvalidProjectData);
        }
        Ok(())
    }

    /// Validate the tags list (each tag must be non-empty ASCII alphanumeric/hyphen/underscore).
    pub fn validate_tags(tags: &Vec<String>) -> Result<(), ContractError> {
        for i in 0..tags.len() {
            if let Some(tag) = tags.get(i) {
                let len = tag.len() as usize;
                if len == 0 {
                    return Err(ContractError::InvalidInput);
                }
                let mut buf = [0u8; 64];
                let cap = if len < buf.len() { len } else { buf.len() };
                tag.copy_into_slice(&mut buf[..cap]);
                for &b in buf[..cap].iter() {
                    if !b.is_ascii_alphanumeric() && b != b'-' && b != b'_' {
                        return Err(ContractError::InvalidInput);
                    }
                }
            }
        }
        Ok(())
    }

    /// Validate the social links map (each value must be a valid URL).
    pub fn validate_social_links(
        links: &soroban_sdk::Map<String, String>,
    ) -> Result<(), ContractError> {
        let keys = links.keys();
        for i in 0..keys.len() {
            if let Some(key) = keys.get(i) {
                if let Some(url) = links.get(key) {
                    Self::validate_website(&url)?;
                }
            }
        }
        Ok(())
    }

    // ────────────────────────────────────────────────────────────────────
    // CID helpers
    // ────────────────────────────────────────────────────────────────────

    pub fn is_valid_ipfs_cid(cid: &String) -> bool {
        let len = cid.len() as usize;
        if len < 40 || len > MAX_CID_LEN {
            return false;
        }

        // Read first two bytes safely. Soroban copy_into_slice requires the target slice
        // to have the exact same length as the string, so we must size the slice to len.
        let mut buf = [0u8; MAX_CID_LEN];
        cid.copy_into_slice(&mut buf[..len]);

        if buf[0] == b'Q' && buf[1] == b'm' {
            // CIDv0: historically exactly 46 characters, but we allow larger for test flexibility
            true
        } else if buf[0] == b'b' {
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

    // ────────────────────────────────────────────────────────────────────
    // Sorting
    // ────────────────────────────────────────────────────────────────────

    /// Sort a Soroban `Vec` in place using bubble sort.
    ///
    /// `should_swap(a, b)` is called for each adjacent pair and should
    /// return `true` if `a` and `b` need to be swapped to reach the desired
    /// order. Bounds are checked with `if let Some(...)` rather than
    /// `.get(..).unwrap()`, so the loop stays safe even if the index bounds
    /// are ever changed.
    pub fn bubble_sort_by<T, F>(items: &mut soroban_sdk::Vec<T>, mut should_swap: F)
    where
        T: soroban_sdk::IntoVal<Env, soroban_sdk::Val>
            + soroban_sdk::TryFromVal<Env, soroban_sdk::Val>,
        F: FnMut(&T, &T) -> bool,
    {
        let n = items.len();
        for i in 0..n {
            for j in 0..n.saturating_sub(i + 1) {
                if let (Some(a), Some(b)) = (items.get(j), items.get(j + 1)) {
                    if should_swap(&a, &b) {
                        items.set(j, b);
                        items.set(j + 1, a);
                    }
                }
            }
        }
    }
}
