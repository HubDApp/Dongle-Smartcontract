#![no_std]
<<<<<<< feat/submit-review-13

//! # Dongle Smart Contract
//! 
//! A decentralized project registry and discovery platform built on Stellar/Soroban.
//! This contract enables transparent project registration, community reviews, and 
//! verification processes for the Stellar ecosystem.

mod types;
mod errors;
mod project_registry;
mod review_registry;
=======
#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(clippy::too_many_arguments)]

use soroban_sdk::{contract, contractimpl, Address, Env, String, Vec};

mod errors;
mod fee_manager;
mod project_registry;
mod review_registry;
mod types;
mod utils;
>>>>>>> main
mod verification_registry;
mod fee_manager;
mod utils;

<<<<<<< feat/submit-review-13
use soroban_sdk::{contract, contractimpl, Address, Env, String, Vec};

use types::{Project, Review, VerificationRecord, FeeConfig};
use crate::errors::ContractError;
use crate::review_registry::ReviewRegistry;
=======
use crate::project_registry::ProjectRegistry;
use crate::types::Project;
>>>>>>> main

/// The main Dongle smart contract
#[contract]
pub struct DongleContract;

/// Contract implementation with all core functionality
#[contractimpl]
impl DongleContract {
<<<<<<< feat/submit-review-13
    // ==========================================
    // INITIALIZATION & ADMIN FUNCTIONS
    // ==========================================

=======
>>>>>>> main
    pub fn initialize(_env: Env, _admin: Address, _treasury: Address) -> Result<(), ContractError> {
        todo!("Contract initialization not yet implemented")
    }

<<<<<<< feat/submit-review-13
    pub fn set_admin(_env: Env, _caller: Address, _new_admin: Address) -> Result<(), ContractError> {
=======
    pub fn set_admin(
        _env: Env,
        _caller: Address,
        _new_admin: Address,
    ) -> Result<(), ContractError> {
>>>>>>> main
        todo!("Admin management not yet implemented")
    }

    // ==========================================
    // PROJECT REGISTRY FUNCTIONS
    // ==========================================

    pub fn register_project(
<<<<<<< feat/submit-review-13
        _env: Env,
        _owner: Address,
        _name: String,
        _description: String,
        _category: String,
        _website: Option<String>,
        _logo_cid: Option<String>,
        _metadata_cid: Option<String>,
    ) -> Result<u64, ContractError> {
        todo!("Project registration not yet implemented")
    }

    pub fn update_project(
=======
        env: Env,
        owner: Address,
        name: String,
        description: String,
        category: String,
        website: Option<String>,
        logo_cid: Option<String>,
        metadata_cid: Option<String>,
    ) -> Result<u64, ContractError> {
        project_registry::ProjectRegistry::register_project(
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

    pub fn update_project(
        env: Env,
        project_id: u64,
        caller: Address,
        name: Option<String>,
        description: Option<String>,
        category: Option<String>,
        website: Option<Option<String>>,
        logo_cid: Option<Option<String>>,
        metadata_cid: Option<Option<String>>,
    ) -> Option<Project> {
        ProjectRegistry::update_project(
            &env,
            project_id,
            caller,
            name,
            description,
            category,
            website,
            logo_cid,
            metadata_cid,
        )
    }

    pub fn get_project(env: Env, project_id: u64) -> Result<Project, ContractError> {
        // ACTUAL IMPLEMENTATION: Replacing todo!() with our retrieval logic
        ProjectRegistry::get_project(&env, project_id).ok_or(ContractError::ProjectNotFound)
    }

    pub fn list_projects(
>>>>>>> main
        _env: Env,
        _project_id: u64,
        _caller: Address,
        _name: String,
        _description: String,
        _category: String,
        _website: Option<String>,
        _logo_cid: Option<String>,
        _metadata_cid: Option<String>,
    ) -> Result<(), ContractError> {
        todo!("Project updates not yet implemented")
    }

    pub fn get_project(_env: Env, _project_id: u64) -> Result<Project, ContractError> {
        todo!("Project retrieval not yet implemented")
    }

    pub fn list_projects(_env: Env, _start_id: u64, _limit: u32) -> Result<Vec<Project>, ContractError> {
        todo!("Project listing not yet implemented")
    }

    // ==========================================
    // REVIEW SYSTEM FUNCTIONS (Your Logic Integrated)
    // ==========================================

    pub fn add_review(
<<<<<<< feat/submit-review-13
        env: Env,
        project_id: u64,
        reviewer: Address,
        rating: u8,
        comment_cid: Option<String>,
    ) -> Result<(), ContractError> {
        ReviewRegistry::add_review(&env, project_id, reviewer, rating, comment_cid)
    }

    pub fn update_review(
        env: Env,
        project_id: u64,
        reviewer: Address,
        rating: u8,
        comment_cid: Option<String>,
    ) -> Result<(), ContractError> {
        ReviewRegistry::update_review(&env, project_id, reviewer, rating, comment_cid)
    }

    pub fn get_review(env: Env, project_id: u64, reviewer: Address) -> Result<Review, ContractError> {
        ReviewRegistry::get_review(&env, project_id, reviewer)
            .ok_or(ContractError::ReviewNotFound)
=======
        _env: Env,
        _project_id: u64,
        _reviewer: Address,
        _rating: u32,
        _comment_cid: Option<String>,
    ) -> Result<(), ContractError> {
        todo!("Review submission not yet implemented")
    }

    pub fn update_review(
        _env: Env,
        _project_id: u64,
        _reviewer: Address,
        _rating: u32,
        _comment_cid: Option<String>,
    ) -> Result<(), ContractError> {
        todo!("Review updates not yet implemented")
    }

    pub fn get_review(
        _env: Env,
        _project_id: u64,
        _reviewer: Address,
    ) -> Result<Review, ContractError> {
        todo!("Review retrieval not yet implemented")
>>>>>>> main
    }

    pub fn get_project_reviews(
        _env: Env,
        _project_id: u64,
        _start_reviewer: Option<Address>,
        _limit: u32,
    ) -> Result<Vec<Review>, ContractError> {
        todo!("Project review listing not yet implemented")
    }

    // ==========================================
    // VERIFICATION SYSTEM FUNCTIONS
    // ==========================================

    pub fn request_verification(
        _env: Env,
        _project_id: u64,
        _requester: Address,
        _evidence_cid: String,
    ) -> Result<(), ContractError> {
        todo!("Verification requests not yet implemented")
    }

    pub fn approve_verification(
        _env: Env,
        _project_id: u64,
        _admin: Address,
    ) -> Result<(), ContractError> {
        todo!("Verification approval not yet implemented")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::{vec, Vec as SorobanVec};

<<<<<<< feat/submit-review-13
    pub fn get_verification(_env: Env, _project_id: u64) -> Result<VerificationRecord, ContractError> {
        todo!("Verification record retrieval not yet implemented")
    }

    // ==========================================
    // FEE MANAGEMENT FUNCTIONS
    // ==========================================

    pub fn set_fee_config(
        _env: Env,
        _admin: Address,
        _token: Option<Address>,
        _verification_fee: u128,
        _registration_fee: u128,
    ) -> Result<(), ContractError> {
        todo!("Fee configuration not yet implemented")
    }
=======
    fn setup_env() -> Env {
        Env::default()
    }

    #[test]
    fn get_project_by_id_returns_complete_metadata() {
        let env = setup_env();
        let contract_id = env.register_contract(None, DongleContract);
        let client = DongleContractClient::new(&env, &contract_id);
        env.mock_all_auths();
>>>>>>> main

        let owner = Address::generate(&env);
        let name = String::from_str(&env, "Alpha");
        let description = String::from_str(&env, "Desc");
        let category = String::from_str(&env, "Tools");
        let logo = Some(String::from_str(&env, "bafylogo"));
        let metadata = Some(String::from_str(&env, "bafymeta"));

<<<<<<< feat/submit-review-13
    pub fn set_treasury(_env: Env, _admin: Address, _treasury: Address) -> Result<(), ContractError> {
        todo!("Treasury management not yet implemented")
=======
        let project_id = client.register_project(
            &owner,
            &name,
            &description,
            &category,
            &None,
            &logo,
            &metadata,
        );

        let project = client.get_project(&project_id).unwrap();
        assert_eq!(project.id, project_id);
        assert_eq!(project.owner, owner);
        assert_eq!(project.name, name);
        assert_eq!(project.description, description);
        assert_eq!(project.category, category);
        assert_eq!(project.logo_cid, logo);
        assert_eq!(project.metadata_cid, metadata);
        assert_eq!(project.created_at, project.updated_at);
>>>>>>> main
    }

    #[test]
    fn get_projects_by_owner_returns_all_projects() {
        let env = setup_env();
        let contract_id = env.register_contract(None, DongleContract);
        let client = DongleContractClient::new(&env, &contract_id);
        env.mock_all_auths();

        let owner = Address::generate(&env);
        let other = Address::generate(&env);

        let id1 = client.register_project(
            &owner,
            &String::from_str(&env, "Alpha"),
            &String::from_str(&env, "A"),
            &String::from_str(&env, "Cat"),
            &None,
            &None,
            &None,
        );
        let id2 = client.register_project(
            &owner,
            &String::from_str(&env, "Beta"),
            &String::from_str(&env, "B"),
            &String::from_str(&env, "Cat"),
            &None,
            &None,
            &None,
        );
        client.register_project(
            &other,
            &String::from_str(&env, "Gamma"),
            &String::from_str(&env, "C"),
            &String::from_str(&env, "Cat"),
            &None,
            &None,
            &None,
        );

        let projects = client.get_projects_by_owner(&owner);
        let mut ids: SorobanVec<u64> = SorobanVec::new(&env);
        for project in projects.iter() {
            ids.push_back(project.id);
        }
        assert_eq!(projects.len(), 2);
        assert_eq!(ids, vec![&env, id1, id2]);
    }
<<<<<<< feat/submit-review-13
}
=======

    #[test]
    fn get_project_returns_none_for_invalid_id() {
        let env = setup_env();
        let contract_id = env.register_contract(None, DongleContract);
        let client = DongleContractClient::new(&env, &contract_id);

        let missing = client.get_project(&9999u64);
        assert!(missing.is_none());
    }

    #[test]
    fn get_projects_by_owner_handles_empty_owner() {
        let env = setup_env();
        let contract_id = env.register_contract(None, DongleContract);
        let client = DongleContractClient::new(&env, &contract_id);

        let owner = Address::generate(&env);
        let projects = client.get_projects_by_owner(&owner);
        assert_eq!(projects.len(), 0);
    }
}
>>>>>>> main
