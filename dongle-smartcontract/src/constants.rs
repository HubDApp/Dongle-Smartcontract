#![allow(dead_code)]
//! Contract limits and validation constants. Kept in one place for easy future updates.

/// Maximum number of projects a single user (address) can register. Prevents abuse.
#[allow(dead_code)]
pub const MAX_PROJECTS_PER_USER: u32 = 50;

/// Minimum length for name, description, category (must be non-empty after trim in validation).
#[allow(dead_code)]
pub const MIN_STRING_LEN: usize = 1;

/// Maximum length for project name.
pub const MAX_NAME_LEN: usize = 50;

/// Maximum length for project slug.
pub const MAX_SLUG_LEN: usize = 64;

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

// ── TTL (Time To Live) Constants ──────────────────────────────────────────

/// TTL for critical contract data (admin list, fee config, treasury).
/// Set to ~30 days (30 * 24 * 60 * 60 / 5 seconds per ledger = 518,400 ledgers).
/// This data should persist long-term and be extended regularly.
pub const LEDGER_THRESHOLD_CRITICAL: u32 = 518_400;

/// TTL for project data (projects, project stats, project counts).
/// Set to ~90 days (90 * 24 * 60 * 60 / 5 = 1,555,200 ledgers).
/// Projects are core entities and should have long persistence.
pub const LEDGER_THRESHOLD_PROJECT: u32 = 1_555_200;

/// TTL for review data (reviews, review stats).
/// Set to ~60 days (60 * 24 * 60 * 60 / 5 = 1,036,800 ledgers).
/// Reviews are important but can be archived if inactive.
pub const LEDGER_THRESHOLD_REVIEW: u32 = 1_036_800;

/// TTL for verification data (verification records, fee payments).
/// Set to ~45 days (45 * 24 * 60 * 60 / 5 = 777,600 ledgers).
/// Verification data is moderately important.
pub const LEDGER_THRESHOLD_VERIFICATION: u32 = 777_600;

/// TTL for user-related data (owner projects, user reviews).
/// Set to ~60 days (60 * 24 * 60 * 60 / 5 = 1,036,800 ledgers).
/// User data should persist reasonably long.
pub const LEDGER_THRESHOLD_USER: u32 = 1_036_800;

/// TTL bump amount - how much to extend when bumping.
/// Set to the same as the threshold to maintain consistent lifetime.
pub const LEDGER_BUMP_CRITICAL: u32 = LEDGER_THRESHOLD_CRITICAL;
pub const LEDGER_BUMP_PROJECT: u32 = LEDGER_THRESHOLD_PROJECT;
pub const LEDGER_BUMP_REVIEW: u32 = LEDGER_THRESHOLD_REVIEW;
pub const LEDGER_BUMP_VERIFICATION: u32 = LEDGER_THRESHOLD_VERIFICATION;
pub const LEDGER_BUMP_USER: u32 = LEDGER_THRESHOLD_USER;
