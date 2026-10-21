//! Review input validation helpers (CID, rating bounds, ownership checks).

use crate::constants::{
    MAX_CID_LEN, MAX_EVIDENCE_LINKS_PER_REVIEW, MAX_EVIDENCE_LINK_URL_LEN, RATING_MAX, RATING_MIN,
};
use crate::errors::ContractError;
use crate::project_registry::ProjectRegistry;
use crate::types::{EvidenceLink, Project};
use crate::utils::Utils;
use soroban_sdk::{Address, Env, String};

/// Validation helpers for review create/update paths.
pub struct ReviewValidation;

impl ReviewValidation {
    pub fn validate_review_cid(cid: &String) -> Result<(), ContractError> {
        if !Utils::is_valid_ipfs_cid(cid) || cid.len() as usize > MAX_CID_LEN {
            return Err(ContractError::InvalidProjectData);
        }
        Ok(())
    }

    pub fn validate_rating(rating: u32) -> Result<(), ContractError> {
        if !(RATING_MIN..=RATING_MAX).contains(&rating) {
            return Err(ContractError::InvalidRating);
        }
        Ok(())
    }

    /// Project owners cannot review their own project.
    pub fn ensure_not_owner(project: &Project, reviewer: &Address) -> Result<(), ContractError> {
        if project.owner == *reviewer {
            return Err(ContractError::OwnerCannotReview);
        }
        Ok(())
    }

    /// Nobody who controls a project may review it (issue #478).
    ///
    /// Extends the owner check to maintainers. A maintainer is added by the
    /// owner and can edit the project, so allowing maintainers to review is the
    /// same rating-inflation hole as allowing the owner — just one call away.
    ///
    /// Applied to both the create and the update path. Update matters because
    /// control can be acquired *after* an honest review: a reviewer who is
    /// later made maintainer, or who receives ownership, could otherwise keep
    /// editing their rating from the inside.
    pub fn ensure_can_review(
        env: &Env,
        project_id: u64,
        project: &Project,
        reviewer: &Address,
    ) -> Result<(), ContractError> {
        Self::ensure_not_owner(project, reviewer)?;

        if ProjectRegistry::is_maintainer(env, project_id, reviewer) {
            return Err(ContractError::OwnerCannotReview);
        }

        Ok(())
    }

    /// Validate a single evidence link URL.
    ///
    /// Rules:
    /// - Non-empty (byte length >= 1).
    /// - Byte length <= `MAX_EVIDENCE_LINK_URL_LEN` (512).
    /// - Starts with `https://` or `http://`.
    ///
    /// Requirements: 2.1, 2.2, 2.3
    pub fn validate_evidence_link_url(url: &String) -> Result<(), ContractError> {
        let len = url.len() as usize;
        if len == 0 {
            return Err(ContractError::InvalidEvidenceLink);
        }
        if len > MAX_EVIDENCE_LINK_URL_LEN {
            return Err(ContractError::EvidenceLinkTooLong);
        }
        // Copy bytes into a stack buffer for prefix inspection.
        let mut buf = [0u8; MAX_EVIDENCE_LINK_URL_LEN];
        url.copy_into_slice(&mut buf[..len]);
        let slice = &buf[..len];
        if !slice.starts_with(b"https://") && !slice.starts_with(b"http://") {
            return Err(ContractError::InvalidEvidenceLink);
        }
        Ok(())
    }

    /// Validate the full list of evidence links for a review submission or update.
    ///
    /// - Rejects if `links.len() as u32 > MAX_EVIDENCE_LINKS_PER_REVIEW`.
    /// - Calls `validate_evidence_link_url` on each link's URL and propagates any error.
    ///
    /// Requirements: 1.3, 3.4
    pub fn validate_evidence_links(
        links: &soroban_sdk::Vec<EvidenceLink>,
    ) -> Result<(), ContractError> {
        if links.len() as u32 > MAX_EVIDENCE_LINKS_PER_REVIEW {
            return Err(ContractError::TooManyEvidenceLinks);
        }
        for i in 0..links.len() {
            let link = links.get(i).unwrap();
            Self::validate_evidence_link_url(&link.url)?;
        }
        Ok(())
    }
}
