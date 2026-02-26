#![allow(dead_code)]
//! Contract limits and validation constants. Kept in one place for easy future updates.

/// Maximum number of projects a single user (address) can register. Prevents abuse.
#[allow(dead_code)]
pub const MAX_PROJECTS_PER_USER: u32 = 50;

/// Minimum length for name, description, category (must be non-empty after trim in validation).
#[allow(dead_code)]
pub const MIN_STRING_LEN: usize = 1;

/// Maximum length for project name.
#[allow(dead_code)]
pub const MAX_NAME_LEN: usize = 128;

/// Maximum length for project description.
#[allow(dead_code)]
pub const MAX_DESCRIPTION_LEN: usize = 2048;

/// Maximum length for category.
#[allow(dead_code)]
pub const MAX_CATEGORY_LEN: usize = 64;

/// Maximum length for website URL.
#[allow(dead_code)]
pub const MAX_WEBSITE_LEN: usize = 256;

/// Maximum length for any CID (logo, metadata, comment, evidence).
#[allow(dead_code)]
pub const MAX_CID_LEN: usize = 128;

/// Valid rating range (inclusive). Reviews must be in [RATING_MIN, RATING_MAX]. u32 for Soroban Val.
#[allow(dead_code)]
pub const RATING_MIN: u32 = 1;
#[allow(dead_code)]
pub const RATING_MAX: u32 = 5;
