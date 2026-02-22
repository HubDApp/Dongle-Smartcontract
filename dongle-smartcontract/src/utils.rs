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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Project, Review};
    use soroban_sdk::{testutils::Address as _, Address, Env, String};

    fn create_test_project(env: &Env, id: u64, rating_sum: u64, review_count: u32) -> Project {
        Project {
            id,
            owner: Address::generate(env),
            name: String::from_str(env, "Test Project"),
            description: String::from_str(env, "Test Description"),
            category: String::from_str(env, "Test"),
            website: None,
            logo_cid: None,
            metadata_cid: None,
            rating_sum,
            review_count,
            average_rating: if review_count > 0 {
                (rating_sum / review_count as u64) as u32
            } else {
                0
            },
        }
    }

    fn create_test_review(
        env: &Env,
        project_id: u64,
        reviewer: Address,
        rating: u8,
    ) -> Review {
        Review {
            project_id,
            reviewer,
            rating,
            comment_cid: None,
            timestamp: 0,
        }
    }

    #[test]
    fn test_verify_rating_sum_empty() {
        let env = Env::default();
        let project = create_test_project(&env, 1, 0, 0);
        
        assert!(RatingValidator::verify_rating_sum(&env, &project));
    }

    #[test]
    fn test_verify_rating_sum_single_review() {
        let env = Env::default();
        let reviewer = Address::generate(&env);
        let project = create_test_project(&env, 1, 400, 1);
        
        // Add review to storage
        let mut reviews: Map<(u64, Address), Review> = Map::new(&env);
        reviews.set(
            (1, reviewer.clone()),
            create_test_review(&env, 1, reviewer, 4),
        );
        env.storage().instance().set(&symbol_short!("REVIEWS"), &reviews);
        
        assert!(RatingValidator::verify_rating_sum(&env, &project));
    }

    #[test]
    fn test_verify_review_count_empty() {
        let env = Env::default();
        let project = create_test_project(&env, 1, 0, 0);
        
        assert!(RatingValidator::verify_review_count(&env, &project));
    }

    #[test]
    fn test_verify_review_count_multiple() {
        let env = Env::default();
        let reviewer1 = Address::generate(&env);
        let reviewer2 = Address::generate(&env);
        let project = create_test_project(&env, 1, 900, 2);
        
        // Add reviews to storage
        let mut reviews: Map<(u64, Address), Review> = Map::new(&env);
        reviews.set(
            (1, reviewer1.clone()),
            create_test_review(&env, 1, reviewer1, 4),
        );
        reviews.set(
            (1, reviewer2.clone()),
            create_test_review(&env, 1, reviewer2, 5),
        );
        env.storage().instance().set(&symbol_short!("REVIEWS"), &reviews);
        
        assert!(RatingValidator::verify_review_count(&env, &project));
    }

    #[test]
    fn test_verify_aggregates_correct() {
        let env = Env::default();
        let reviewer = Address::generate(&env);
        let project = create_test_project(&env, 1, 500, 1);
        
        // Add review to storage
        let mut reviews: Map<(u64, Address), Review> = Map::new(&env);
        reviews.set(
            (1, reviewer.clone()),
            create_test_review(&env, 1, reviewer, 5),
        );
        env.storage().instance().set(&symbol_short!("REVIEWS"), &reviews);
        
        assert!(RatingValidator::verify_aggregates(&env, &project));
    }
}
