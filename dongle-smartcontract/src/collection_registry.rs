//! Collection registry – project lifecycle and listing.

use soroban_sdk::{contract, contractimpl, Address, Env, String, Vec};

use crate::constants::*;
use crate::errors::ContractError;
use crate::storage_keys::StorageKey;
use crate::types::{
    Project, ProjectRegistrationParams, ProjectUpdateParams, Review, ReviewParams, SlugIndexKey,
};
use crate::utils;

#[contract]
pub struct CollectionRegistry;

#[contractimpl]
impl CollectionRegistry {
    pub fn initialize(env: Env, admin: Address) -> Result<(), ContractError> {
        if utils::is_initialized(&env) {
            return Err(ContractError::AlreadyInitialized);
        }
        env.storage().instance().set(&StorageKey::Admin, &admin);
        env.storage().instance().set(&StorageKey::NextProjectId, &1u64);
        env.storage().instance().set(&StorageKey::Initialized, &true);
        Ok(())
    }

    pub fn register_project(
        env: Env,
        params: ProjectRegistrationParams,
    ) -> Result<u64, ContractError> {
        // Validate fields
        if !utils::is_valid_slug(&params.slug) {
            return Err(ContractError::InvalidSlug);
        }
        if params.name.len() == 0 || params.name.len() > MAX_NAME_LENGTH as u32 {
            return Err(ContractError::InvalidName);
        }
        if params.description.len() == 0 || params.description.len() > MAX_DESCRIPTION_LENGTH as u32 {
            return Err(ContractError::InvalidDescription);
        }
        if !utils::is_valid_category(&params.category) {
            return Err(ContractError::InvalidCategory);
        }

        // Validate optional bounty URL/CID
        if let Some(ref url) = params.bounty_url {
            if !url.starts_with("http://") && !url.starts_with("https://") {
                return Err(ContractError::InvalidBountyUrl);
            }
            // If it looks like a URL, validate further (basic length check)
            if url.len() < 10 {
                return Err(ContractError::InvalidBountyUrl);
            }
        }

        // Check slug uniqueness
        let slug_key = SlugIndexKey { slug: params.slug.clone() };
        if env.storage().persistent().has(&slug_key) {
            return Err(ContractError::SlugAlreadyExists);
        }

        // Check owner projects count limit
        let owner = params.owner.clone();
        let owner_projects_key = StorageKey::OwnerProjects(owner.clone());
        let mut owner_projects: Vec<u64> = env
            .storage()
            .persistent()
            .get(&owner_projects_key)
            .unwrap_or(Vec::new(&env));
        if owner_projects.len() >= MAX_PROJECTS_PER_USER as u32 {
            return Err(ContractError::MaxProjectsExceeded);
        }

        let project_id = env
            .storage()
            .instance()
            .get::<_, u64>(&StorageKey::NextProjectId)
            .unwrap();

        let project = Project {
            id: project_id,
            owner: owner.clone(),
            name: params.name,
            slug: params.slug,
            description: params.description,
            category: params.category,
            website: params.website,
            license: params.license,
            logo_cid: params.logo_cid,
            metadata_cid: params.metadata_cid,
            tags: params.tags,
            social_links: params.social_links,
            launch_timestamp: params.launch_timestamp,
            bounty_url: params.bounty_url,
            maintainers: None,
            archived: false,
            created_at: env.ledger().timestamp(),
            updated_at: env.ledger().timestamp(),
            verification_status: crate::types::VerificationStatus::Unverified,
            verified_at: None,
        };

        // Store project
        let project_key = StorageKey::Project(project_id);
        env.storage().persistent().set(&project_key, &project);

        // Update indexes
        owner_projects.push_back(project_id);
        env.storage()
            .persistent()
            .set(&owner_projects_key, &owner_projects);

        // Store slug index
        env.storage()
            .persistent()
            .set(&slug_key, &project_id);

        // Increment next ID
        env.storage()
            .instance()
            .set(&StorageKey::NextProjectId, &(project_id + 1));

        // Emit event (not shown for brevity)

        Ok(project_id)
    }

    pub fn update_project(
        env: Env,
        project_id: u64,
        caller: Address,
        params: ProjectUpdateParams,
    ) -> Result<(), ContractError> {
        let project_key = StorageKey::Project(project_id);
        let mut project: Project = env
            .storage()
            .persistent()
            .get(&project_key)
            .ok_or(ContractError::ProjectNotFound)?;

        // Authorization check
        if project.owner != caller && !utils::is_maintainer(&env, &project, &caller) {
            return Err(ContractError::NotOwner);
        }

        // Update optional fields
        if let Some(ref name) = params.name {
            if name.len() == 0 || name.len() > MAX_NAME_LENGTH as u32 {
                return Err(ContractError::InvalidName);
            }
            project.name = name.clone();
        }
        if let Some(ref description) = params.description {
            if description.len() == 0 || description.len() > MAX_DESCRIPTION_LENGTH as u32 {
                return Err(ContractError::InvalidDescription);
            }
            project.description = description.clone();
        }
        if let Some(ref category) = params.category {
            if !utils::is_valid_category(category) {
                return Err(ContractError::InvalidCategory);
            }
            project.category = category.clone();
        }
        if let Some(website) = params.website {
            project.website = website;
        }
        if let Some(license) = params.license {
            project.license = license;
        }
        if let Some(logo_cid) = params.logo_cid {
            project.logo_cid = logo_cid;
        }
        if let Some(metadata_cid) = params.metadata_cid {
            project.metadata_cid = metadata_cid;
        }
        if let Some(tags) = params.tags {
            project.tags = tags;
        }
        if let Some(social_links) = params.social_links {
            project.social_links = social_links;
        }
        if let Some(launch_timestamp) = params.launch_timestamp {
            project.launch_timestamp = launch_timestamp;
        }

        // Update bounty_url with validation
        if let Some(ref bounty_url) = params.bounty_url {
            // Validate URL or CID
            if !bounty_url.starts_with("http://") && !bounty_url.starts_with("https://") {
                return Err(ContractError::InvalidBountyUrl);
            }
            if bounty_url.len() < 10 {
                return Err(ContractError::InvalidBountyUrl);
            }
            project.bounty_url = Some(bounty_url.clone());
        } else if params.bounty_url_clear {
            // Clear existing bounty if a clear flag is set (could be omitted)
            project.bounty_url = None;
        }

        project.updated_at = env.ledger().timestamp();

        env.storage().persistent().set(&project_key, &project);
        Ok(())
    }

    // Other methods omitted...
}
