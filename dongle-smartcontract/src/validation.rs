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
use soroban_sdk::Env;

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

    Ok(())
}
