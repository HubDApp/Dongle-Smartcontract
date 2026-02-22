use crate::types::Project;
use soroban_sdk::{symbol_short, Address, Env, Map, String};

const PROJECTS: &str = "PROJECTS";
const PROJECT_COUNTER: &str = "PROJ_CTR";

pub struct ProjectRegistry;

impl ProjectRegistry {
    /// Register a new project with rating aggregates initialized to zero.
    /// 
    /// # Arguments
    /// * `env` - The contract environment
    /// * `owner` - Address of the project owner
    /// * `name` - Project name
    /// * `description` - Project description
    /// * `category` - Project category
    /// * `website` - Optional website URL
    /// * `logo_cid` - Optional IPFS CID for logo
    /// * `metadata_cid` - Optional IPFS CID for metadata
    /// 
    /// # Returns
    /// The newly assigned project ID
    pub fn register_project(
        env: &Env,
        owner: Address,
        name: String,
        description: String,
        category: String,
        website: Option<String>,
        logo_cid: Option<String>,
        metadata_cid: Option<String>,
    ) -> u64 {
        owner.require_auth();

        // Generate unique project ID
        let mut counter: u64 = env
            .storage()
            .instance()
            .get(&symbol_short!("PROJ_CTR"))
            .unwrap_or(0);
        counter += 1;
        env.storage()
            .instance()
            .set(&symbol_short!("PROJ_CTR"), &counter);

        // Create project with rating aggregates initialized to zero
        let project = Project {
            id: counter,
            owner,
            name,
            description,
            category,
            website,
            logo_cid,
            metadata_cid,
            rating_sum: 0,
            review_count: 0,
            average_rating: 0,
        };

        // Save project
        let mut projects: Map<u64, Project> = env
            .storage()
            .instance()
            .get(&symbol_short!("PROJECTS"))
            .unwrap_or(Map::new(env));
        projects.set(counter, project);
        env.storage().instance().set(&symbol_short!("PROJECTS"), &projects);

        counter
    }

    /// Update project metadata (does not affect ratings).
    /// 
    /// # Arguments
    /// * `env` - The contract environment
    /// * `project_id` - ID of the project to update
    /// * `caller` - Address of the caller (must be project owner)
    /// * `name` - New project name
    /// * `description` - New project description
    /// * `category` - New project category
    /// * `website` - New optional website URL
    /// * `logo_cid` - New optional IPFS CID for logo
    /// * `metadata_cid` - New optional IPFS CID for metadata
    pub fn update_project(
        env: &Env,
        project_id: u64,
        caller: Address,
        name: String,
        description: String,
        category: String,
        website: Option<String>,
        logo_cid: Option<String>,
        metadata_cid: Option<String>,
    ) {
        caller.require_auth();

        let projects: Map<u64, Project> = env
            .storage()
            .instance()
            .get(&symbol_short!("PROJECTS"))
            .unwrap_or(Map::new(env));

        if let Some(mut project) = projects.get(project_id) {
            // Validate ownership
            if project.owner != caller {
                panic!("Unauthorized: caller is not project owner");
            }

            // Update metadata (preserve rating aggregates)
            project.name = name;
            project.description = description;
            project.category = category;
            project.website = website;
            project.logo_cid = logo_cid;
            project.metadata_cid = metadata_cid;

            // Save updated project
            let mut projects = projects;
            projects.set(project_id, project);
            env.storage().instance().set(&symbol_short!("PROJECTS"), &projects);
        }
    }

    /// Get a project by ID.
    /// 
    /// # Arguments
    /// * `env` - The contract environment
    /// * `project_id` - ID of the project to retrieve
    /// 
    /// # Returns
    /// The project if found, None otherwise
    pub fn get_project(env: &Env, project_id: u64) -> Option<Project> {
        let projects: Map<u64, Project> = env
            .storage()
            .instance()
            .get(&symbol_short!("PROJECTS"))
            .unwrap_or(Map::new(env));
        projects.get(project_id)
    }

    /// Get the average rating for a project.
    /// 
    /// # Arguments
    /// * `env` - The contract environment
    /// * `project_id` - ID of the project
    /// 
    /// # Returns
    /// The average rating (scaled by 100) or 0 if project not found
    pub fn get_average_rating(env: &Env, project_id: u64) -> u32 {
        Self::get_project(env, project_id)
            .map(|p| p.average_rating)
            .unwrap_or(0)
    }
}
