#![no_std]

mod errors;
mod project_registry;
mod rating_calculator;
mod review_registry;
mod types;
mod utils;

pub use errors::ContractError;
pub use project_registry::ProjectRegistry;
pub use rating_calculator::RatingCalculator;
pub use review_registry::ReviewRegistry;
pub use types::{Project, Review};
pub use utils::RatingValidator;

use soroban_sdk::{contract, contractimpl, Address, Env, String};

#[contract]
pub struct DongleContract;

#[contractimpl]
impl DongleContract {
    /// Register a new project
    pub fn register_project(
        env: Env,
        owner: Address,
        name: String,
        description: String,
        category: String,
        website: Option<String>,
        logo_cid: Option<String>,
        metadata_cid: Option<String>,
    ) -> u64 {
        ProjectRegistry::register_project(
            &env,
            owner,
            name,
            description,
            category,
            website,
            logo_cid,
            metadata_cid,
        )
    }

    /// Get a project by ID
    pub fn get_project(env: Env, project_id: u64) -> Option<Project> {
        ProjectRegistry::get_project(&env, project_id)
    }

    /// Get average rating for a project
    pub fn get_average_rating(env: Env, project_id: u64) -> u32 {
        ProjectRegistry::get_average_rating(&env, project_id)
    }

    /// Add a review for a project
    pub fn add_review(
        env: Env,
        project_id: u64,
        reviewer: Address,
        rating: u8,
        comment_cid: Option<String>,
    ) -> Result<(), ContractError> {
        ReviewRegistry::add_review(&env, project_id, reviewer, rating, comment_cid)
    }

    /// Update an existing review
    pub fn update_review(
        env: Env,
        project_id: u64,
        reviewer: Address,
        rating: u8,
        comment_cid: Option<String>,
    ) -> Result<(), ContractError> {
        ReviewRegistry::update_review(&env, project_id, reviewer, rating, comment_cid)
    }

    /// Delete a review
    pub fn delete_review(
        env: Env,
        project_id: u64,
        reviewer: Address,
    ) -> Result<(), ContractError> {
        ReviewRegistry::delete_review(&env, project_id, reviewer)
    }

    /// Get a specific review
    pub fn get_review(env: Env, project_id: u64, reviewer: Address) -> Option<Review> {
        ReviewRegistry::get_review(&env, project_id, reviewer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env};

    #[test]
    fn test_full_workflow() {
        let env = Env::default();
        let owner = Address::generate(&env);
        let reviewer1 = Address::generate(&env);
        let reviewer2 = Address::generate(&env);

        // Register project
        let project_id = DongleContract::register_project(
            env.clone(),
            owner.clone(),
            String::from_str(&env, "Test Project"),
            String::from_str(&env, "A test project"),
            String::from_str(&env, "Testing"),
            None,
            None,
            None,
        );

        assert_eq!(project_id, 1);

        // Verify initial state
        let project = DongleContract::get_project(env.clone(), project_id).unwrap();
        assert_eq!(project.rating_sum, 0);
        assert_eq!(project.review_count, 0);
        assert_eq!(project.average_rating, 0);

        // Add first review (rating: 4)
        env.mock_all_auths();
        DongleContract::add_review(
            env.clone(),
            project_id,
            reviewer1.clone(),
            4,
            None,
        )
        .unwrap();

        let project = DongleContract::get_project(env.clone(), project_id).unwrap();
        assert_eq!(project.rating_sum, 400);
        assert_eq!(project.review_count, 1);
        assert_eq!(project.average_rating, 400); // 4.00

        // Add second review (rating: 5)
        DongleContract::add_review(
            env.clone(),
            project_id,
            reviewer2.clone(),
            5,
            None,
        )
        .unwrap();

        let project = DongleContract::get_project(env.clone(), project_id).unwrap();
        assert_eq!(project.rating_sum, 900);
        assert_eq!(project.review_count, 2);
        assert_eq!(project.average_rating, 450); // 4.50

        // Update first review (4 -> 3)
        DongleContract::update_review(
            env.clone(),
            project_id,
            reviewer1.clone(),
            3,
            None,
        )
        .unwrap();

        let project = DongleContract::get_project(env.clone(), project_id).unwrap();
        assert_eq!(project.rating_sum, 800);
        assert_eq!(project.review_count, 2);
        assert_eq!(project.average_rating, 400); // 4.00

        // Delete second review
        DongleContract::delete_review(env.clone(), project_id, reviewer2.clone()).unwrap();

        let project = DongleContract::get_project(env.clone(), project_id).unwrap();
        assert_eq!(project.rating_sum, 300);
        assert_eq!(project.review_count, 1);
        assert_eq!(project.average_rating, 300); // 3.00

        // Delete last review
        DongleContract::delete_review(env.clone(), project_id, reviewer1.clone()).unwrap();

        let project = DongleContract::get_project(env.clone(), project_id).unwrap();
        assert_eq!(project.rating_sum, 0);
        assert_eq!(project.review_count, 0);
        assert_eq!(project.average_rating, 0); // Reset to zero
    }

    #[test]
    fn test_invalid_rating() {
        let env = Env::default();
        let owner = Address::generate(&env);
        let reviewer = Address::generate(&env);

        env.mock_all_auths();

        let project_id = DongleContract::register_project(
            env.clone(),
            owner,
            String::from_str(&env, "Test"),
            String::from_str(&env, "Test"),
            String::from_str(&env, "Test"),
            None,
            None,
            None,
        );

        // Test rating too low
        let result = DongleContract::add_review(env.clone(), project_id, reviewer.clone(), 0, None);
        assert_eq!(result, Err(ContractError::InvalidRating));

        // Test rating too high
        let result = DongleContract::add_review(env.clone(), project_id, reviewer, 6, None);
        assert_eq!(result, Err(ContractError::InvalidRating));
    }

    #[test]
    fn test_duplicate_review() {
        let env = Env::default();
        let owner = Address::generate(&env);
        let reviewer = Address::generate(&env);

        env.mock_all_auths();

        let project_id = DongleContract::register_project(
            env.clone(),
            owner,
            String::from_str(&env, "Test"),
            String::from_str(&env, "Test"),
            String::from_str(&env, "Test"),
            None,
            None,
            None,
        );

        // Add first review
        DongleContract::add_review(env.clone(), project_id, reviewer.clone(), 4, None).unwrap();

        // Try to add duplicate
        let result = DongleContract::add_review(env.clone(), project_id, reviewer, 5, None);
        assert_eq!(result, Err(ContractError::ReviewAlreadyExists));
    }
}
