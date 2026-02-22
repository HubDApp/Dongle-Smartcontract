use crate::errors::ContractError;
use crate::rating_calculator::RatingCalculator;
use crate::types::{Project, Review};
use soroban_sdk::{symbol_short, Address, Env, Map, String};

const REVIEWS: &str = "REVIEWS";
const PROJECTS: &str = "PROJECTS";

pub struct ReviewRegistry;

impl ReviewRegistry {
    /// Add a new review for a project and update rating aggregates.
    /// 
    /// # Arguments
    /// * `env` - The contract environment
    /// * `project_id` - ID of the project being reviewed
    /// * `reviewer` - Address of the reviewer
    /// * `rating` - Rating value (must be 1-5)
    /// * `comment_cid` - Optional IPFS CID for review comment
    /// 
    /// # Errors
    /// * `InvalidRating` - If rating is not in range 1-5
    /// * `ReviewAlreadyExists` - If reviewer has already reviewed this project
    /// * `ProjectNotFound` - If project doesn't exist
    pub fn add_review(
        env: &Env,
        project_id: u64,
        reviewer: Address,
        rating: u8,
        comment_cid: Option<String>,
    ) -> Result<(), ContractError> {
        // Validate rating is in range 1-5
        if rating < 1 || rating > 5 {
            return Err(ContractError::InvalidRating);
        }

        // Check if review already exists
        let reviews: Map<(u64, Address), Review> = env.storage().instance().get(&symbol_short!("REVIEWS")).unwrap_or(Map::new(env));
        let review_key = (project_id, reviewer.clone());
        
        if reviews.contains_key(review_key.clone()) {
            return Err(ContractError::ReviewAlreadyExists);
        }

        // Get project
        let projects: Map<u64, Project> = env.storage().instance().get(&symbol_short!("PROJECTS")).unwrap_or(Map::new(env));
        let mut project = projects.get(project_id).ok_or(ContractError::ProjectNotFound)?;

        // Create review
        let review = Review {
            project_id,
            reviewer: reviewer.clone(),
            rating,
            comment_cid,
            timestamp: env.ledger().timestamp(),
        };

        // Update rating aggregates
        let (new_sum, new_count, new_average) = RatingCalculator::add_rating(
            project.rating_sum,
            project.review_count,
            rating,
        );

        project.rating_sum = new_sum;
        project.review_count = new_count;
        project.average_rating = new_average;

        // Save review and updated project
        let mut reviews = reviews;
        reviews.set(review_key, review);
        env.storage().instance().set(&symbol_short!("REVIEWS"), &reviews);

        let mut projects = projects;
        projects.set(project_id, project);
        env.storage().instance().set(&symbol_short!("PROJECTS"), &projects);

        Ok(())
    }

    pub fn update_review(
        env: &Env,
        project_id: u64,
        reviewer: Address,
        rating: u8,
        comment_cid: Option<String>,
    ) -> Result<(), ContractError> {
        // Validate rating is in range 1-5
        if rating < 1 || rating > 5 {
            return Err(ContractError::InvalidRating);
        }

        // Verify caller is the original reviewer
        reviewer.require_auth();

        // Get existing review
        let reviews: Map<(u64, Address), Review> = env.storage().instance().get(&symbol_short!("REVIEWS")).unwrap_or(Map::new(env));
        let review_key = (project_id, reviewer.clone());
        
        let mut review = reviews.get(review_key.clone()).ok_or(ContractError::ReviewNotFound)?;
        let old_rating = review.rating;

        // Get project
        let projects: Map<u64, Project> = env.storage().instance().get(&symbol_short!("PROJECTS")).unwrap_or(Map::new(env));
        let mut project = projects.get(project_id).ok_or(ContractError::ProjectNotFound)?;

        // Update rating aggregates
        let (new_sum, new_count, new_average) = RatingCalculator::update_rating(
            project.rating_sum,
            project.review_count,
            old_rating,
            rating,
        );

        project.rating_sum = new_sum;
        project.review_count = new_count;
        project.average_rating = new_average;

        // Update review
        review.rating = rating;
        review.comment_cid = comment_cid;
        review.timestamp = env.ledger().timestamp();

        // Save updated review and project
        let mut reviews = reviews;
        reviews.set(review_key, review);
        env.storage().instance().set(&symbol_short!("REVIEWS"), &reviews);

        let mut projects = projects;
        projects.set(project_id, project);
        env.storage().instance().set(&symbol_short!("PROJECTS"), &projects);

        Ok(())
    }

    pub fn delete_review(
        env: &Env,
        project_id: u64,
        reviewer: Address,
    ) -> Result<(), ContractError> {
        Ok(())
    }

    pub fn get_review(env: &Env, project_id: u64, reviewer: Address) -> Option<Review> {
        let reviews: Map<(u64, Address), Review> = env.storage().instance().get(&symbol_short!("REVIEWS")).unwrap_or(Map::new(env));
        reviews.get((project_id, reviewer))
    }
}
