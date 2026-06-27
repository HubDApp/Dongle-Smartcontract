#![allow(dead_code)]
//! Contract limits and validation constants. Kept in one place for easy future updates.

/// Maximum number of projects a single user (address) can register. Prevents abuse.
#[allow(dead_code)]
pub const MAX_PROJECTS_PER_USER: u32 = 50;

// ── Storage index size limits ───────────────────────────────────────────────
// Vec-based indexes are capped on write to avoid unbounded per-user/project growth.
// See STORAGE_INDEXES.md for the full index catalog and pagination strategy.

/// Maximum unique reviewers indexed per project (`ProjectReviews`).
pub const MAX_REVIEWS_PER_PROJECT: u32 = 500;

/// Maximum projects indexed per reviewer (`UserReviews`).
/// Also used as the shared index-capacity error (`MaxProjectsExceeded`) for review indexes.
pub const MAX_REVIEWS_PER_USER: u32 = 200;

/// Maximum items returned per paginated read query across list endpoints.
pub const MAX_PAGE_LIMIT: u32 = 100;

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

/// Project metadata fields whose changes invalidate an existing verification.
pub const MAJOR_METADATA_FIELD_NAME: &str = "name";
pub const MAJOR_METADATA_FIELD_WEBSITE: &str = "website";
pub const MAJOR_METADATA_FIELD_METADATA_CID: &str = "metadata_cid";
pub const MAJOR_METADATA_FIELDS: [&str; 3] = [
    MAJOR_METADATA_FIELD_NAME,
    MAJOR_METADATA_FIELD_WEBSITE,
    MAJOR_METADATA_FIELD_METADATA_CID,
];

/// Minimum project age in seconds before verification can be requested (default: 0 for backward compatibility).
pub const MIN_PROJECT_AGE_SECONDS: u64 = 0;

/// Maximum number of tags per project.
pub const MAX_TAGS_PER_PROJECT: u32 = 10;

/// Maximum length for a single tag.
pub const MAX_TAG_LENGTH: usize = 32;

/// Maximum number of social links per project.
pub const MAX_SOCIAL_LINKS: u32 = 10;

/// Maximum length for social link URL.
pub const MAX_SOCIAL_LINK_URL_LEN: usize = 256;

/// Maximum length for social link platform name.
pub const MAX_SOCIAL_LINK_PLATFORM_LEN: usize = 32;

/// Valid rating range (inclusive). Reviews must be in [RATING_MIN, RATING_MAX]. u32 for Soroban Val.
#[allow(dead_code)]
pub const RATING_MIN: u32 = 1;
#[allow(dead_code)]
pub const RATING_MAX: u32 = 5;

/// Verification validity period in seconds (365 days).
/// After this period, verified projects need to renew their verification.
#[allow(dead_code)]
pub const VERIFICATION_VALIDITY_PERIOD: u64 = 365 * 24 * 60 * 60;

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

/// Maximum number of collections that can exist.
pub const MAX_COLLECTIONS: u32 = 100;

/// Maximum length for a collection name.
pub const MAX_COLLECTION_NAME_LEN: usize = 100;

/// Maximum length for a collection description.
pub const MAX_COLLECTION_DESCRIPTION_LEN: usize = 500;

/// Maximum number of projects per collection.
pub const MAX_PROJECTS_PER_COLLECTION: u32 = 500;

/// TTL bump amount - how much to extend when bumping.
/// Set to the same as the threshold to maintain consistent lifetime.
/// Maximum entries returned per admin action log paginated query.
pub const MAX_ADMIN_ACTION_LOG_PAGE: u32 = 100;

pub const LEDGER_BUMP_CRITICAL: u32 = LEDGER_THRESHOLD_CRITICAL;
pub const LEDGER_BUMP_PROJECT: u32 = LEDGER_THRESHOLD_PROJECT;
pub const LEDGER_BUMP_REVIEW: u32 = LEDGER_THRESHOLD_REVIEW;
pub const LEDGER_BUMP_VERIFICATION: u32 = LEDGER_THRESHOLD_VERIFICATION;
pub const LEDGER_BUMP_USER: u32 = LEDGER_THRESHOLD_USER;

/// Minimum timelock delay in seconds (1 day).
/// Scheduled actions must have execution_timestamp >= now + TIMELOCK_MIN_DELAY.
pub const TIMELOCK_MIN_DELAY: u64 = 86400;

/// Fee payment validity window in seconds (7 days).
/// After this window, the payment record is considered expired and the
/// verification request is rejected until the owner re-pays.
pub const FEE_PAYMENT_EXPIRY_SECONDS: u64 = 7 * 24 * 60 * 60;
