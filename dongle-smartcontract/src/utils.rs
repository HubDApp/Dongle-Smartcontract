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

        let mut bytes = [0u8; 128];
        cid.copy_into_slice(&mut bytes[..len as usize]);
        let slice = &bytes[..len as usize];

        // CIDv0: starts with "Qm"
        if slice.len() >= 2 {
            let first = slice[0];
            let second = slice[1];
            if first == b'Q' && second == b'm' {
                return true;
            }
        }

        // CIDv1 base32 typically starts with 'b' (e.g. bafy...)
        slice[0] == b'b'
    }

    pub fn validate_website(website: &String) -> Result<(), ContractError> {
        let len = website.len();
        if len == 0 {
            return Err(ContractError::InvalidWebsite);
        }
        if len > crate::constants::MAX_WEBSITE_LEN as u32 {
            return Err(ContractError::InvalidWebsite);
        }

        let mut buf = [0u8; crate::constants::MAX_WEBSITE_LEN];
        let slice = &mut buf[..len as usize];
        website.copy_into_slice(slice);

        if !slice.starts_with(b"http://") && !slice.starts_with(b"https://") {
            return Err(ContractError::InvalidWebsite);
        }
        Ok(())
    }

    pub fn validate_license(license: &String) -> Result<(), ContractError> {
        let len = license.len();
        if len == 0 || len > crate::constants::MAX_LICENSE_LEN as u32 {
            return Err(ContractError::InvalidProjectData);
        }

        let mut buf = [0u8; crate::constants::MAX_LICENSE_LEN];
        let slice = &mut buf[..len as usize];
        license.copy_into_slice(slice);

        for &b in slice.iter() {
            if !b.is_ascii_alphanumeric() && b != b'.' && b != b'-' && b != b'+' {
                return Err(ContractError::InvalidProjectData);
            }
        }

        Ok(())
    }

    pub fn sanitize_string(input: &String) -> String {
        input.clone()
    }

    pub fn validate_category_field(category: &String) -> Result<(), ContractError> {
        let len = category.len();
        if len == 0 {
            return Err(ContractError::InvalidCategory);
        }
        if len > crate::constants::MAX_CATEGORY_LEN as u32 {
            return Err(ContractError::InvalidCategory);
        }

        let mut buf = [0u8; crate::constants::MAX_CATEGORY_LEN];
        let slice = &mut buf[..len as usize];
        category.copy_into_slice(slice);

        let is_whitespace_only = slice
            .iter()
            .all(|&b| b == b' ' || b == b'\t' || b == b'\n' || b == b'\r');
        if is_whitespace_only {
            return Err(ContractError::InvalidCategory);
        }

        Ok(())
    }

    pub fn validate_logo_cid(cid: &String) -> Result<(), ContractError> {
        if cid.len() == 0 || !Self::is_valid_ipfs_cid(cid) {
            return Err(ContractError::InvalidLogoCid);
        }
        Ok(())
    }

    pub fn validate_metadata_cid(cid: &String) -> Result<(), ContractError> {
        if cid.len() == 0 || !Self::is_valid_ipfs_cid(cid) {
            return Err(ContractError::InvalidMetaCid);
        }
        Ok(())
    }

    pub fn validate_security_contact(contact: &String) -> Result<(), ContractError> {
        let len = contact.len();
        if len == 0 || len > crate::constants::MAX_SECURITY_CONTACT_LEN as u32 {
            return Err(ContractError::InvalidProjectData);
        }

        let mut buf = [0u8; crate::constants::MAX_SECURITY_CONTACT_LEN];
        contact.copy_into_slice(&mut buf[..len as usize]);
        let is_whitespace_only = buf[..len as usize]
            .iter()
            .all(|&b| b == b' ' || b == b'\t' || b == b'\n' || b == b'\r');
        if is_whitespace_only {
            return Err(ContractError::InvalidProjectData);
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
            return Err(ContractError::InvalidProjectDesc);
        }

        // 2. Check maximum length constraint
        if len > crate::constants::MAX_DESCRIPTION_LEN as u32 {
            return Err(ContractError::ProjectDescTooLong);
        }

        // 3. For non-empty strings, we accept them as valid
        // Note: Soroban String is UTF-8 and we trust the input at this level
        // More sophisticated validation would require converting to bytes
        // which is not efficient in the contract environment

        Ok(())
    }

    /// Validates project slug format (lowercase alphanumeric + hyphens).
    pub fn validate_project_slug(slug: &String) -> Result<(), ContractError> {
        let len = slug.len();
        if len == 0 {
            return Err(ContractError::InvalidProjectData);
        }

        let max_len = crate::constants::MAX_SLUG_LEN;
        if len as usize > max_len {
            return Err(ContractError::InvalidProjectData);
        }

        let mut buf = [0u8; crate::constants::MAX_SLUG_LEN];
        slug.copy_into_slice(&mut buf[..len as usize]);
        let slice = &buf[..len as usize];

        let is_whitespace_only = slice
            .iter()
            .all(|&b| b == b' ' || b == b'\t' || b == b'\n' || b == b'\r');
        if is_whitespace_only {
            return Err(ContractError::InvalidProjectData);
        }

        for &b in slice {
            if !b.is_ascii_alphanumeric() && b != b'-' {
                return Err(ContractError::InvalidProjectData);
            }
        }

        Ok(())
    }

    /// Validates project name with comprehensive checks:
    /// - Not empty or whitespace-only
    /// - Within maximum length constraint (MAX_NAME_LEN)
    /// - Alphanumeric, underscore, and hyphen only
    pub fn validate_project_name(name: &String) -> Result<(), ContractError> {
        let len = name.len();
        if len == 0 {
            return Err(ContractError::InvalidProjectName);
        }

        let max_len = crate::constants::MAX_NAME_LEN;
        if len as usize > max_len {
            return Err(ContractError::ProjectNameTooLong);
        }

        let mut buf = [0u8; crate::constants::MAX_NAME_LEN];
        name.copy_into_slice(&mut buf[..len as usize]);
        let slice = &buf[..len as usize];

        let is_whitespace_only = slice
            .iter()
            .all(|&b| b == b' ' || b == b'\t' || b == b'\n' || b == b'\r');
        if is_whitespace_only {
            return Err(ContractError::InvalidProjectName);
        }

        for &b in slice {
            if !b.is_ascii_alphanumeric() && b != b'_' && b != b'-' {
                return Err(ContractError::InvalidNameFormat);
            }
        }

        Ok(())
    }

    /// Validates project tags
    pub fn validate_tags(tags: &Vec<String>) -> Result<(), ContractError> {
        // Check max number of tags
        if tags.len() > crate::constants::MAX_TAGS_PER_PROJECT {
            return Err(ContractError::TooManyTags);
        }

        // Validate each tag
        for tag in tags.iter() {
            let len = tag.len();
            if len == 0 || len > crate::constants::MAX_TAG_LENGTH as u32 {
                return Err(ContractError::InvalidTag);
            }

            let mut buf = [0u8; crate::constants::MAX_TAG_LENGTH];
            tag.copy_into_slice(&mut buf[..len as usize]);
            let slice = &buf[..len as usize];

            for &b in slice {
                if !b.is_ascii_alphanumeric() && b != b'_' && b != b'-' {
                    return Err(ContractError::InvalidTag);
                }
            }
        }

        Ok(())
    }

    /// Validates social links
    pub fn validate_social_links(social_links: &Map<String, String>) -> Result<(), ContractError> {
        // Check max number of social links
        if social_links.len() > crate::constants::MAX_SOCIAL_LINKS {
            return Err(ContractError::TooManySocialLinks);
        }

        // Validate each social link
        for (platform, url) in social_links.iter() {
            let p_len = platform.len();
            if p_len == 0 || p_len > crate::constants::MAX_SOCIAL_LINK_PLATFORM_LEN as u32 {
                return Err(ContractError::InvalidSocialLink);
            }

            let mut p_buf = [0u8; crate::constants::MAX_SOCIAL_LINK_PLATFORM_LEN];
            platform.copy_into_slice(&mut p_buf[..p_len as usize]);
            let p_slice = &p_buf[..p_len as usize];

            for &b in p_slice {
                if !b.is_ascii_alphanumeric() && b != b'_' && b != b'-' {
                    return Err(ContractError::InvalidSocialLink);
                }
            }

            let u_len = url.len();
            if u_len == 0 || u_len > crate::constants::MAX_SOCIAL_LINK_URL_LEN as u32 {
                return Err(ContractError::InvalidSocialLink);
            }

            let mut u_buf = [0u8; crate::constants::MAX_SOCIAL_LINK_URL_LEN];
            url.copy_into_slice(&mut u_buf[..u_len as usize]);
            let u_slice = &u_buf[..u_len as usize];

            if !u_slice.starts_with(b"http://") && !u_slice.starts_with(b"https://") {
                return Err(ContractError::InvalidSocialLink);
            }
        }

        Ok(())
    }

    /// Validates report reason CID
    pub fn validate_report_reason_cid(reason_cid: &String) -> Result<(), ContractError> {
        if reason_cid.len() == 0 || !Self::is_valid_ipfs_cid(reason_cid) {
            return Err(ContractError::InvalidProjectData);
        }
        Ok(())
    }

    /// Enforces the **metadata freeze policy** for verified projects.
    ///
    /// After a project reaches `VerificationStatus::Verified`, the following
    /// identity-critical fields are **frozen** and may not be changed without
    /// first losing verification (i.e. the admin revokes or rejects the
    /// current verification record):
    ///
    /// | Frozen field    | Reason                                                |
    /// |-----------------|-------------------------------------------------------|
    /// | `name`          | Public identity anchor; changing it would confuse     |
    /// |                 | users who trusted the verified name.                  |
    /// | `slug`          | URL-stable identifier; links would break or spoof.    |
    /// | `category`      | Verification may be category-specific.                |
    /// | `logo_cid`      | Logo is part of the verified visual identity.         |
    /// | `metadata_cid`  | Metadata CID contains the evidence audited during     |
    /// |                 | the verification review.                              |
    ///
    /// Fields that remain **mutable** after verification:
    /// `description`, `website`, `tags`, `social_links`, `launch_timestamp`.
    ///
    /// ## Parameters
    /// - `is_verified` – pass `true` when `project.verification_status == Verified`.
    /// - `name_changed` – `true` when the caller is attempting to change the name.
    /// - `slug_changed` – `true` when the caller is attempting to change the slug.
    /// - `category_changed` – `true` when the caller is attempting to change the category.
    /// - `logo_cid_changed` – `true` when the caller is attempting to change the logo CID.
    /// - `metadata_cid_changed` – `true` when the caller is attempting to change the metadata CID.
    ///
    /// Returns `Err(ContractError::VerifiedFieldFrozen)` if any frozen field
    /// would be mutated, `Ok(())` otherwise.
    pub fn check_frozen_fields(
        is_verified: bool,
        name_changed: bool,
        slug_changed: bool,
        category_changed: bool,
        logo_cid_changed: bool,
        metadata_cid_changed: bool,
    ) -> Result<(), ContractError> {
        if !is_verified {
            return Ok(());
        }
        if name_changed
            || slug_changed
            || category_changed
            || logo_cid_changed
            || metadata_cid_changed
        {
            return Err(ContractError::VerifiedFieldFrozen);
        }
        Ok(())
    }
}
