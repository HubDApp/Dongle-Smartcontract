use crate::types::{DataKey, Project};
use crate::errors::Error;
use crate::utils::{compute_name_hash, compute_metadata_hash};
use soroban_sdk::{contract, contractimpl, Env, Address, String};

#[contract]
pub struct ProjectRegistry;

#[contractimpl]
impl ProjectRegistry {
    pub fn register_project(
        env: Env,
        owner: Address,
        name: String,
        description: String,
        category: String,
        website: Option<String>,
        logo_cid: Option<String>,
        metadata_cid: Option<String>,
    ) -> Result<u64, Error> {
        owner.require_auth();

        let name_hash = compute_name_hash(&env, &name);
        
        // 1. Check duplicate name
        if env.storage().persistent().has(&DataKey::ProjectNameHash(name_hash.clone())) {
            return Err(Error::DuplicateName);
        }

        // 2. Check metadata hash (for overall uniqueness across owner/description/category)
        let metadata_hash = compute_metadata_hash(&env, &name, &description, &category, &owner);
        if env.storage().persistent().has(&DataKey::ProjectHash(metadata_hash.clone())) {
            return Err(Error::DuplicateProject);
        }

        // Generate unique project ID
        let mut target_id: u64 = env.storage().instance().get(&DataKey::ProjectCounter).unwrap_or(0);
        target_id += 1;
        env.storage().instance().set(&DataKey::ProjectCounter, &target_id);

        let project = Project {
            owner: owner.clone(),
            name,
            description,
            category,
            website,
            logo_cid,
            metadata_cid,
        };

        // Save project and mark hashes as used
        env.storage().persistent().set(&DataKey::Project(target_id), &project);
        env.storage().persistent().set(&DataKey::ProjectNameHash(name_hash), &true);
        env.storage().persistent().set(&DataKey::ProjectHash(metadata_hash), &true);

        Ok(target_id)
    }

    pub fn get_project(env: Env, project_id: u64) -> Option<Project> {
        env.storage().persistent().get(&DataKey::Project(project_id))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{testutils::{Address as _, MockAuth, MockAuthInvoke}, Env, String};

    #[test]
    fn test_duplicate_registration() {
        let env = Env::default();
        let contract_id = env.register_contract(None, ProjectRegistry);
        let client = ProjectRegistryClient::new(&env, &contract_id);

        let owner = Address::generate(&env);
        let name = String::from_str(&env, "Dongle Project");
        let desc = String::from_str(&env, "First description");
        let category = String::from_str(&env, "DeFi");

        client.mock_all_auths().register_project(
            &owner, &name, &desc, &category, &None, &None, &None,
        );

        // Exact match testing should fail with DuplicateName
        let result = client.mock_all_auths().try_register_project(
            &owner, &name, &desc, &category, &None, &None, &None,
        );
        let err = result.err().unwrap().unwrap();
        assert_eq!(err, Error::DuplicateName);
        
        // Slight variation test: Same name but different description should also fail
        let desc_alt = String::from_str(&env, "Second description");
        let result2 = client.mock_all_auths().try_register_project(
            &owner, &name, &desc_alt, &category, &None, &None, &None,
        );
        let err2 = result2.err().unwrap().unwrap();
        assert_eq!(err2, Error::DuplicateName);
    }
}
