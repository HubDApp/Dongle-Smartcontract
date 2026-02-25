#![allow(dead_code)]
//! Contract limits and validation constants. Kept in one place for easy future updates.

/// Maximum number of projects a single user (address) can register. Prevents abuse.
pub const MAX_PROJECTS_PER_USER: u32 = 50;

/// Minimum length for name, description, category (must be non-empty after trim in validation).
pub const MIN_STRING_LEN: usize = 1;

/// Maximum length for project name.
pub const MAX_NAME_LEN: usize = 128;

/// Maximum length for project description.
pub const MAX_DESCRIPTION_LEN: usize = 2048;

/// Maximum length for category.
pub const MAX_CATEGORY_LEN: usize = 64;

/// Maximum length for website URL.
pub const MAX_WEBSITE_LEN: usize = 256;

/// Maximum length for any CID (logo, metadata, comment, evidence).
pub const MAX_CID_LEN: usize = 128;

/// Valid rating range (inclusive). Reviews must be in [RATING_MIN, RATING_MAX]. u32 for Soroban Val.
pub const RATING_MIN: u32 = 1;
pub const RATING_MAX: u32 = 5;
