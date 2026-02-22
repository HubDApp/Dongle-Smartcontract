use soroban_sdk::{contracttype, symbol_short, Env, Address, String};

// ── Types ────────────────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct Review {
    pub reviewer:    Address,
    pub project_id:  u64,
    pub rating:      u32,
    pub comment_cid: Option<String>,   // IPFS CID stored on-chain
    pub created_at:  u64,              // ledger timestamp
    pub updated_at:  u64,
}

#[contracttype]
#[derive(Clone, Debug, Default)]
pub struct ProjectAggregate {
    pub total_rating: u64,
    pub review_count: u64,
}

// ── Storage keys ─────────────────────────────────────────────────────────────

#[contracttype]
pub enum DataKey {
    Review(u64, Address),        // (project_id, reviewer) → Review
    Aggregate(u64),              // project_id             → ProjectAggregate
}

// ── Events ───────────────────────────────────────────────────────────────────

fn emit_review_added(env: &Env, project_id: u64, reviewer: &Address, rating: u32) {
    env.events().publish(
        (symbol_short!("rev_add"), project_id),
        (reviewer.clone(), rating),
    );
}

fn emit_review_updated(env: &Env, project_id: u64, reviewer: &Address, old_rating: u32, new_rating: u32) {
    env.events().publish(
        (symbol_short!("rev_upd"), project_id),
        (reviewer.clone(), old_rating, new_rating),
    );
}

// ── Registry ─────────────────────────────────────────────────────────────────

pub struct ReviewRegistry;

impl ReviewRegistry {

    // ── Validation ───────────────────────────────────────────────────────────

    fn validate_rating(rating: u32) {
        assert!(rating >= 1 && rating <= 5, "Rating must be between 1 and 5");
    }

    // ── Aggregate helpers ─────────────────────────────────────────────────────

    fn get_aggregate(env: &Env, project_id: u64) -> ProjectAggregate {
        env.storage()
            .persistent()
            .get(&DataKey::Aggregate(project_id))
            .unwrap_or_default()
    }

    fn save_aggregate(env: &Env, project_id: u64, agg: &ProjectAggregate) {
        env.storage()
            .persistent()
            .set(&DataKey::Aggregate(project_id), agg);
    }

    /// Recompute average: total_rating / review_count (returns 0 when no reviews).
    pub fn average_rating(env: &Env, project_id: u64) -> u64 {
        let agg = Self::get_aggregate(env, project_id);
        if agg.review_count == 0 { 0 } else { agg.total_rating / agg.review_count }
    }

    // ── Core operations ──────────────────────────────────────────────────────

    /// Add a new review for a project.
    /// Panics if the reviewer has already submitted a review for this project.
    pub fn add_review(
        env:         &Env,
        project_id:  u64,
        reviewer:    Address,
        rating:      u32,
        comment_cid: Option<String>,
    ) {
        // 1. Authenticate – the caller must be the reviewer
        reviewer.require_auth();

        // 2. Validate rating range
        Self::validate_rating(rating);

        // 3. Ensure no duplicate review
        let key = DataKey::Review(project_id, reviewer.clone());
        assert!(
            !env.storage().persistent().has(&key),
            "Reviewer has already submitted a review for this project"
        );

        // 4. Persist the review
        let now = env.ledger().timestamp();
        let review = Review {
            reviewer:    reviewer.clone(),
            project_id,
            rating,
            comment_cid,
            created_at: now,
            updated_at: now,
        };
        env.storage().persistent().set(&key, &review);

        // 5. Update aggregate
        let mut agg = Self::get_aggregate(env, project_id);
        agg.total_rating  += rating as u64;
        agg.review_count  += 1;
        Self::save_aggregate(env, project_id, &agg);

        // 6. Emit event
        emit_review_added(env, project_id, &reviewer, rating);
    }

    /// Update an existing review.
    /// Only the original reviewer may call this function.
    /// The aggregate rating is recalculated automatically.
    pub fn update_review(
        env:         &Env,
        project_id:  u64,
        reviewer:    Address,
        new_rating:  u32,
        comment_cid: Option<String>,
    ) {
        // 1. Authenticate – the caller must be the original reviewer
        reviewer.require_auth();

        // 2. Validate new rating
        Self::validate_rating(new_rating);

        // 3. Load existing review – panics if it doesn't exist
        let key = DataKey::Review(project_id, reviewer.clone());
        let mut review: Review = env
            .storage()
            .persistent()
            .get(&key)
            .expect("Review not found: reviewer has not submitted a review for this project");

        // 4. Ownership guard (redundant with require_auth but makes intent explicit)
        assert!(
            review.reviewer == reviewer,
            "Unauthorized: only the original reviewer can update this review"
        );

        // 5. Recalculate aggregate before mutating the review
        let old_rating = review.rating;
        let mut agg = Self::get_aggregate(env, project_id);
        agg.total_rating = agg.total_rating
            .saturating_sub(old_rating as u64)
            .saturating_add(new_rating as u64);
        // review_count stays the same
        Self::save_aggregate(env, project_id, &agg);

        // 6. Overwrite the review with new data
        review.rating      = new_rating;
        review.comment_cid = comment_cid; // new IPFS CID replaces previous one
        review.updated_at  = env.ledger().timestamp();
        env.storage().persistent().set(&key, &review);

        // 7. Emit update event
        emit_review_updated(env, project_id, &reviewer, old_rating, new_rating);
    }

    /// Retrieve a single review, returning None if it doesn't exist.
    pub fn get_review(env: &Env, project_id: u64, reviewer: Address) -> Option<Review> {
        env.storage()
            .persistent()
            .get(&DataKey::Review(project_id, reviewer))
    }
}