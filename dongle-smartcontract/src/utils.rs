use crate::types::{Project, Review};
use soroban_sdk::{symbol_short, Address, Env, Map};

/// Utility functions for verifying rating aggregate invariants.
pub struct RatingValidator;

impl RatingValidator {
    /// Verify that the project's rating_sum equals the sum of all review ratings.
    /// 
    /// # Arguments
    /// * `env` - The contract environment
    /// * `project` - The project to verify
    /// 
    /// # Returns
    /// `true` if the rating_sum matches the actual sum of all reviews, `false` otherwise
    pub fn verify_rating_sum(env: &Env, project: &Project) -> bool {
        let reviews: Map<(u64, Address), Review> = env
            .storage()
            .instance()
            .get(&symbol_short!("REVIEWS"))
            .unwrap_or(Map::new(env));

        let mut actual_sum: u64 = 0;
        
        // Iterate through all reviews for this project
        for ((proj_id, _reviewer), review) in reviews.iter() {
            if proj_id == project.id {
                actual_sum += (review.rating as u64) * 100;
            }
        }

        project.rating_sum == actual_sum
    }

    /// Verify that the project's review_count equals the actual number of reviews.
    /// 
    /// # Arguments
    /// * `env` - The contract environment
    /// * `project` - The project to verify
    /// 
    /// # Returns
    /// `true` if the review_count matches the actual count, `false` otherwise
    pub fn verify_review_count(env: &Env, project: &Project) -> bool {
        let reviews: Map<(u64, Address), Review> = env
            .storage()
            .instance()
            .get(&symbol_short!("REVIEWS"))
            .unwrap_or(Map::new(env));

        let mut actual_count: u32 = 0;
        
        // Count all reviews for this project
        for ((proj_id, _reviewer), _review) in reviews.iter() {
            if proj_id == project.id {
                actual_count += 1;
            }
        }

        project.review_count == actual_count
    }

    /// Verify both rating_sum and review_count invariants.
    /// 
    /// # Arguments
    /// * `env` - The contract environment
    /// * `project` - The project to verify
    /// 
    /// # Returns
    /// `true` if both invariants hold, `false` otherwise
    pub fn verify_aggregates(env: &Env, project: &Project) -> bool {
        Self::verify_rating_sum(env, project) && Self::verify_review_count(env, project)
    }
}

