use crate::types::Project;
use soroban_sdk::{Env, Address, Map};

pub struct ProjectRegistry;

impl ProjectRegistry {
    pub fn register_project(env: &Env, owner: Address, name: String, description: String, category: String, website: Option<String>, logo_cid: Option<String>, metadata_cid: Option<String>) -> u64 {
        // Generate unique project ID
         let project_id: u64 = 0;
        // Capture registration timestamp from the current ledger
        let registered_at: u64 = env.ledger().timestamp();

        let project = Project {
            id: project_id,
            owner,
            name,
            description,
            category,
            website,
            logo_cid,
            metadata_cid,
            registered_at,
        };

        // Save project in Map<u64, Project>
        // Emit ProjectRegistered event

        project_id
    }

    pub fn update_project(env: &Env, project_id: u64, caller: Address, ...) {
        // Validate ownership
        // Update project metadata
    }

    pub fn get_project(env: &Env, project_id: u64) -> Option<Project> {
        None
    }
}
