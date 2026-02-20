#![no_std]

//! Dongle Smart Contract: project registry, reviews, and verification on Stellar/Soroban.

mod errors;
mod fee_manager;
mod project_registry;
mod review_registry;
mod types;
mod utils;
mod verification_registry;

use soroban_sdk::{contract, contractimpl, Env, Address, String};

use errors::ContractError;
use types::{FeeConfig, Project, Review, VerificationRecord};

#[contract]
pub struct DongleContract;

#[contractimpl]
impl DongleContract {
    /// Validates project description length (200–1000 chars).
    /// Panics with descriptive message on invalid input.
    pub fn register_project(env: Env, owner: Address, name: String, description: String /* other params */) {
        owner.require_auth();  // ← add this if upstream/main has it (security)

        let desc_len = description.len() as u32;

        if desc_len == 0 {
            panic!("Description cannot be empty");
        }
        if desc_len < 200 {
            panic!("Description must be at least 200 characters long");
        }
        if desc_len > 1000 {
            panic!("Description exceeds maximum length of 1000 characters");
        }

        // ← Add the rest of the function body from upstream/HEAD if needed
        // (e.g. storage.set, events.publish, return project_id)
        // For now, if upstream had todo!(), just leave as panic or empty
    }

    // Optional: simple getter for testing
    pub fn get_description(env: Env /* , project_key: Symbol */) -> String {
        String::from_str(&env, "")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{Env, Address, String};

    #[test]
    #[should_panic(expected = "Description cannot be empty")]
    fn test_empty_description_panics() {
        let env = Env::default();
        let owner = Address::from_string(&env, "GAEXAMPLEADDRESS1234567890");
        let name = String::from_str(&env, "Test Project");
        let empty_desc = String::from_str(&env, "");

        DongleContract::register_project(env.clone(), owner, name, empty_desc);
    }

    #[test]
    #[should_panic(expected = "Description must be at least 200 characters long")]
    fn test_short_description_panics() {
        let env = Env::default();
        let owner = Address::from_string(&env, "GAEXAMPLEADDRESS1234567890");
        let name = String::from_str(&env, "Test Project");
        let short_desc = String::from_str(&env, "short"); // < 200

        DongleContract::register_project(env.clone(), owner, name, short_desc);
    }

    #[test]
    #[should_panic(expected = "Description exceeds maximum length of 1000 characters")]
    fn test_long_description_panics() {
        let env = Env::default();
        let owner = Address::from_string(&env, "GAEXAMPLEADDRESS1234567890");
        let name = String::from_str(&env, "Test Project");
        let long_desc = String::from_str(&env, &("a".repeat(1001))); // > 1000

        DongleContract::register_project(env.clone(), owner, name, long_desc);
    }

    #[test]
    fn test_valid_description_does_not_panic() {
        let env = Env::default();
        let owner = Address::from_string(&env, "GAEXAMPLEADDRESS1234567890");
        let name = String::from_str(&env, "Test Project");
        let valid_desc = String::from_str(&env, &("a".repeat(500))); // valid

        DongleContract::register_project(env.clone(), owner, name, valid_desc);
        // No panic = pass
    }
}