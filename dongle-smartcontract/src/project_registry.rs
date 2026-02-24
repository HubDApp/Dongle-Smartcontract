extern crate alloc;
use alloc::string::ToString;
use crate::types::Project;
use soroban_sdk::{Env, Address, Map, String};
use crate::errors::Error;

pub struct ProjectRegistry;

pub const MAX_NAME_LEN: usize = 50;

impl ProjectRegistry {
    pub fn register_project(
        env: &Env, 
        owner: Address, 
        name: String, 
        description: String, 
        category: String, 
        website: Option<String>, 
        logo_cid: Option<String>, 
        metadata_cid: Option<String>
    ) -> Result<u64, Error> {
        let name_str = name.to_string();
        
        // 1. Validate Non-empty and not only whitespace
        if name_str.trim().is_empty() {
            return Err(Error::InvalidProjectName);
        }
        
        // 2. Validate max length
        if name_str.len() > MAX_NAME_LEN {
            return Err(Error::ProjectNameTooLong);
        }
        
        // 3. Validate alphanumeric, underscore, hyphen
        for c in name_str.chars() {
            if !c.is_ascii_alphanumeric() && c != '_' && c != '-' {
                return Err(Error::InvalidProjectNameFormat);
            }
        }

        // Generate unique project ID
        // Save project in Map<u64, Project>
        // Emit ProjectRegistered event
        Ok(0)
    }

    pub fn update_project(env: &Env, project_id: u64, caller: Address) {
        // Validate ownership
        // Update project metadata
    }

    pub fn get_project(env: &Env, project_id: u64) -> Option<Project> {
        None
    }
}

// Unit tests covering valid and invalid cases
#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::Env;

    #[test]
    fn test_valid_project_name() {
        let env = Env::default();
        let owner = Address::generate(&env);
        let name = String::from_str(&env, "Valid-Project_Name123");
        
        let result = ProjectRegistry::register_project(
            &env, owner, name, 
            String::from_str(&env, "Desc"), String::from_str(&env, "Cat"), 
            None, None, None
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_empty_or_whitespace_name() {
        let env = Env::default();
        let owner = Address::generate(&env);
        let name = String::from_str(&env, "   ");
        
        let result = ProjectRegistry::register_project(
            &env, owner, name, 
            String::from_str(&env, "Desc"), String::from_str(&env, "Cat"), 
            None, None, None
        );
        assert_eq!(result, Err(Error::InvalidProjectName));
    }

    #[test]
    fn test_invalid_characters_in_name() {
        let env = Env::default();
        let owner = Address::generate(&env);
        let name = String::from_str(&env, "My Project *");
        
        let result = ProjectRegistry::register_project(
            &env, owner, name, 
            String::from_str(&env, "Desc"), String::from_str(&env, "Cat"), 
            None, None, None
        );
        assert_eq!(result, Err(Error::InvalidProjectNameFormat));
    }

    #[test]
    fn test_name_too_long() {
        let env = Env::default();
        let owner = Address::generate(&env);
        // 51 characters
        let name = String::from_str(&env, "ThisProjectNameIsWayTooLongAndExceedsTheFiftyCharL1");
        
        let result = ProjectRegistry::register_project(
            &env, owner, name, 
            String::from_str(&env, "Desc"), String::from_str(&env, "Cat"), 
            None, None, None
        );
        assert_eq!(result, Err(Error::ProjectNameTooLong));
    }
}
