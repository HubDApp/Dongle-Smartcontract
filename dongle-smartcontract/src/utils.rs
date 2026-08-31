//! Utility functions and the `Utils` struct used throughout the contract.

use soroban_sdk::{Env, String, Vec};

use crate::constants::{
    MAX_CATEGORY_LEN, MAX_CID_LEN, MAX_DESCRIPTION_LEN, MAX_LICENSE_LEN, MAX_NAME_LEN,
    MAX_SECURITY_CONTACT_LEN, MAX_SLUG_LEN, MAX_SOCIAL_LINK_PLATFORM_LEN, MAX_TAGS_PER_PROJECT,
    MAX_TAG_LENGTH, MAX_WEBSITE_LEN, MIN_CID_LEN,
};
use crate::errors::ContractError;

/// Utility struct — all methods are associated functions (no instance needed).
pub struct Utils;

impl Utils {
    /// Return whether exactly one optional value is present.
    pub fn exactly_one_some<T, I>(values: I) -> bool
    where
        I: IntoIterator<Item = Option<T>>,
    {
        values.into_iter().filter(Option::is_some).count() == 1
    }

    // ────────────────────────────────────────────────────────────────────
    // Vec helpers
    // ────────────────────────────────────────────────────────────────────

    /// Push `item` into `vec` only if it is not already present.
    pub fn add_unique_to_vec<
        T: PartialEq
            + Clone
            + soroban_sdk::TryFromVal<soroban_sdk::Env, soroban_sdk::Val>
            + soroban_sdk::IntoVal<soroban_sdk::Env, soroban_sdk::Val>,
    >(
        vec: &mut Vec<T>,
        item: &T,
    ) -> bool {
        for i in 0..vec.len() {
            if let Some(existing) = vec.get(i) {
                if &existing == item {
                    return false;
                }
            }
        }
        vec.push_back(item.clone());
        true
    }

    /// Return a new Vec containing all items from `vec` except those equal to `item`.
    pub fn remove_item_from_vec<
        T: PartialEq
            + Clone
            + soroban_sdk::TryFromVal<soroban_sdk::Env, soroban_sdk::Val>
            + soroban_sdk::IntoVal<soroban_sdk::Env, soroban_sdk::Val>,
    >(
        env: &Env,
        vec: &Vec<T>,
        item: &T,
    ) -> Vec<T> {
        let mut result = Vec::new(env);
        for i in 0..vec.len() {
            if let Some(v) = vec.get(i) {
                if &v != item {
                    result.push_back(v);
                }
            }
        }
        result
    }

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
            return String::from_str(env, "");
        }

        // Allocate a source buffer and an output buffer of the same size
        // (normalization can only shrink or preserve length).
        let max = if len > 64 { 64 } else { len }; // MAX_NAME_LEN is 50, safe upper bound
        let mut src = [0u8; 64];
        let mut out = [0u8; 64];
        name.copy_into_slice(&mut src[..max]);

        let mut out_len: usize = 0;
        let mut last_was_space = true; // treat start as "space" to strip leading

        for &b in src.iter().take(max) {
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
                if !last_was_space && out_len < max {
                    out[out_len] = b' ';
                    out_len += 1;
                }
                last_was_space = true;
            } else {
                if out_len < max {
                    out[out_len] = normalized;
                    out_len += 1;
                }
                last_was_space = false;
            }
        }

        // Trim trailing space
        while out_len > 0 && out[out_len - 1] == b' ' {
            out_len -= 1;
        }

        // Convert back to a Soroban String
        // SAFETY: all bytes are valid ASCII (subset of UTF-8).
        let s = core::str::from_utf8(&out[..out_len]).unwrap_or("");
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
    /// - Lowercase alphanumeric plus `-` or `_` only.
    /// - No leading or trailing `-`.
    /// - Uppercase is rejected so the canonical storage key is always lowercase.
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
            if b.is_ascii_uppercase() {
                return Err(ContractError::InvalidProjectSlug);
            }
            if !b.is_ascii_lowercase() && !b.is_ascii_digit() && b != b'-' && b != b'_' {
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

    /// Validate a website URL.
    ///
    /// Rules:
    /// - Non-empty, at most `MAX_WEBSITE_LEN` bytes.
    /// - Must use the `https://` scheme only. Plain `http://` is rejected to
    ///   enforce encrypted transport on all project websites.
    /// - Must have a non-empty host component after the `https://` prefix
    ///   (i.e. the URL cannot be just `https://`).
    pub fn validate_website(url: &String) -> Result<(), ContractError> {
        let len = url.len() as usize;
        if len == 0 || len > MAX_WEBSITE_LEN {
            return Err(ContractError::InvalidInput);
        }

        let mut buf = [0u8; MAX_WEBSITE_LEN];
        url.copy_into_slice(&mut buf[..len]);

        // Only https:// is accepted; http:// and any other scheme are rejected.
        const SCHEME: &[u8] = b"https://";
        if !buf[..len].starts_with(SCHEME) {
            return Err(ContractError::InvalidInput);
        }

        // There must be at least one character after the scheme — a bare
        // `https://` with no host is not a useful or valid URL.
        if len <= SCHEME.len() {
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
    ///
    /// Uses the strict charset validator (base58btc for CIDv0, base32 for CIDv1).
    pub fn validate_logo_cid(cid: &String) -> Result<(), ContractError> {
        if cid.is_empty() || !Self::is_valid_ipfs_cid_strict(cid) {
            return Err(ContractError::InvalidCid);
        }
        Ok(())
    }

    /// Validate a metadata CID.
    ///
    /// Uses the strict charset validator (base58btc for CIDv0, base32 for CIDv1).
    pub fn validate_metadata_cid(cid: &String) -> Result<(), ContractError> {
        if cid.is_empty() || !Self::is_valid_ipfs_cid_strict(cid) {
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

    /// Validate the tags list.
    ///
    /// Rules:
    /// - At most `MAX_TAGS_PER_PROJECT` tags.
    /// - Each tag is non-empty, at most `MAX_TAG_LENGTH` bytes, and ASCII
    ///   alphanumeric / hyphen / underscore only.
    /// - Values must be unique after ASCII-lowercase normalization
    ///   (e.g. `DeFi` and `defi` are duplicates).
    pub fn validate_tags(tags: &Vec<String>) -> Result<(), ContractError> {
        if tags.len() > MAX_TAGS_PER_PROJECT {
            return Err(ContractError::InvalidTags);
        }

        for i in 0..tags.len() {
            if let Some(tag) = tags.get(i) {
                Self::validate_single_tag(&tag)?;
                for j in 0..i {
                    if let Some(prev) = tags.get(j) {
                        if Self::tags_equal_normalized(&tag, &prev) {
                            return Err(ContractError::InvalidTags);
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn validate_single_tag(tag: &String) -> Result<(), ContractError> {
        let len = tag.len() as usize;
        if len == 0 || len > MAX_TAG_LENGTH {
            return Err(ContractError::InvalidTags);
        }

        let mut buf = [0u8; MAX_TAG_LENGTH];
        tag.copy_into_slice(&mut buf[..len]);
        for &b in buf[..len].iter() {
            if !b.is_ascii_alphanumeric() && b != b'-' && b != b'_' {
                return Err(ContractError::InvalidTags);
            }
        }
        Ok(())
    }

    /// Case-insensitive equality after ASCII-lowercase normalization.
    /// Callers must already have validated each tag length is `<= MAX_TAG_LENGTH`.
    fn tags_equal_normalized(a: &String, b: &String) -> bool {
        let a_len = a.len() as usize;
        let b_len = b.len() as usize;
        if a_len != b_len {
            return false;
        }

        let mut a_buf = [0u8; MAX_TAG_LENGTH];
        let mut b_buf = [0u8; MAX_TAG_LENGTH];
        a.copy_into_slice(&mut a_buf[..a_len]);
        b.copy_into_slice(&mut b_buf[..b_len]);

        for i in 0..a_len {
            if a_buf[i].to_ascii_lowercase() != b_buf[i].to_ascii_lowercase() {
                return false;
            }
        }
        true
    }

    /// Validate the social links map (each value must be a valid URL).
    pub fn validate_social_links(
        links: &soroban_sdk::Map<String, String>,
    ) -> Result<(), ContractError> {
        let keys = links.keys();
        for i in 0..keys.len() {
            if let Some(key) = keys.get(i) {
                let key_len = key.len() as usize;
                if key_len == 0 || key_len > MAX_SOCIAL_LINK_PLATFORM_LEN {
                    return Err(ContractError::InvalidInput);
                }

                let mut key_buf = [0u8; MAX_SOCIAL_LINK_PLATFORM_LEN];
                key.copy_into_slice(&mut key_buf[..key_len]);
                if key_buf[..key_len]
                    .iter()
                    .any(|&b| !b.is_ascii_alphanumeric() && b != b'-' && b != b'_')
                {
                    return Err(ContractError::InvalidInput);
                }

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

    /// Return `true` if `cid` is a structurally valid IPFS CID.
    ///
    /// This is the **lenient** validator used for evidence CIDs where we only
    /// enforce the prefix and length window but do **not** validate the character
    /// set. Use [`is_valid_ipfs_cid_strict`] for project metadata fields (logo,
    /// metadata CID) where full structural validation is required.
    ///
    /// # CIDv0 (`Qm…`)
    /// - Length in `[MIN_CID_LEN, MAX_CID_LEN]`.
    /// - First two bytes must be `Q` and `m`.
    ///
    /// # CIDv1 (`b…`)
    /// - Length in `[MIN_CID_LEN, MAX_CID_LEN]`.
    /// - First byte must be `b`.
    pub fn is_valid_ipfs_cid(cid: &String) -> bool {
        let len = cid.len() as usize;
        if !(MIN_CID_LEN..=MAX_CID_LEN).contains(&len) {
            return false;
        }

        let mut buf = [0u8; MAX_CID_LEN];
        cid.copy_into_slice(&mut buf[..len]);

        if buf[0] == b'Q' && buf[1] == b'm' {
            true
        } else if buf[0] == b'b' {
            true
        } else {
            false
        }
    }

    /// Return `true` if `cid` is a structurally valid IPFS CID with full
    /// character-set validation (issue #620).
    ///
    /// Used for project metadata fields (logo CID, metadata CID) where the
    /// stored value is a real IPFS address and must conform to the encoding
    /// alphabet.
    ///
    /// # CIDv0 (`Qm…`)
    /// - Exactly 46 characters (the canonical base58btc SHA2-256 multihash).
    /// - All characters must be in the base58btc alphabet:
    ///   `123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz`
    ///   (digits 1–9, uppercase A–Z excluding I O, lowercase a–z excluding l).
    ///
    /// # CIDv1 (`b…`)
    /// - Length in `[MIN_CID_LEN, MAX_CID_LEN]`.
    /// - After the leading `b` multibase prefix, all remaining characters must
    ///   be in the base32 lower-case alphabet: `a-z` and `2-7`.
    ///
    /// This validation is structural (prefix + charset + length); it does not
    /// decode or verify any embedded multihash digest.
    pub fn is_valid_ipfs_cid_strict(cid: &String) -> bool {
        let len = cid.len() as usize;
        if !(MIN_CID_LEN..=MAX_CID_LEN).contains(&len) {
            return false;
        }

        let mut buf = [0u8; MAX_CID_LEN];
        cid.copy_into_slice(&mut buf[..len]);

        if buf[0] == b'Q' && buf[1] == b'm' {
            // CIDv0: must be exactly 46 chars, base58btc alphabet throughout.
            if len != 46 {
                return false;
            }
            for &b in buf[..len].iter() {
                if !Self::is_base58btc_char(b) {
                    return false;
                }
            }
            true
        } else if buf[0] == b'b' {
            // CIDv1 multibase-base32: leading 'b' + base32 lowercase body.
            for &b in buf[1..len].iter() {
                if !Self::is_base32_lower_char(b) {
                    return false;
                }
            }
            true
        } else {
            false
        }
    }

    /// Return `true` if `b` is in the base58btc alphabet.
    ///
    /// Base58btc omits `0` (zero), `O` (uppercase-o), `I` (uppercase-i), and
    /// `l` (lowercase-L) from the standard alphanumeric set.
    #[inline]
    fn is_base58btc_char(b: u8) -> bool {
        matches!(b,
            b'1'..=b'9'
            | b'A'..=b'H'   // A–H (skips I)
            | b'J'..=b'N'   // J–N (skips O)
            | b'P'..=b'Z'   // P–Z
            | b'a'..=b'k'   // a–k (skips l)
            | b'm'..=b'z'   // m–z
        )
    }

    /// Return `true` if `b` is in the RFC 4648 base32 lowercase alphabet.
    ///
    /// Base32 lowercase uses `a-z` and `2-7`.  Padding (`=`) is intentionally
    /// excluded because CIDv1 strings do not include padding characters.
    #[inline]
    fn is_base32_lower_char(b: u8) -> bool {
        matches!(b, b'a'..=b'z' | b'2'..=b'7')
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
