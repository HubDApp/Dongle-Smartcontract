use crate::types::Project;
use crate::utils::validate_project_name;
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
        metadata_cid: Option<String>
    ) -> Result<u64, crate::errors::ContractError> {
        owner.require_auth();
        
        // Validate Project Name Constraints
        validate_project_name(&name)?;
        
        let mut prj_count: u64 = env.storage().instance().get(&soroban_sdk::symbol_short!("count")).unwrap_or(0);
        prj_count += 1;
        env.storage().instance().set(&soroban_sdk::symbol_short!("count"), &prj_count);
        
        let new_project = Project {
            owner: owner.clone(),
            name: name.clone(),
            description,
            category,
            website,
            logo_cid,
            metadata_cid,
        };
        
        env.storage().persistent().set(&prj_count, &new_project);
        
        Ok(prj_count)
    }

    pub fn update_project(
        env: Env, 
        project_id: u64, 
        caller: Address, 
        name: String,
        description: String,
        category: String, 
        website: Option<String>, 
        logo_cid: Option<String>, 
        metadata_cid: Option<String>
    ) -> Result<(), crate::errors::ContractError> {
        caller.require_auth();
        
        let mut project: Project = env.storage().persistent().get(&project_id).unwrap();
        
        if project.owner != caller {
            panic!("unauthorized");
        }
        
        validate_project_name(&name)?;
        
        project.name = name;
        project.description = description;
        project.category = category;
        project.website = website;
        project.logo_cid = logo_cid;
        project.metadata_cid = metadata_cid;
        
        env.storage().persistent().set(&project_id, &project);
        
        Ok(())
    }

    pub fn get_project(env: Env, project_id: u64) -> Option<Project> {
        env.storage().persistent().get(&project_id)
    }
}

mod test;
