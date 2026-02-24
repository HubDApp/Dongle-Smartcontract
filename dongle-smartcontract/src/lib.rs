#![no_std]

use soroban_sdk::{contract, contractimpl, Env, Address, String};

#[contract]
pub struct DongleContract;

#[contractimpl]
impl DongleContract {
    /// Validates project description length (200–1000 chars).
/// Panics with descriptive message on invalid input.
pub fn register_project(_env: Env, _owner: Address, _name: String, description: String /* other params */) {
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
}

    // Optional simple getter (for future use or testing)
    pub fn get_description(env: Env, /* project_key: Symbol */) -> String {
        // Placeholder - return empty for now
        String::from_str(&env, "")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{Env, Address, String};

    #[test]
    #[should_panic]
    fn test_empty_description_panics() {
        let env = Env::default();
        let owner = Address::from_string(&String::from_str(&env, "GAEXAMPLEADDRESS1234567890"));
        let name = String::from_str(&env, "Test Project");
        let empty_desc = String::from_str(&env, "");

        let _ = DongleContract::register_project(env.clone(), owner, name, empty_desc);
    }

    #[test]
    #[should_panic]
    fn test_short_description_panics() {
        let env = Env::default();
        let owner = Address::from_string(&String::from_str(&env, "GAEXAMPLEADDRESS1234567890"));
        let name = String::from_str(&env, "Test Project");
        let short_desc = String::from_str(&env, "short description"); // < 200 chars

        let _ = DongleContract::register_project(env.clone(), owner, name, short_desc);
    }

    #[test]
    #[should_panic]
    fn test_long_description_panics() {
        let env = Env::default();
        let owner = Address::from_string(&String::from_str(&env, "GAEXAMPLEADDRESS1234567890"));
        let name = String::from_str(&env, "Test Project");
        let long_desc = String::from_str(&env, &("a".repeat(1001))); // > 1000 chars

        let _ = DongleContract::register_project(env.clone(), owner, name, long_desc);
    }

    #[test]
    fn test_valid_description_does_not_panic() {
        let env = Env::default();
        let owner = Address::from_string(&String::from_str(&env, "GAEXAMPLEADDRESS1234567890"));
        let name = String::from_str(&env, "Test Project");
        let valid_desc = String::from_str(&env, &("a".repeat(500))); // 500 chars = valid

        let _ = DongleContract::register_project(env.clone(), owner, name, valid_desc);
        // No panic → test passes automatically
    }
}
