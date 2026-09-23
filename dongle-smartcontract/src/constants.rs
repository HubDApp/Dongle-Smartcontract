#![allow(dead_code)]
//! Contract limits and validation constants. Kept in one place for easy future updates.

/// Maximum number of projects a single user (address) can register.
/// 50: the `OwnerProjects(Address)` index is a `Vec<u64>` rewritten on every
/// register/transfer/archive; 50 entries keeps that read-modify-write cheap and
/// blocks Sybil-style index bloat. Raise only with a matching pagination story.
pub const MAX_PROJECTS_PER_USER: u32 = 50;

// ── Storage index size limits ───────────────────────────────────────────────
// Vec-based indexes are capped on write to avoid unbounded per-user/project growth.
// See STORAGE_INDEXES.md for the full index catalog and pagination strategy.

/// Maximum unique reviewers indexed per project (`ProjectReviews`).
pub const MAX_REVIEWS_PER_PROJECT: u32 = 20_000;

/// Maximum projects indexed per reviewer (`UserReviews`).
/// Also used as the shared index-capacity error (`MaxProjectsExceeded`) for review indexes.
pub const MAX_REVIEWS_PER_USER: u32 = 200;

/// Maximum items returned per paginated read query across list endpoints.
pub const MAX_PAGE_LIMIT: u32 = 100;

/// Maximum records that can be refreshed by one batch TTL extension call.
pub const MAX_TTL_BATCH_SIZE: u32 = 100;

// ── Field length limits (string byte lengths) ──────────────────────────────
//
// Every user-supplied string field is bounded on write. The limits balance
// three constraints:
//
//   1. **Soroban ledger entry size.** A single persistent entry must stay well
//      under the ~64 KiB XDR limit; project entries aggregate ~15 fields plus
//      Vec indexes, so each field is kept to the smallest size that fits its
//      purpose.
//   2. **Validation cost.** Character-set checks copy the string into a
//      fixed-size stack buffer (`[u8; N]`) in `utils.rs`; the buffer is sized
//      from these constants, so smaller limits mean smaller stack frames and
//      cheaper CPU metering.
//   3. **Display/UX.** Names, categories and slugs appear in lists and URLs and
//      are deliberately short; descriptions are the one "long form" field.
//
// Rationale for each value is on the constant. See `docs/STORAGE_SCHEMA.md` for
// the integrator-facing table and `src/tests/field_limits.rs` for the
// boundary tests that enforce every limit at `N` (accepted) and `N + 1`
// (rejected).

/// Minimum length for name, description, category (must be non-empty after trim in validation).
/// Enforced as "non-empty and not whitespace-only" — a zero-length field carries no information
/// and would break slug/URL generation and list rendering.
#[allow(dead_code)]
pub const MIN_STRING_LEN: usize = 1;

/// Maximum length for project name.
/// 50 bytes: long enough for real project names, short enough to render in a
/// single list row and to derive a slug from. Validation buffer is `[u8; 128]`.
pub const MAX_NAME_LEN: usize = 50;

/// Maximum length for project slug.
/// 64 bytes: URL path segment. Kept slightly larger than the name so a name at
/// `MAX_NAME_LEN` can still be slugified (spaces → `-`) without truncation.
pub const MAX_SLUG_LEN: usize = 64;

/// Maximum length for project description.
/// 2048 bytes: the only long-form field. One order of magnitude below the
/// ledger entry ceiling so a project entry with a full description plus all
/// other fields still serialises comfortably. Validation buffer is
/// `[u8; MAX_DESCRIPTION_LEN]`.
pub const MAX_DESCRIPTION_LEN: usize = 2048;

/// Maximum length for category.
/// 64 bytes: categories are a small controlled vocabulary ("DeFi", "NFT", …);
/// the limit only guards against abuse of the free-text field.
pub const MAX_CATEGORY_LEN: usize = 64;

/// Maximum length for website URL.
/// 256 bytes: comfortably covers real URLs (the de-facto practical URL length
/// browsers rely on is ~2000, but project sites are short landing pages) while
/// bounding the validation buffer `[u8; MAX_WEBSITE_LEN]`.
pub const MAX_WEBSITE_LEN: usize = 256;

/// Maximum length for a project's license description.
/// 64 bytes: fits SPDX identifiers ("Apache-2.0", "GPL-3.0-or-later") and short
/// free-text license notes.
pub const MAX_LICENSE_LEN: usize = 64;

/// Maximum length for a project's published security contact.
/// 256 bytes: covers a `mailto:`, an HTTPS disclosure page, or a
/// `security.txt`-style contact line.
pub const MAX_SECURITY_CONTACT_LEN: usize = 256;

/// Maximum length for any CID (logo, metadata, comment, evidence).
/// 128 bytes: CIDv1 (base32) is ~59 chars; 128 leaves headroom for longer
/// multibase/multicodec encodings without allowing arbitrary blobs.
pub const MAX_CID_LEN: usize = 128;

/// Minimum length for a valid IPFS CID (46 = shortest CIDv0 "Qm..." string).
/// A CIDv0 is exactly 46 base58 characters; anything shorter cannot be a valid CID.
pub const MIN_CID_LEN: usize = 46;

/// Maximum stored edit revisions per review (oldest dropped when exceeded).
/// 50: revision history is a bounded ring buffer so an actively-edited review
/// cannot grow its storage entry without limit.
pub const MAX_REVIEW_REVISIONS: u32 = 50;

/// Bayesian prior review count for weighted rating (see RatingCalculator::calculate_weighted).
/// Represents the "strength" of the prior belief vs actual data.
/// A value of 5 means the prior is treated as if it were 5 hypothetical reviews.
/// Higher values give more weight to the prior (more conservative ratings).
/// Lower values give more weight to actual reviews (more volatile ratings).
/// Chosen as 5 to balance stability with responsiveness to genuine feedback.
pub const WEIGHTED_RATING_PRIOR_COUNT: u32 = 5;

/// Bayesian prior mean rating scaled by 100 (350 = 3.50 stars).
/// Represents the baseline rating for an "average" project.
/// Set to 3.50 (middle of 1-5 scale, slightly above midpoint) to assume
/// most projects are decent but not perfect.
/// Prevents new projects from starting at the extremes (1.0 or 5.0).
/// As review count grows, the actual average dominates this prior.
pub const WEIGHTED_RATING_PRIOR_MEAN: u32 = 350;

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

/// Maximum depth of the transitive project-dependency graph.
///
/// When a project owner adds a dependency that points at another registered
/// project (`DependencyRef::project_id`), the registry walks the transitive
/// dependency graph starting from the new target. Adding an edge that would
/// let the chain reach more than this many levels deep — or that would close
/// a cycle back to the dependent project — is rejected. See
/// `docs/DEPENDENCY_REGISTRY.md`.
pub const MAX_DEPENDENCY_DEPTH: u32 = 5;

/// Maximum number of tags per project.
/// 10: tags are stored as a `Vec<String>` inside the project entry and are
/// scanned linearly on every tag-index update; 10 keeps that O(n) work trivial.
pub const MAX_TAGS_PER_PROJECT: u32 = 10;

/// Maximum length for a single tag.
/// 32 bytes: tags are single words / short slugs (`[A-Za-z0-9_-]`); the
/// validation buffer is sized from this value.
pub const MAX_TAG_LENGTH: usize = 32;

/// Maximum number of social links per project.
/// 10: same `Vec` bound-on-write reasoning as tags; covers every mainstream
/// platform a project would list.
pub const MAX_SOCIAL_LINKS: u32 = 10;

/// Maximum length for social link URL.
/// 256 bytes: matches `MAX_WEBSITE_LEN` — same "short landing/profile URL" shape.
pub const MAX_SOCIAL_LINK_URL_LEN: usize = 256;

/// Maximum length for social link platform name.
/// 32 bytes: platform labels ("twitter", "discord", "github") are short.
pub const MAX_SOCIAL_LINK_PLATFORM_LEN: usize = 32;

/// Valid rating range (inclusive). Reviews must be in [RATING_MIN, RATING_MAX]. u32 for Soroban Val.
pub const RATING_MIN: u32 = 1;
pub const RATING_MAX: u32 = 5;

/// Verification validity period in seconds (365 days).
/// After this period, verified projects need to renew their verification.
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

/// Maximum number of featured projects that can be active simultaneously.
/// When the limit is reached and a new project is featured, the oldest
/// featured project (front of the list) is evicted to make room.
pub const MAX_FEATURED_PROJECTS: u32 = 20;

/// Maximum number of collections that can exist.
/// 100: collections are a curated, admin-scale feature, not user-generated at
/// scale; the global cap keeps `list_collections` pagination bounded.
pub const MAX_COLLECTIONS: u32 = 100;

/// Maximum length for a collection name.
/// 100 bytes: collection names are display titles (curator-authored), allowed
/// to be more descriptive than a project name.
pub const MAX_COLLECTION_NAME_LEN: usize = 100;

/// Maximum length for a collection description.
/// 500 bytes: a short blurb, not long-form prose like a project description.
pub const MAX_COLLECTION_DESCRIPTION_LEN: usize = 500;

/// Maximum number of projects per collection.
/// 500: the membership list is a `Vec<u64>` in the collection entry; 500 × 8
/// bytes = 4 KiB, well within a single ledger entry.
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

// ── Verification Expiry Constants ─────────────────────────────────────────

/// Default duration (in seconds) that a verified status remains active.
/// Defaults to 365 days (365 * 24 * 60 * 60 = 31_536_000 seconds).
/// Admins can override this via set_verification_duration.
pub const DEFAULT_VERIFICATION_DURATION_SECS: u64 = 31_536_000;
/// Minimum timelock delay in seconds (1 day).
///
/// Scheduled admin actions (`schedule_set_fee`, `schedule_add_admin`,
/// `schedule_remove_admin`) must have
/// `execution_timestamp >= now + TIMELOCK_MIN_DELAY`. A delay of `0` (execute
/// immediately) is therefore **not** allowed — it would defeat the purpose of
/// the timelock, which is to give the community and the other admins a
/// guaranteed window to react to a pending change. See `docs/TIMELOCK.md`.
pub const TIMELOCK_MIN_DELAY: u64 = 86_400;

/// Maximum timelock delay in seconds (90 days).
///
/// Scheduled admin actions must have
/// `execution_timestamp <= now + TIMELOCK_MAX_DELAY`. The upper bound keeps
/// the scheduled-action queue bounded and prevents "zombie" actions that sit
/// executable far into the future after their context (admin set, fee model,
/// threat model) has changed. Re-schedule instead of scheduling years out.
/// See `docs/TIMELOCK.md`.
pub const TIMELOCK_MAX_DELAY: u64 = 90 * 24 * 60 * 60;

/// Fee payment validity window in seconds (7 days).
/// After this window, the payment record is considered expired and the
/// verification request is rejected until the owner re-pays.
pub const FEE_PAYMENT_EXPIRY_SECONDS: u64 = 7 * 24 * 60 * 60;

/// Validity window for a pending contract-address claim in seconds (30 days).
/// After this window, a pending claim is treated as expired and a new claim
/// may be submitted for the same address.
pub const CLAIM_EXPIRY_SECONDS: u64 = 30 * 24 * 60 * 60;

/// Minimum seconds a reviewer must wait before updating their review again (default: 1 hour).
/// Configurable by changing this constant.
pub const REVIEW_UPDATE_COOLDOWN_SECONDS: u64 = 3600;

/// Minimum age in seconds for a reviewer before they can submit a review (default: 0, disabled).
pub const DEFAULT_MIN_REVIEWER_AGE_SECONDS: u64 = 0;

/// Default setting for whether endorsements are required for reviews (default: false).
pub const DEFAULT_REQUIRE_ENDORSEMENT: bool = false;

/// Default review fee amount (default: 0, free).
pub const DEFAULT_REVIEW_FEE: u128 = 0;

// ── Contract metadata (read by `get_config`) ────────────────────────────────

/// Semantic version of the contract, surfaced verbatim through `get_config`.
/// Bump when a non-backwards-compatible change to the public contract surface
/// is released (storage layout, argument shape, new required fields, etc.).
pub const CONTRACT_VERSION: &str = "1.0.0";

// ── Recommendation Constants (Issue #820) ──────────────────────────────────

/// Maximum length for a recommendation `label` in bytes. Labels are short
/// display strings ("Trending", "You might like", …); the limit is generous
/// enough for UI copy but tight enough to avoid storage-entry bloat.
pub const MAX_RECOMMENDATION_LABEL_LEN: usize = 128;

/// Maximum number of recommendations a single target project can have across
/// all algorithms. Acts as a flood-gate against storage-key spam for popular
/// projects; 200 is ~5-10 full recommendation surfaces (20 recs × 10 views).
pub const MAX_RECOMMENDATIONS_PER_PROJECT: u32 = 200;

/// Maximum recommendations stored globally (RecommendationList length cap).
/// 10_000 is comfortably above what a single contract instance needs and
/// keeps `list_recommendations` pagination bounded.
pub const MAX_RECOMMENDATIONS_GLOBAL: u32 = 10_000;

/// Weight of click-through rate in the composite effectiveness score.
/// (Scaled basis-point weight; all weights sum to 10_000.)
pub const EFFECTIVENESS_WEIGHT_CTR_BPS: u32 = 3_500;

/// Weight of helpful-ratio in the composite effectiveness score.
pub const EFFECTIVENESS_WEIGHT_HELPFUL_BPS: u32 = 3_500;

/// Weight of downstream engagements (follow + bookmark + endorse + review)
/// in the composite effectiveness score.
pub const EFFECTIVENESS_WEIGHT_ENGAGEMENT_BPS: u32 = 3_000;

/// Scaling factor used for CTR and helpful ratio (ppm = parts per million).
pub const RATIO_SCALE_PPM: u32 = 1_000_000;

/// Composite effectiveness score scale (basis points, 0–10_000).
pub const SCORE_SCALE_BPS: u32 = 10_000;

/// Minimum impressions required before CTR contributes a non-zero signal to
/// the effectiveness score. Prevents a single-click "1/1 = 100%" fluke from
/// dominating the ranking.
pub const MIN_IMPRESSIONS_FOR_CTR_SIGNAL: u64 = 5;

/// Minimum feedback votes required before helpful-ratio contributes a non-zero
/// signal to the effectiveness score (same rationale as CTR floor above).
pub const MIN_FEEDBACK_FOR_HELPFUL_SIGNAL: u64 = 3;

// ── Community Collection Constants (Issue #821) ─────────────────────────────

/// Maximum length of a community collection `name` in bytes (same generous limit
/// as admin-only collections: 100 bytes).
pub const MAX_COMMUNITY_COL_NAME_LEN: usize = 100;

/// Maximum length of a community collection `description` in bytes (short blurb,
/// 500 bytes, same as admin-only collections).
pub const MAX_COMMUNITY_COL_DESCRIPTION_LEN: usize = 500;

/// Maximum length of the optional comma-separated `tags` string in bytes.
/// 256 bytes covers ~20 typical tag tokens.
pub const MAX_COMMUNITY_COL_TAGS_LEN: usize = 256;

/// Maximum number of projects a community collection can contain. Kept lower
/// than admin-only collections (200 vs 500) because these are user-generated
/// and the voting/curation loop is O(|project_ids|) in several places.
pub const MAX_COMMUNITY_COL_PROJECTS: u32 = 200;

/// Maximum curators allowed per community collection. Prevents runaway
/// TTL-extend loops on the curator list.
pub const MAX_COMMUNITY_COL_CURATORS: u32 = 30;

/// Global cap on community collection count. Much higher than admin-only
/// collections since *any* user can create them, but still bounded so
/// `list_community_collections` pagination remains tractable.
pub const MAX_COMMUNITY_COLLECTIONS: u32 = 5_000;

/// Global cap on featured community collections (AC1). Matches the
/// featured-projects cap (20). Admin toggles FIFO-evict the oldest entry
/// when the cap is reached.
pub const MAX_FEATURED_COMMUNITY_COLLECTIONS: u32 = 20;

/// Default approval-vote threshold for newly-created community collections.
/// 3 community yays auto-include unless a curator explicitly vetoes.
pub const DEFAULT_COMMUNITY_COL_APPROVAL_THRESHOLD: u32 = 3;

/// Default disapproval-vote threshold for newly-created community collections.
/// Symmetric with the approval threshold.
pub const DEFAULT_COMMUNITY_COL_DISAPPROVAL_THRESHOLD: u32 = 3;

/// Default creator revenue share (basis points) applied to new community
/// collections when the creator does not explicitly specify one. 60% to
/// the creator leaves 40% to be split evenly across curators.
pub const DEFAULT_COMMUNITY_COL_CREATOR_SHARE_BPS: u32 = 6_000;

/// Maximum valid creator revenue share (basis points). Leaves at least 10%
/// to curators to keep curation incentives aligned.
pub const MAX_COMMUNITY_COL_CREATOR_SHARE_BPS: u32 = 9_000;

/// Minimum valid creator revenue share (basis points). Ensures the creator
/// is meaningfully compensated for originating and stewarding the collection.
pub const MIN_COMMUNITY_COL_CREATOR_SHARE_BPS: u32 = 1_000;

// ── Social Analytics Constants (Issue #822) ────────────────────────────────

/// Daily checkpoint per-project cap. 730 days = ~2 years of daily history.
/// After the cap is reached, `record_project_social_daily_checkpoint` evicts
/// the oldest checkpoint (FIFO) before appending a new one so growth
/// analytics retain a rolling 2-year window rather than failing.
pub const MAX_SOCIAL_CHECKPOINTS_PER_PROJECT: u32 = 730;

/// Scaling factor for engagement-rate ratios. 1e6 ppm = 1.0 (100%).
pub const SOCIAL_ENGAGEMENT_RATE_SCALE_PPM: u64 = 1_000_000;

/// Rating scale used by `ProjectStats.average_rating` (review_registry).
/// Values are 0–50_000 for a 0–5 star rating (10_000 bps per star).
pub const SOCIAL_RATING_BPS_PER_STAR: u32 = 10_000;

/// Maximum number of peer projects to include in the peer comparison
/// snapshot used for AC3. Limiting the peer set to 50 keeps the export
/// report payload small while still representing a broad percentile rank.
pub const SOCIAL_ANALYTICS_MAX_PEERS: u32 = 50;

/// Minimum number of checkpoints required for a "growth over time"
/// export report (AC1 + AC4). With >= 2 checkpoints we can derive at
/// least one delta window. Callers with 1 checkpoint will still get a
/// report but the export will note that deltas are zero.
pub const SOCIAL_ANALYTICS_MIN_CHECKPOINTS_FOR_GROWTH: u32 = 2;

/// Number of days in the "last-week" preset window used by export (6 days + today inclusive = 7).
pub const SOCIAL_WINDOW_7_DAYS: u32 = 7;

/// Number of days in the "last-30-days" preset window used by export.
pub const SOCIAL_WINDOW_30_DAYS: u32 = 30;
