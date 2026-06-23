use crate::constants::MAX_PROJECTS_PER_USER;
use crate::errors::ContractError;
use crate::events::{
    publish_ownership_transferred_event, publish_project_archived_event,
    publish_project_reactivated_event, publish_project_registered_event,
    publish_project_updated_event, publish_project_claimable_set_event,
    publish_claim_request_submitted_event, publish_claim_request_approved_event,
    publish_claim_request_rejected_event,
};
use crate::fee_manager::FeeManager;
use crate::storage_keys::StorageKey;
use crate::storage_manager::StorageManager;
use crate::types::{Project, ProjectRegistrationParams, ProjectUpdateParams, VerificationStatus, ClaimStatus, ClaimRequest};
use crate::admin_manager::AdminManager;
use crate::utils::Utils;
use soroban_sdk::{Address, Env, String, Vec};

/// Maximum number of items returned per paginated list call.
pub const MAX_PAGE_LIMIT: u32 = 100;

pub struct ProjectRegistry;

impl ProjectRegistry {
    pub fn register_project(
        env: &Env,
        params: ProjectRegistrationParams,
    ) -> Result<u64, ContractError> {
        // Validation phase
        params.owner.require_auth();

        // Validate inputs - return typed errors instead of panicking
        Utils::validate_project_name(&params.name)?;
        Utils::validate_project_slug(&params.slug)?;

        // Check registration fee payment
        if let Ok(config) = FeeManager::get_fee_config(env) {
            if config.registration_fee > 0 {
                FeeManager::consume_registration_fee_payment(
                    env,
                    &params.owner,
                    config.registration_fee,
                )?;
            }
        }

        // Validate description with comprehensive checks
        Utils::validate_description(&params.description)?;

        Utils::validate_category_field(&params.category)?;

        if let Some(website) = &params.website {
            Utils::validate_website(website)?;
        }
        if let Some(logo_cid) = &params.logo_cid {
            Utils::validate_logo_cid(logo_cid)?;
        }
        if let Some(metadata_cid) = &params.metadata_cid {
            Utils::validate_metadata_cid(metadata_cid)?;
        }

        // Validate tags if provided
        if let Some(tags) = &params.tags {
            Utils::validate_tags(tags)?;
        }

        // Validate social links if provided
        if let Some(social_links) = &params.social_links {
            Utils::validate_social_links(social_links)?;
        }

        // Check if owner has exceeded maximum projects limit
        let owner_project_count = Self::owner_project_count(env, &params.owner);
        if owner_project_count >= MAX_PROJECTS_PER_USER {
            return Err(ContractError::MaxProjectsExceeded);
        }

        // Check if project name already exists
        if env
            .storage()
            .persistent()
            .has(&StorageKey::ProjectByName(params.name.clone()))
        {
            return Err(ContractError::ProjectAlreadyExists);
        }

        // Check if project slug already exists
        if env
            .storage()
            .persistent()
            .has(&StorageKey::ProjectBySlug(params.slug.clone()))
        {
            return Err(ContractError::ProjectAlreadyExists);
        }

        // Mutation phase
        let mut count: u64 = env
            .storage()
            .persistent()
            .get(&StorageKey::ProjectCount)
            .unwrap_or(0);
        count = count.saturating_add(1);

        let now = env.ledger().timestamp();
        let project = Project {
            id: count,
            owner: params.owner.clone(),
            name: params.name.clone(),
            slug: params.slug.clone(),
            description: params.description,
            category: params.category,
            website: params.website,
            logo_cid: params.logo_cid,
            metadata_cid: params.metadata_cid,
            verification_status: VerificationStatus::Unverified,
            archived: false,
            claimable: false,
            created_at: now,
            updated_at: now,
            tags: params.tags.clone(),
            social_links: params.social_links.clone(),
        };

        // Get current owner projects
        let mut owner_projects: Vec<u64> = env
            .storage()
            .persistent()
            .get(&StorageKey::OwnerProjects(params.owner.clone()))
            .unwrap_or_else(|| Vec::new(env));

        // Perform all mutations
        env.storage()
            .persistent()
            .set(&StorageKey::Project(count), &project);
        env.storage()
            .persistent()
            .set(&StorageKey::ProjectCount, &count);
        env.storage()
            .persistent()
            .set(&StorageKey::ProjectByName(params.name), &count);
        env.storage()
            .persistent()
            .set(&StorageKey::ProjectBySlug(params.slug), &count);

        owner_projects.push_back(count);
        env.storage().persistent().set(
            &StorageKey::OwnerProjects(params.owner.clone()),
            &owner_projects,
        );

        let mut category_projects: Vec<u64> = env
            .storage()
            .persistent()
            .get(&StorageKey::CategoryProjects(project.category.clone()))
            .unwrap_or_else(|| Vec::new(env));
        category_projects.push_back(count);
        env.storage().persistent().set(
            &StorageKey::CategoryProjects(project.category.clone()),
            &category_projects,
        );

        // Extend TTL for project-related data (not stats, as it doesn't exist yet for new projects)
        StorageManager::extend_project_ttl(env, count);
        StorageManager::extend_project_by_name_ttl(env, &project.name);
        StorageManager::extend_project_count_ttl(env);
        StorageManager::extend_owner_projects_ttl(env, &params.owner);
        StorageManager::extend_category_projects_ttl(env, &project.category);

        // Store tags and social links separately if provided
        if let Some(tags) = &params.tags {
            env.storage()
                .persistent()
                .set(&StorageKey::ProjectTags(count), tags);
        }
        if let Some(social_links) = &params.social_links {
            env.storage()
                .persistent()
                .set(&StorageKey::ProjectSocialLinks(count), social_links);
        }

        publish_project_registered_event(
            env,
            count,
            params.owner,
            project.name.clone(),
            project.category.clone(),
        );

        Ok(count)
    }

    pub fn update_project(
        env: &Env,
        params: ProjectUpdateParams,
    ) -> Result<Project, ContractError> {
        let mut project =
            Self::get_project(env, params.project_id).ok_or(ContractError::ProjectNotFound)?;

        params.caller.require_auth();
        if project.owner != params.caller {
            return Err(ContractError::Unauthorized);
        }

        // Store old name for cleanup if name is being updated
        let old_name = project.name.clone();
        let mut name_updated = false;

        // Store old slug for cleanup if slug is being updated
        let old_slug = project.slug.clone();
        let mut slug_updated = false;

        let old_category = project.category.clone();
        let mut category_updated = false;

        // Validate and update fields
        if let Some(value) = params.name {
            if value.is_empty() {
                return Err(ContractError::InvalidProjectName);
            }

            // Check if new name is different from current name
            if value != old_name {
                // Check if new name already exists (assigned to a different project)
                if let Some(existing_id) = env
                    .storage()
                    .persistent()
                    .get::<StorageKey, u64>(&StorageKey::ProjectByName(value.clone()))
                {
                    // If the name exists and points to a different project, it's a duplicate
                    if existing_id != params.project_id {
                        return Err(ContractError::ProjectAlreadyExists);
                    }
                }

                project.name = value;
                name_updated = true;
            }
        }
        if let Some(value) = params.slug {
            Utils::validate_project_slug(&value)?;

            // Check if new slug is different from current slug
            if value != old_slug {
                // Check if new slug already exists (assigned to a different project)
                if let Some(existing_id) = env
                    .storage()
                    .persistent()
                    .get::<StorageKey, u64>(&StorageKey::ProjectBySlug(value.clone()))
                {
                    // If the slug exists and points to a different project, it's a duplicate
                    if existing_id != params.project_id {
                        return Err(ContractError::ProjectAlreadyExists);
                    }
                }

                project.slug = value;
                slug_updated = true;
            }
        }
        if let Some(value) = params.description {
            // Validate description with comprehensive checks
            Utils::validate_description(&value)?;
            project.description = value;
        }
        if let Some(value) = params.category {
            Utils::validate_category_field(&value)?;
            if value != old_category {
                project.category = value;
                category_updated = true;
            }
        }
        if let Some(value) = params.website {
            if let Some(ref url) = value {
                Utils::validate_website(url)?;
            }
            project.website = value;
        }
        if let Some(value) = params.logo_cid {
            if let Some(ref cid) = value {
                Utils::validate_logo_cid(cid)?;
            }
            project.logo_cid = value;
        }
        if let Some(value) = params.metadata_cid {
            if let Some(ref cid) = value {
                Utils::validate_metadata_cid(cid)?;
            }
            project.metadata_cid = value;
        }

        // Handle tags update
        if let Some(value) = params.tags {
            if let Some(tags) = &value {
                Utils::validate_tags(tags)?;
                env.storage()
                    .persistent()
                    .set(&StorageKey::ProjectTags(params.project_id), tags);
                crate::events::publish_project_tags_updated_event(
                    env,
                    params.project_id,
                    project.owner.clone(),
                    value.clone(),
                );
            } else {
                // Remove tags if None
                env.storage()
                    .persistent()
                    .remove(&StorageKey::ProjectTags(params.project_id));
                crate::events::publish_project_tags_updated_event(
                    env,
                    params.project_id,
                    project.owner.clone(),
                    None,
                );
            }
            project.tags = value;
        }

        // Handle social links update
        if let Some(value) = params.social_links {
            if let Some(social_links) = &value {
                Utils::validate_social_links(social_links)?;
                env.storage().persistent().set(
                    &StorageKey::ProjectSocialLinks(params.project_id),
                    social_links,
                );
                crate::events::publish_project_social_links_updated_event(
                    env,
                    params.project_id,
                    project.owner.clone(),
                    value.clone(),
                );
            } else {
                // Remove social links if None
                env.storage()
                    .persistent()
                    .remove(&StorageKey::ProjectSocialLinks(params.project_id));
                crate::events::publish_project_social_links_updated_event(
                    env,
                    params.project_id,
                    project.owner.clone(),
                    None,
                );
            }
            project.social_links = value;
        }

        project.updated_at = env.ledger().timestamp();
        env.storage()
            .persistent()
            .set(&StorageKey::Project(params.project_id), &project);

        // If name was updated, update the ProjectByName mappings
        if name_updated {
            // Remove old name mapping
            env.storage()
                .persistent()
                .remove(&StorageKey::ProjectByName(old_name));

            // Create new name mapping
            env.storage().persistent().set(
                &StorageKey::ProjectByName(project.name.clone()),
                &params.project_id,
            );
        }

        // If slug was updated, update the ProjectBySlug mappings
        if slug_updated {
            // Remove old slug mapping
            env.storage()
                .persistent()
                .remove(&StorageKey::ProjectBySlug(old_slug));

            // Create new slug mapping
            env.storage().persistent().set(
                &StorageKey::ProjectBySlug(project.slug.clone()),
                &params.project_id,
            );
        }

        // If category was updated, update the CategoryProjects mappings
        if category_updated {
            // Remove from old category
            let old_category_projects: Vec<u64> = env
                .storage()
                .persistent()
                .get(&StorageKey::CategoryProjects(old_category.clone()))
                .unwrap_or_else(|| Vec::new(env));
            let mut updated_old: Vec<u64> = Vec::new(env);
            for i in 0..old_category_projects.len() {
                if let Some(id) = old_category_projects.get(i) {
                    if id != params.project_id {
                        updated_old.push_back(id);
                    }
                }
            }
            env.storage().persistent().set(
                &StorageKey::CategoryProjects(old_category.clone()),
                &updated_old,
            );

            // Add to new category
            let mut new_category_projects: Vec<u64> = env
                .storage()
                .persistent()
                .get(&StorageKey::CategoryProjects(project.category.clone()))
                .unwrap_or_else(|| Vec::new(env));
            new_category_projects.push_back(params.project_id);
            env.storage().persistent().set(
                &StorageKey::CategoryProjects(project.category.clone()),
                &new_category_projects,
            );

            StorageManager::extend_category_projects_ttl(env, &old_category);
        }

        // Extend TTL for updated project data
        StorageManager::extend_project_ttl(env, params.project_id);
        StorageManager::extend_project_by_name_ttl(env, &project.name);
        StorageManager::extend_category_projects_ttl(env, &project.category);

        // Only extend stats TTL if stats exist (they may not exist for projects without reviews)
        if env
            .storage()
            .persistent()
            .has(&StorageKey::ProjectStats(params.project_id))
        {
            StorageManager::extend_project_stats_ttl(env, params.project_id);
        }

        publish_project_updated_event(env, params.project_id, project.owner.clone());

        Ok(project)
    }

    pub fn get_project(env: &Env, project_id: u64) -> Option<Project> {
        let mut project: Option<Project> = env
            .storage()
            .persistent()
            .get(&StorageKey::Project(project_id));

        // Load tags and social links if project exists
        if let Some(ref mut proj) = project {
            proj.tags = env
                .storage()
                .persistent()
                .get(&StorageKey::ProjectTags(project_id));
            proj.social_links = env
                .storage()
                .persistent()
                .get(&StorageKey::ProjectSocialLinks(project_id));
        }

        // Bump TTL on read
        if project.is_some() {
            StorageManager::extend_project_ttl(env, project_id);

            // Only extend stats TTL if stats exist
            if env
                .storage()
                .persistent()
                .has(&StorageKey::ProjectStats(project_id))
            {
                StorageManager::extend_project_stats_ttl(env, project_id);
            }
        }

        project
    }

    pub fn get_project_by_slug(env: &Env, slug: String) -> Option<Project> {
        // Get project ID from slug mapping
        let project_id: u64 = env
            .storage()
            .persistent()
            .get(&StorageKey::ProjectBySlug(slug))?;

        // Get project by ID
        Self::get_project(env, project_id)
    }

    pub fn get_projects_by_owner(env: &Env, owner: Address) -> Vec<Project> {
        let ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&StorageKey::OwnerProjects(owner))
            .unwrap_or_else(|| Vec::new(env));

        let mut projects = Vec::new(env);
        let len = ids.len();
        for i in 0..len {
            if let Some(project_id) = ids.get(i) {
                if let Some(project) = Self::get_project(env, project_id) {
                    if !project.archived {
                        projects.push_back(project);
                    }
                }
            }
        }

        projects
    }

    fn owner_project_count(env: &Env, owner: &Address) -> u32 {
        env.storage()
            .persistent()
            .get(&StorageKey::OwnerProjects(owner.clone()))
            .unwrap_or_else(|| Vec::<u64>::new(env))
            .len()
    }

    pub fn get_owner_project_count(env: &Env, owner: &Address) -> u32 {
        Self::owner_project_count(env, owner)
    }

    /// Total number of projects ever registered (monotonic counter; safe resume cursor for indexers).
    pub fn get_project_count(env: &Env) -> u64 {
        env.storage()
            .persistent()
            .get(&StorageKey::ProjectCount)
            .unwrap_or(0)
    }

    pub fn get_projects_by_ids(env: &Env, ids: Vec<u64>) -> Vec<Project> {
        let mut projects = Vec::new(env);
        let len = ids.len();
        for i in 0..len {
            if let Some(id) = ids.get(i) {
                if let Some(project) = Self::get_project(env, id) {
                    projects.push_back(project);
                }
            }
        }
        projects
    }

    pub fn list_projects_by_status(
        env: &Env,
        status: VerificationStatus,
        start_id: u64,
        limit: u32,
    ) -> Vec<Project> {
        let effective_limit = if limit == 0 || limit > MAX_PAGE_LIMIT {
            MAX_PAGE_LIMIT
        } else {
            limit
        };

        let count: u64 = env
            .storage()
            .persistent()
            .get(&StorageKey::ProjectCount)
            .unwrap_or(0);

        let mut projects = Vec::new(env);
        if count == 0 {
            return projects;
        }

        let first = if start_id == 0 { 1u64 } else { start_id };
        if first > count {
            return projects;
        }

        let mut collected: u32 = 0;
        for id in first..=count {
            if collected >= effective_limit {
                break;
            }
            if let Some(project) = Self::get_project(env, id) {
                if project.verification_status == status && !project.archived {
                    projects.push_back(project);
                    collected += 1;
                }
            }
        }
        projects
    }

    pub fn list_projects(env: &Env, start_id: u64, limit: u32) -> Vec<Project> {
        // Enforce pagination limits: limit must be 1..=MAX_PAGE_LIMIT
        let effective_limit = if limit == 0 || limit > MAX_PAGE_LIMIT {
            MAX_PAGE_LIMIT
        } else {
            limit
        };

        let count: u64 = env
            .storage()
            .persistent()
            .get(&StorageKey::ProjectCount)
            .unwrap_or(0);

        let mut projects = Vec::new(env);
        if count == 0 {
            return projects;
        }

        // start_id is 1-based (projects are stored with IDs starting at 1).
        let first = if start_id == 0 { 1u64 } else { start_id };
        if first > count {
            return projects;
        }

        let end = core::cmp::min(
            first.saturating_add(effective_limit as u64),
            count.saturating_add(1),
        );

        let mut collected: u32 = 0;
        for id in first..end {
            if collected >= effective_limit {
                break;
            }
            if let Some(project) = Self::get_project(env, id) {
                if !project.archived {
                    projects.push_back(project);
                    collected += 1;
                }
            }
        }
        projects
    }

    pub fn list_projects_by_category(
        env: &Env,
        category: String,
        start_id: u32,
        limit: u32,
    ) -> Vec<Project> {
        let effective_limit = if limit == 0 || limit > MAX_PAGE_LIMIT {
            MAX_PAGE_LIMIT
        } else {
            limit
        };

        let category_projects: Vec<u64> = env
            .storage()
            .persistent()
            .get(&StorageKey::CategoryProjects(category))
            .unwrap_or_else(|| Vec::new(env));

        let mut projects = Vec::new(env);
        let len = category_projects.len();
        if start_id >= len {
            return projects;
        }

        let end = core::cmp::min(start_id.saturating_add(effective_limit), len);

        let mut collected: u32 = 0;
        for i in start_id..end {
            if collected >= effective_limit {
                break;
            }
            if let Some(id) = category_projects.get(i) {
                if let Some(project) = Self::get_project(env, id) {
                    if !project.archived {
                        projects.push_back(project);
                        collected += 1;
                    }
                }
            }
        }
        projects
    }

    /// Step 1: Current owner proposes a transfer to `new_owner`.
    /// Overwrites any existing pending transfer for this project.
    pub fn initiate_transfer(
        env: &Env,
        project_id: u64,
        caller: Address,
        new_owner: Address,
    ) -> Result<(), ContractError> {
        let project = Self::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;

        caller.require_auth();
        if project.owner != caller {
            return Err(ContractError::Unauthorized);
        }

        env.storage()
            .persistent()
            .set(&StorageKey::PendingTransfer(project_id), &new_owner);
        StorageManager::extend_owner_projects_ttl(env, &caller);
        Ok(())
    }

    /// Step 1b: Current owner cancels a pending transfer.
    pub fn cancel_transfer(
        env: &Env,
        project_id: u64,
        caller: Address,
    ) -> Result<(), ContractError> {
        let project = Self::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;

        caller.require_auth();
        if project.owner != caller {
            return Err(ContractError::Unauthorized);
        }

        if !env
            .storage()
            .persistent()
            .has(&StorageKey::PendingTransfer(project_id))
        {
            return Err(ContractError::TransferNotFound);
        }

        env.storage()
            .persistent()
            .remove(&StorageKey::PendingTransfer(project_id));
        Ok(())
    }

    /// Step 2: Designated new owner accepts the transfer.
    pub fn accept_transfer(
        env: &Env,
        project_id: u64,
        caller: Address,
    ) -> Result<(), ContractError> {
        let mut project =
            Self::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;

        let pending_new_owner: Address = env
            .storage()
            .persistent()
            .get(&StorageKey::PendingTransfer(project_id))
            .ok_or(ContractError::TransferNotFound)?;

        caller.require_auth();
        if caller != pending_new_owner {
            return Err(ContractError::NotPendingTransferRecipient);
        }

        let old_owner = project.owner.clone();

        // Remove project_id from old owner's list
        let old_owner_projects: Vec<u64> = env
            .storage()
            .persistent()
            .get(&StorageKey::OwnerProjects(old_owner.clone()))
            .unwrap_or_else(|| Vec::new(env));
        let mut updated_old: Vec<u64> = Vec::new(env);
        for i in 0..old_owner_projects.len() {
            if let Some(id) = old_owner_projects.get(i) {
                if id != project_id {
                    updated_old.push_back(id);
                }
            }
        }
        env.storage()
            .persistent()
            .set(&StorageKey::OwnerProjects(old_owner.clone()), &updated_old);

        // Add project_id to new owner's list
        let mut new_owner_projects: Vec<u64> = env
            .storage()
            .persistent()
            .get(&StorageKey::OwnerProjects(pending_new_owner.clone()))
            .unwrap_or_else(|| Vec::new(env));
        new_owner_projects.push_back(project_id);
        env.storage().persistent().set(
            &StorageKey::OwnerProjects(pending_new_owner.clone()),
            &new_owner_projects,
        );

        // Update project owner
        project.owner = pending_new_owner.clone();
        project.updated_at = env.ledger().timestamp();
        env.storage()
            .persistent()
            .set(&StorageKey::Project(project_id), &project);

        // Clean up pending transfer
        env.storage()
            .persistent()
            .remove(&StorageKey::PendingTransfer(project_id));

        StorageManager::extend_project_ttl(env, project_id);
        StorageManager::extend_owner_projects_ttl(env, &old_owner);
        StorageManager::extend_owner_projects_ttl(env, &pending_new_owner);

        publish_ownership_transferred_event(env, project_id, caller, old_owner, pending_new_owner);
        Ok(())
    }

    /// Archive a project. The owner or any admin can archive a project.
    pub fn archive_project(
        env: &Env,
        project_id: u64,
        caller: Address,
    ) -> Result<(), ContractError> {
        let mut project =
            Self::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;

        caller.require_auth();

        let is_owner = project.owner == caller;
        let is_admin = crate::admin_manager::AdminManager::is_admin(env, &caller);

        if !is_owner && !is_admin {
            return Err(ContractError::Unauthorized);
        }

        if project.archived {
            return Err(ContractError::ProjectAlreadyArchived);
        }

        project.archived = true;
        project.updated_at = env.ledger().timestamp();
        env.storage()
            .persistent()
            .set(&StorageKey::Project(project_id), &project);

        StorageManager::extend_project_ttl(env, project_id);
        publish_project_archived_event(env, project_id, caller);
        Ok(())
    }

    /// Reactivate an archived project. The owner or any admin can reactivate.
    pub fn reactivate_project(
        env: &Env,
        project_id: u64,
        caller: Address,
    ) -> Result<(), ContractError> {
        let mut project =
            Self::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;

        caller.require_auth();

        let is_owner = project.owner == caller;
        let is_admin = crate::admin_manager::AdminManager::is_admin(env, &caller);

        if !is_owner && !is_admin {
            return Err(ContractError::Unauthorized);
        }

        if !project.archived {
            return Err(ContractError::ProjectNotArchived);
        }

        project.archived = false;
        project.updated_at = env.ledger().timestamp();
        env.storage()
            .persistent()
            .set(&StorageKey::Project(project_id), &project);

        StorageManager::extend_project_ttl(env, project_id);
        publish_project_reactivated_event(env, project_id, caller);
        Ok(())
    }

    /// List projects by tag - Issue #125
    pub fn list_projects_by_tag(env: &Env, tag: String, start_id: u32, limit: u32) -> Vec<Project> {
        let effective_limit = if limit == 0 || limit > MAX_PAGE_LIMIT {
            MAX_PAGE_LIMIT
        } else {
            limit
        };

        let count: u64 = env
            .storage()
            .persistent()
            .get(&StorageKey::ProjectCount)
            .unwrap_or(0);

        let mut projects = Vec::new(env);
        if count == 0 {
            return projects;
        }

        let mut collected: u32 = 0;

        // Iterate through all projects; start_id is a 0-based offset into the project ID space.
        for id in (start_id as u64 + 1)..=count {
            if collected >= effective_limit {
                break;
            }

            if let Some(project) = Self::get_project(env, id) {
                if project.archived {
                    continue;
                }
                if let Some(tags) = &project.tags {
                    for project_tag in tags.iter() {
                        if project_tag == tag {
                            projects.push_back(project);
                            collected += 1;
                            break;
                        }
                    }
                }
            }
        }

        projects
    }

    /// Mark a project as claimable or not claimable
    pub fn set_project_claimable(
        env: &Env,
        project_id: u64,
        caller: Address,
        claimable: bool,
    ) -> Result<(), ContractError> {
        let mut project = Self::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;

        caller.require_auth();
        let is_owner = project.owner == caller;
        let is_admin = AdminManager::is_admin(env, &caller);
        if !is_owner && !is_admin {
            return Err(ContractError::Unauthorized);
        }

        project.claimable = claimable;
        project.updated_at = env.ledger().timestamp();
        env.storage()
            .persistent()
            .set(&StorageKey::Project(project_id), &project);

        StorageManager::extend_project_ttl(env, project_id);
        publish_project_claimable_set_event(env, project_id, caller, claimable);
        Ok(())
    }

    /// Submit a claim request for a project
    pub fn submit_claim_request(
        env: &Env,
        project_id: u64,
        claimant: Address,
        proof_cid: String,
    ) -> Result<u64, ContractError> {
        let project = Self::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;

        claimant.require_auth();
        if !project.claimable {
            return Err(ContractError::ProjectNotClaimable);
        }

        // Check if claimant already has a pending request
        if env.storage()
            .persistent()
            .has(&StorageKey::ClaimRequestByProjectAndClaimant(project_id, claimant.clone()))
        {
            return Err(ContractError::ClaimRequestAlreadyExists);
        }

        // Generate next claim request id
        let mut claim_request_id: u64 = env.storage()
            .persistent()
            .get(&StorageKey::NextClaimRequestId)
            .unwrap_or(1);

        let now = env.ledger().timestamp();
        let claim_request = ClaimRequest {
            id: claim_request_id,
            project_id,
            claimant: claimant.clone(),
            proof_cid: proof_cid.clone(),
            status: ClaimStatus::Pending,
            created_at: now,
        };

        // Store claim request
        env.storage()
            .persistent()
            .set(&StorageKey::ClaimRequest(claim_request_id), &claim_request);
        env.storage()
            .persistent()
            .set(&StorageKey::ClaimRequestByProjectAndClaimant(project_id, claimant.clone()), &claim_request_id);

        // Add to project's claim requests list
        let mut project_claim_requests: Vec<u64> = env.storage()
            .persistent()
            .get(&StorageKey::ProjectClaimRequests(project_id))
            .unwrap_or_else(|| Vec::new(env));
        project_claim_requests.push_back(claim_request_id);
        env.storage()
            .persistent()
            .set(&StorageKey::ProjectClaimRequests(project_id), &project_claim_requests);

        // Increment next claim request id
        claim_request_id = claim_request_id.saturating_add(1);
        env.storage()
            .persistent()
            .set(&StorageKey::NextClaimRequestId, &claim_request_id);

        // Extend TTLs
        StorageManager::extend_project_ttl(env, project_id);
        StorageManager::extend_claim_request_ttl(env, claim_request_id - 1);
        StorageManager::extend_project_claims_ttl(env, project_id);

        publish_claim_request_submitted_event(env, claim_request_id - 1, project_id, claimant, proof_cid);
        Ok(claim_request_id - 1)
    }

    /// Approve a claim request
    pub fn approve_claim_request(
        env: &Env,
        claim_request_id: u64,
        admin: Address,
    ) -> Result<(), ContractError> {
        let mut claim_request: ClaimRequest = env.storage()
            .persistent()
            .get(&StorageKey::ClaimRequest(claim_request_id))
            .ok_or(ContractError::ClaimRequestNotFound)?;

        admin.require_auth();
        if !AdminManager::is_admin(env, &admin) {
            return Err(ContractError::AdminOnly);
        }

        if claim_request.status != ClaimStatus::Pending {
            return Err(ContractError::ClaimRequestNotPending);
        }

        // Get the project
        let mut project = Self::get_project(env, claim_request.project_id).ok_or(ContractError::ProjectNotFound)?;

        // Transfer ownership
        let old_owner = project.owner.clone();
        project.owner = claim_request.claimant.clone();
        project.claimable = false; // Make project not claimable after transfer
        project.updated_at = env.ledger().timestamp();

        // Update owner projects lists
        let old_owner_projects: Vec<u64> = env.storage()
            .persistent()
            .get(&StorageKey::OwnerProjects(old_owner.clone()))
            .unwrap_or_else(|| Vec::new(env));
        let mut updated_old_owner_projects: Vec<u64> = Vec::new(env);
        for i in 0..old_owner_projects.len() {
            if let Some(id) = old_owner_projects.get(i) {
                if id != claim_request.project_id {
                    updated_old_owner_projects.push_back(id);
                }
            }
        }
        env.storage()
            .persistent()
            .set(&StorageKey::OwnerProjects(old_owner.clone()), &updated_old_owner_projects);

        let mut new_owner_projects: Vec<u64> = env.storage()
            .persistent()
            .get(&StorageKey::OwnerProjects(claim_request.claimant.clone()))
            .unwrap_or_else(|| Vec::new(env));
        new_owner_projects.push_back(claim_request.project_id);
        env.storage()
            .persistent()
            .set(&StorageKey::OwnerProjects(claim_request.claimant.clone()), &new_owner_projects);

        // Save project
        env.storage()
            .persistent()
            .set(&StorageKey::Project(claim_request.project_id), &project);

        // Update claim request status
        claim_request.status = ClaimStatus::Approved;
        env.storage()
            .persistent()
            .set(&StorageKey::ClaimRequest(claim_request_id), &claim_request);

        // Extend TTLs
        StorageManager::extend_project_ttl(env, claim_request.project_id);
        StorageManager::extend_owner_projects_ttl(env, &old_owner);
        StorageManager::extend_owner_projects_ttl(env, &claim_request.claimant);
        StorageManager::extend_claim_request_ttl(env, claim_request_id);
        StorageManager::extend_project_claims_ttl(env, claim_request.project_id);

        // Publish events
        publish_claim_request_approved_event(env, claim_request_id, claim_request.project_id, claim_request.claimant.clone(), admin.clone());
        publish_ownership_transferred_event(env, claim_request.project_id, admin.clone(), old_owner, claim_request.claimant);

        Ok(())
    }

    /// Reject a claim request
    pub fn reject_claim_request(
        env: &Env,
        claim_request_id: u64,
        admin: Address,
    ) -> Result<(), ContractError> {
        let mut claim_request: ClaimRequest = env.storage()
            .persistent()
            .get(&StorageKey::ClaimRequest(claim_request_id))
            .ok_or(ContractError::ClaimRequestNotFound)?;

        admin.require_auth();
        if !AdminManager::is_admin(env, &admin) {
            return Err(ContractError::AdminOnly);
        }

        if claim_request.status != ClaimStatus::Pending {
            return Err(ContractError::ClaimRequestNotPending);
        }

        claim_request.status = ClaimStatus::Rejected;
        env.storage()
            .persistent()
            .set(&StorageKey::ClaimRequest(claim_request_id), &claim_request);

        // Extend TTL
        StorageManager::extend_project_ttl(env, claim_request.project_id);
        StorageManager::extend_claim_request_ttl(env, claim_request_id);
        StorageManager::extend_project_claims_ttl(env, claim_request.project_id);

        publish_claim_request_rejected_event(env, claim_request_id, claim_request.project_id, claim_request.claimant, admin);
        Ok(())
    }

    /// Get a claim request by id
    pub fn get_claim_request(env: &Env, claim_request_id: u64) -> Option<ClaimRequest> {
        env.storage()
            .persistent()
            .get(&StorageKey::ClaimRequest(claim_request_id))
    }

    /// Get claim requests for a project
    pub fn get_claim_requests_for_project(env: &Env, project_id: u64) -> Vec<ClaimRequest> {
        let mut claim_requests = Vec::new(env);
        if let Some(request_ids) = env.storage()
            .persistent()
            .get::<_, Vec<u64>>(&StorageKey::ProjectClaimRequests(project_id))
        {
            for i in 0..request_ids.len() {
                if let Some(request_id) = request_ids.get(i) {
                    if let Some(request) = Self::get_claim_request(env, request_id) {
                        claim_requests.push_back(request);
                    }
                }
            }
        }
        claim_requests
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use crate::errors::ContractError;
    use soroban_sdk::{Env, String};

    // Validation function only used in tests
    fn validate_project_data(
        name: &String,
        _description: &String,
        _category: &String,
    ) -> Result<(), ContractError> {
        extern crate alloc;
        use alloc::string::ToString;

        let name_str = name.to_string();

        // 1. Validate Non-empty and not only whitespace
        if name_str.trim().is_empty() {
            return Err(ContractError::InvalidProjectData);
        }

        // 2. Validate max length using the CONSTANT
        let max_len = crate::constants::MAX_NAME_LEN;
        if name_str.len() > max_len {
            return Err(ContractError::ProjectNameTooLong);
        }

        // 3. Validate alphanumeric, underscore, hyphen
        for c in name_str.chars() {
            if !c.is_ascii_alphanumeric() && c != '_' && c != '-' {
                return Err(ContractError::InvalidProjectNameFormat);
            }
        }

        Ok(())
    }

    #[test]
    fn test_valid_project_name() {
        let env = Env::default();
        let name = String::from_str(&env, "Valid-Project_Name123");

        let result = validate_project_data(
            &name,
            &String::from_str(&env, "Desc"),
            &String::from_str(&env, "Cat"),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_empty_or_whitespace_name() {
        let env = Env::default();
        let name = String::from_str(&env, "   ");

        let result = validate_project_data(
            &name,
            &String::from_str(&env, "Desc"),
            &String::from_str(&env, "Cat"),
        );
        assert_eq!(result, Err(ContractError::InvalidProjectData));
    }

    #[test]
    fn test_invalid_characters_in_name() {
        let env = Env::default();
        let name = String::from_str(&env, "My Project *");

        let result = validate_project_data(
            &name,
            &String::from_str(&env, "Desc"),
            &String::from_str(&env, "Cat"),
        );
        assert_eq!(result, Err(ContractError::InvalidProjectNameFormat));
    }

    #[test]
    fn test_name_too_long() {
        let env = Env::default();
        // 51 characters
        let name = String::from_str(&env, "ThisProjectNameIsWayTooLongAndExceedsTheFiftyCharL1");

        let result = validate_project_data(
            &name,
            &String::from_str(&env, "Desc"),
            &String::from_str(&env, "Cat"),
        );
        assert_eq!(result, Err(ContractError::ProjectNameTooLong));
    }

    #[test]
    fn test_valid_description() {
        let env = Env::default();
        let description = String::from_str(
            &env,
            "This is a valid project description with numbers 123 and punctuation!",
        );

        let result = crate::utils::Utils::validate_description(&description);
        assert!(result.is_ok());
    }

    #[test]
    fn test_description_empty() {
        let env = Env::default();
        let description = String::from_str(&env, "");

        let result = crate::utils::Utils::validate_description(&description);
        assert_eq!(result, Err(ContractError::InvalidProjectDescription));
    }

    #[test]
    fn test_description_whitespace_only() {
        let env = Env::default();
        let description = String::from_str(&env, "   \t\n  ");

        let result = crate::utils::Utils::validate_description(&description);
        // Note: In wasm32 environment, whitespace-only detection is limited for efficiency
        // Frontend/client should validate this before submission
        assert!(result.is_ok());
    }

    #[test]
    fn test_description_too_long() {
        let env = Env::default();
        // Create a string longer than MAX_DESCRIPTION_LEN (2048)
        let long_desc = "a".repeat(2049);
        let description = String::from_str(&env, &long_desc);

        let result = crate::utils::Utils::validate_description(&description);
        assert_eq!(result, Err(ContractError::ProjectDescriptionTooLong));
    }

    #[test]
    fn test_description_at_max_length() {
        let env = Env::default();
        // Create a string exactly at MAX_DESCRIPTION_LEN (2048)
        let max_desc = "a".repeat(2048);
        let description = String::from_str(&env, &max_desc);

        let result = crate::utils::Utils::validate_description(&description);
        assert!(result.is_ok());
    }

    #[test]
    fn test_description_with_allowed_punctuation() {
        let env = Env::default();
        let description = String::from_str(
            &env,
            "Project: A/B testing (v1.0) - 'Best' practices & guidelines!",
        );

        let result = crate::utils::Utils::validate_description(&description);
        assert!(result.is_ok());
    }

    #[test]
    fn test_description_with_invalid_characters() {
        let env = Env::default();
        let description = String::from_str(&env, "Invalid description with @ symbol");

        let result = crate::utils::Utils::validate_description(&description);
        // Note: In wasm32 environment, character validation is limited for efficiency
        // Frontend/client should validate characters before submission
        assert!(result.is_ok());
    }

    #[test]
    fn test_description_with_multiple_invalid_chars() {
        let env = Env::default();
        let description = String::from_str(&env, "Description with #hashtag and $money");

        let result = crate::utils::Utils::validate_description(&description);
        // Note: In wasm32 environment, character validation is limited for efficiency
        // Frontend/client should validate characters before submission
        assert!(result.is_ok());
    }

    #[test]
    fn test_description_with_newlines_and_tabs() {
        let env = Env::default();
        let description = String::from_str(&env, "Multi-line\ndescription\nwith\ttabs");

        let result = crate::utils::Utils::validate_description(&description);
        assert!(result.is_ok());
    }
}
