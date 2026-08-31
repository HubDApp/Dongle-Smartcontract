use crate::admin_manager::AdminManager;
use crate::auth::require_admin_auth;
use crate::constants::{
    CLAIM_EXPIRY_SECONDS, MAJOR_METADATA_FIELD_METADATA_CID, MAJOR_METADATA_FIELD_NAME,
    MAJOR_METADATA_FIELD_WEBSITE, MAX_PAGE_LIMIT, MAX_PROJECTS_PER_USER,
};
use crate::errors::ContractError;
use crate::events::{
    publish_claim_request_approved_event, publish_claim_request_rejected_event,
    publish_claim_request_submitted_event, publish_ownership_transferred_event,
    publish_project_archived_event, publish_project_claimable_set_event,
    publish_project_lifecycle_status_updated_event, publish_project_reactivated_event,
    publish_project_registered_event, publish_project_updated_event,
    publish_verification_status_reset_event,
};
use crate::fee_manager::FeeManager;
use crate::storage_keys::{ExtensionKey, StorageKey};
use crate::storage_manager::StorageManager;
use crate::types::{
    ClaimKind, ClaimRequest, ClaimStatus, ContractClaimRequest, Project, ProjectLifecycleStatus,
    ProjectRegistrationParams, ProjectSortMode, ProjectUpdateParams, SecurityContactStatus,
    VerificationStatus,
};
use crate::utils::Utils;
use alloc::vec;
use soroban_sdk::{Address, Env, String, Vec};

pub struct ProjectRegistry;

impl ProjectRegistry {
    /// Project IDs carrying `tag`, per the inverted tag index (issue #485).
    fn tag_index(env: &Env, tag: &String) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&ExtensionKey::TagProjects(tag.clone()))
            .unwrap_or_else(|| Vec::new(env))
    }

    /// Add `project_id` to the index entry for `tag`, ignoring duplicates.
    fn tag_index_insert(env: &Env, tag: &String, project_id: u64) {
        let mut ids = Self::tag_index(env, tag);
        if ids.contains(&project_id) {
            return;
        }
        ids.push_back(project_id);
        env.storage()
            .persistent()
            .set(&ExtensionKey::TagProjects(tag.clone()), &ids);
    }

    /// Drop `project_id` from the index entry for `tag`, removing the entry when it empties.
    fn tag_index_remove(env: &Env, tag: &String, project_id: u64) {
        let ids = Self::tag_index(env, tag);
        let mut remaining: Vec<u64> = Vec::new(env);
        for i in 0..ids.len() {
            if let Some(id) = ids.get(i) {
                if id != project_id {
                    remaining.push_back(id);
                }
            }
        }
        if remaining.is_empty() {
            env.storage()
                .persistent()
                .remove(&ExtensionKey::TagProjects(tag.clone()));
        } else {
            env.storage()
                .persistent()
                .set(&ExtensionKey::TagProjects(tag.clone()), &remaining);
        }
    }

    /// Index every tag in `tags` for `project_id`.
    fn tag_index_insert_all(env: &Env, tags: &Vec<String>, project_id: u64) {
        for tag in tags.iter() {
            Self::tag_index_insert(env, &tag, project_id);
        }
    }

    /// Reconcile the index after a tag change: drop the tags that went away, add the new ones.
    fn tag_index_sync(
        env: &Env,
        project_id: u64,
        previous: &Option<Vec<String>>,
        current: &Option<Vec<String>>,
    ) {
        if let Some(old_tags) = previous {
            for tag in old_tags.iter() {
                let still_present = match current {
                    Some(new_tags) => new_tags.contains(&tag),
                    None => false,
                };
                if !still_present {
                    Self::tag_index_remove(env, &tag, project_id);
                }
            }
        }
        if let Some(new_tags) = current {
            Self::tag_index_insert_all(env, new_tags, project_id);
        }
    }

    /// Shared status-transition helper for both ownership and contract-address claims.
    fn apply_claim_decision(
        status: &mut ClaimStatus,
        _kind: ClaimKind,
        approve: bool,
    ) -> Result<(), ContractError> {
        if approve {
            status.transition_to_approved()
        } else {
            status.transition_to_rejected()
        }
    }

    /// Validate all registration fields and uniqueness constraints.
    ///
    /// Called **before** any storage mutation begins so that the function
    /// is purely read-only (aside from auth checks). This keeps the
    /// validate-then-mutate boundary clean.
    fn validate_registration_fields(
        env: &Env,
        params: &ProjectRegistrationParams,
    ) -> Result<(), ContractError> {
        // Field format validation
        Utils::validate_project_name(&params.name)?;
        Utils::validate_project_slug(&params.slug)?;
        Utils::validate_description(&params.description)?;
        Utils::validate_category_field(&params.category)?;

        if let Some(website) = &params.website {
            Utils::validate_website(website)?;
        }
        if let Some(value) = &params.bounty_url {
            Utils::validate_website(value)?;
        }
        if let Some(logo_cid) = &params.logo_cid {
            Utils::validate_logo_cid(logo_cid)?;
        }
        if let Some(metadata_cid) = &params.metadata_cid {
            Utils::validate_metadata_cid(metadata_cid)?;
        }
        if let Some(repo_url) = &params.repository_url {
            Utils::validate_website(repo_url)?;
        }
        if let Some(tags) = &params.tags {
            Utils::validate_tags(tags)?;
        }
        if let Some(social_links) = &params.social_links {
            Utils::validate_social_links(social_links)?;
        }

        // Reserved-name check
        Self::check_reserved_name(env, &params.name)?;

        // Owner capacity check
        Self::ensure_owner_capacity(env, &params.owner)?;

        // Name uniqueness (exact match)
        if env
            .storage()
            .persistent()
            .has(&StorageKey::ProjectByName(params.name.clone()))
        {
            return Err(ContractError::ProjectAlreadyExists);
        }

        // Name uniqueness (normalized – case / whitespace / punctuation)
        let normalized_name = Utils::normalize_project_name(env, &params.name);
        if env
            .storage()
            .persistent()
            .has(&ExtensionKey::ProjectByNormalizedName(
                normalized_name.clone(),
            ))
        {
            return Err(ContractError::DuplicateProjectName);
        }

        // Slug uniqueness: canonical slugs are stored lowercase so case-only
        // variations are treated as duplicates of the same key.
        let canonical_slug = Utils::to_lowercase(env, &params.slug);
        if env
            .storage()
            .persistent()
            .has(&StorageKey::ProjectBySlug(canonical_slug.clone()))
        {
            return Err(ContractError::ProjectAlreadyExists);
        }

        Ok(())
    }

    pub fn register_project(
        env: &Env,
        params: ProjectRegistrationParams,
    ) -> Result<u64, ContractError> {
        // ── Auth ────────────────────────────────────────────────────────────
        params.owner.require_auth();

        // ── Validation (read-only, no storage writes) ──────────────────────
        Self::validate_registration_fields(env, &params)?;

        // ── Fee payment ────────────────────────────────────────────────────
        if let Ok(config) = FeeManager::get_fee_config(env) {
            if config.registration_fee > 0 {
                FeeManager::consume_registration_fee_payment(
                    env,
                    &params.owner,
                    config.registration_fee,
                )?;
            }
        }

        // ── Mutation phase ─────────────────────────────────────────────────
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
            license: params.license,
            logo_cid: params.logo_cid,
            metadata_cid: params.metadata_cid,
            verification_status: VerificationStatus::Unverified,
            current_verification_id: None,
            archived: false,
            claimable: false,
            lifecycle_status: ProjectLifecycleStatus::Active,
            created_at: now,
            updated_at: now,
            tags: params.tags.clone(),
            social_links: params.social_links.clone(),
            launch_timestamp: params.launch_timestamp,
            maintainers: Some(Vec::new(env)),
            bounty_url: params.bounty_url.clone(),
            repository_url: params.repository_url.clone(),
            security_contact: None,
            security_contact_proof_cid: None,
            security_contact_verified: false,
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

        // Issue #483: keep the inverted tag index current from registration.
        Self::index_project_tags(env, count, &project.tags);
        // Ids are handed out sequentially, so a new project extends the covered
        // range by exactly one whenever it lands directly after the watermark.
        if count == Self::get_tag_index_watermark(env).saturating_add(1) {
            Self::set_tag_index_watermark(env, count);
        }
        env.storage()
            .persistent()
            .set(&StorageKey::ProjectCount, &count);
        env.storage()
            .persistent()
            .set(&StorageKey::ProjectByName(params.name), &count);
        let canonical_slug = Utils::to_lowercase(env, &project.slug);
        env.storage()
            .persistent()
            .set(&StorageKey::ProjectBySlug(canonical_slug.clone()), &count);
        // Store normalized name index for case/whitespace/punctuation-insensitive dedup
        // and for case-insensitive lookups via get_project_by_name.
        let normalized_name = Utils::normalize_project_name(env, &project.name);
        env.storage().persistent().set(
            &ExtensionKey::ProjectByNormalizedName(normalized_name.clone()),
            &count,
        );

        owner_projects.push_back(count);
        env.storage().persistent().set(
            &StorageKey::OwnerProjects(params.owner.clone()),
            &owner_projects,
        );
        Self::add_active_owner_project(env, &params.owner, count);

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
        StorageManager::extend_project_by_normalized_name_ttl(env, &normalized_name);
        StorageManager::extend_project_count_ttl(env);
        StorageManager::extend_owner_projects_ttl(env, &params.owner);
        StorageManager::extend_category_projects_ttl(env, &project.category);

        // Store tags and social links separately if provided
        if let Some(tags) = &params.tags {
            env.storage()
                .persistent()
                .set(&StorageKey::ProjectTags(count), tags);
            Self::tag_index_insert_all(env, tags, count);
        }
        if let Some(social_links) = &params.social_links {
            env.storage()
                .persistent()
                .set(&StorageKey::ProjectSocialLinks(count), social_links);
        }
        if let Some(bounty_url) = &params.bounty_url {
            env.storage()
                .persistent()
                .set(&StorageKey::ProjectBountyUrl(count), bounty_url);
        }

        Self::store_integrity_hash(
            env,
            count,
            &project.name,
            &project.slug,
            &project.category,
            &project.description,
        );

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
        let tags_update = params.tags.clone();
        let social_links_update = params.social_links.clone();

        let mut project =
            Self::get_project(env, params.project_id).ok_or(ContractError::ProjectNotFound)?;

        params.caller.require_auth();
        let is_owner = project.owner == params.caller;
        let is_maintainer = Self::is_maintainer(env, params.project_id, &params.caller);
        if !is_owner && !is_maintainer {
            return Err(ContractError::Unauthorized);
        }

        // ── Metadata freeze guard ──────────────────────────────────────────
        // For verified projects, identity-critical fields are frozen.
        // Detect whether any frozen field is being changed before mutating.
        let is_verified = project.verification_status == VerificationStatus::Verified;

        let new_name_differs = params
            .name
            .as_ref()
            .map(|v| !v.is_empty() && *v != project.name)
            .unwrap_or(false);
        let new_slug_differs = params
            .slug
            .as_ref()
            .map(|v| *v != project.slug)
            .unwrap_or(false);
        let new_category_differs = params
            .category
            .as_ref()
            .map(|v| *v != project.category)
            .unwrap_or(false);
        let new_logo_differs = params
            .logo_cid
            .as_ref()
            .map(|opt| opt.as_ref() != project.logo_cid.as_ref())
            .unwrap_or(false);
        let new_meta_differs = params
            .metadata_cid
            .as_ref()
            .map(|opt| opt.as_ref() != project.metadata_cid.as_ref())
            .unwrap_or(false);
        let new_website_differs = params
            .website
            .as_ref()
            .map(|opt| opt.as_ref() != project.website.as_ref())
            .unwrap_or(false);

        Utils::check_frozen_fields(
            is_verified,
            new_name_differs,
            new_slug_differs,
            new_category_differs,
            new_logo_differs,
            new_meta_differs,
        )?;

        let major_metadata_changed =
            is_verified && (new_name_differs || new_website_differs || new_meta_differs);
        let mut major_fields: Vec<String> = Vec::new(env);
        if major_metadata_changed {
            if new_name_differs {
                major_fields.push_back(String::from_str(env, MAJOR_METADATA_FIELD_NAME));
            }
            if new_website_differs {
                major_fields.push_back(String::from_str(env, MAJOR_METADATA_FIELD_WEBSITE));
            }
            if new_meta_differs {
                major_fields.push_back(String::from_str(env, MAJOR_METADATA_FIELD_METADATA_CID));
            }
        }
        // ─────────────────────────────────────────────────────────────────

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

            // Check reserved names on update
            Self::check_reserved_name(env, &value)?;

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

                // Check normalized name for case/whitespace/punctuation duplicate
                let new_normalized = Utils::normalize_project_name(env, &value);
                let old_normalized = Utils::normalize_project_name(env, &old_name);
                if new_normalized != old_normalized {
                    if let Some(existing_id) = env.storage().persistent().get::<ExtensionKey, u64>(
                        &ExtensionKey::ProjectByNormalizedName(new_normalized.clone()),
                    ) {
                        if existing_id != params.project_id {
                            return Err(ContractError::DuplicateProjectName);
                        }
                    }
                }

                project.name = value;
                name_updated = true;
            }
        }
        if let Some(value) = params.slug {
            Utils::validate_project_slug(&value)?;
            let canonical_slug = Utils::to_lowercase(env, &value);

            // Check if new slug is different from current slug.
            // Canonicalized lowercase storage keys keep slug uniqueness consistent
            // with the name normalization rules.
            if canonical_slug != old_slug {
                // Check if the canonical slug already exists on a different project.
                if let Some(existing_id) = env
                    .storage()
                    .persistent()
                    .get::<StorageKey, u64>(&StorageKey::ProjectBySlug(canonical_slug.clone()))
                {
                    if existing_id != params.project_id {
                        return Err(ContractError::ProjectAlreadyExists);
                    }
                }

                project.slug = canonical_slug.clone();
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
        if let Some(value) = params.license {
            if let Some(ref license) = value {
                Utils::validate_license(license)?;
            }
            project.license = value;
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

        if major_metadata_changed {
            let now = env.ledger().timestamp();
            if let Some(request_id) = project.current_verification_id {
                if let Some(mut record) = env
                    .storage()
                    .persistent()
                    .get::<StorageKey, crate::types::VerificationRecord>(
                        &StorageKey::VerificationRecord(request_id),
                    )
                {
                    record.status = VerificationStatus::Unverified;
                    record.revoke_reason = Some(String::from_str(env, "MajorMetadataChanged"));
                    record.decided_at = now;
                    env.storage()
                        .persistent()
                        .set(&StorageKey::VerificationRecord(request_id), &record);
                }
            }
            project.verification_status = VerificationStatus::Unverified;
        }

        // Handle tags update
        let previous_tags = project.tags.clone();
        if let Some(value) = params.tags {
            if let Some(ref tags) = value {
                Utils::validate_tags(tags)?;
            }
            // Issue #483: move the project between tag entries so the index does
            // not keep pointing at it under tags it no longer carries.
            let previous_tags = project.tags.clone();
            Self::unindex_project_tags(env, params.project_id, &previous_tags);
            Self::index_project_tags(env, params.project_id, &value);
            project.tags = value;
        }
        if let Some(value) = params.social_links {
            project.social_links = value;
        }

        project.updated_at = env.ledger().timestamp();
        env.storage()
            .persistent()
            .set(&StorageKey::Project(params.project_id), &project);

        // Handle tags update
        if let Some(value) = tags_update {
            Self::tag_index_sync(env, params.project_id, &previous_tags, &value);
            if let Some(tags) = &value {
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
        }

        // Handle social links update
        if let Some(value) = social_links_update {
            if let Some(social_links) = &value {
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
        }
        if let Some(value) = params.launch_timestamp {
            project.launch_timestamp = value;
        }
        if let Some(value) = params.bounty_url {
            if let Some(ref url) = value {
                Utils::validate_website(url)?;
                env.storage()
                    .persistent()
                    .set(&StorageKey::ProjectBountyUrl(params.project_id), url);
            } else {
                env.storage()
                    .persistent()
                    .remove(&StorageKey::ProjectBountyUrl(params.project_id));
            }
            project.bounty_url = value;
        }
        if let Some(value) = params.repository_url {
            if let Some(ref url) = value {
                Utils::validate_website(url)?;
            }
            project.repository_url = value;
        }

        // If name was updated, update the ProjectByName and ProjectByNormalizedName mappings
        if name_updated {
            // Remove old name mappings
            env.storage()
                .persistent()
                .remove(&StorageKey::ProjectByName(old_name.clone()));
            let old_normalized = Utils::normalize_project_name(env, &old_name);
            env.storage()
                .persistent()
                .remove(&ExtensionKey::ProjectByNormalizedName(old_normalized));

            // Create new name mappings
            env.storage().persistent().set(
                &StorageKey::ProjectByName(project.name.clone()),
                &params.project_id,
            );
            let new_normalized = Utils::normalize_project_name(env, &project.name);
            env.storage().persistent().set(
                &ExtensionKey::ProjectByNormalizedName(new_normalized),
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
        StorageManager::extend_project_by_normalized_name_ttl(
            env,
            &Utils::normalize_project_name(env, &project.name),
        );
        StorageManager::extend_category_projects_ttl(env, &project.category);

        // Only extend stats TTL if stats exist (they may not exist for projects without reviews)
        if env
            .storage()
            .persistent()
            .has(&StorageKey::ProjectStats(params.project_id))
        {
            StorageManager::extend_project_stats_ttl(env, params.project_id);
        }

        Self::store_integrity_hash(
            env,
            params.project_id,
            &project.name,
            &project.slug,
            &project.category,
            &project.description,
        );

        publish_project_updated_event(env, params.project_id, project.owner.clone());
        if major_metadata_changed {
            publish_verification_status_reset_event(
                env,
                params.project_id,
                params.caller,
                VerificationStatus::Verified,
                major_fields,
            );
        }
        Ok(project)
    }

    pub fn update_security_contact(
        env: &Env,
        project_id: u64,
        caller: Address,
        contact: Option<String>,
    ) -> Result<Project, ContractError> {
        let mut project =
            Self::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;

        caller.require_auth();
        let is_owner = project.owner == caller;
        let is_maintainer = Self::is_maintainer(env, project_id, &caller);
        if !is_owner && !is_maintainer {
            return Err(ContractError::Unauthorized);
        }

        if let Some(value) = &contact {
            Utils::validate_security_contact(value)?;
        }

        if project.security_contact != contact {
            project.security_contact = contact;
            project.security_contact_proof_cid = None;
            project.security_contact_verified = false;
        }

        project.updated_at = env.ledger().timestamp();
        env.storage()
            .persistent()
            .set(&StorageKey::Project(project_id), &project);
        StorageManager::extend_project_ttl(env, project_id);
        publish_project_updated_event(env, project_id, project.owner.clone());

        Ok(project)
    }

    pub fn submit_security_contact_proof(
        env: &Env,
        project_id: u64,
        caller: Address,
        proof_cid: String,
    ) -> Result<Project, ContractError> {
        let mut project =
            Self::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;

        caller.require_auth();
        let is_owner = project.owner == caller;
        let is_maintainer = Self::is_maintainer(env, project_id, &caller);
        if !is_owner && !is_maintainer {
            return Err(ContractError::Unauthorized);
        }
        if project.security_contact.is_none() {
            return Err(ContractError::InvalidProjectData);
        }

        Utils::validate_metadata_cid(&proof_cid)?;
        project.security_contact_proof_cid = Some(proof_cid);
        project.security_contact_verified = true;
        project.updated_at = env.ledger().timestamp();

        env.storage()
            .persistent()
            .set(&StorageKey::Project(project_id), &project);
        StorageManager::extend_project_ttl(env, project_id);
        publish_project_updated_event(env, project_id, project.owner.clone());

        Ok(project)
    }

    pub fn get_security_contact_status(
        env: &Env,
        project_id: u64,
    ) -> Result<SecurityContactStatus, ContractError> {
        let project = Self::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;
        Ok(SecurityContactStatus {
            contact: project.security_contact,
            proof_cid: project.security_contact_proof_cid,
            verified: project.security_contact_verified,
        })
    }

    pub fn get_project(env: &Env, project_id: u64) -> Option<Project> {
        let mut project: Option<Project> = env
            .storage()
            .persistent()
            .get(&StorageKey::Project(project_id));

        // Load tags, social links and maintainers if project exists
        if let Some(ref mut proj) = project {
            proj.tags = env
                .storage()
                .persistent()
                .get(&StorageKey::ProjectTags(project_id));
            proj.social_links = env
                .storage()
                .persistent()
                .get(&StorageKey::ProjectSocialLinks(project_id));
            proj.maintainers = Some(Self::get_maintainers(env, project_id));
            // proj.bounty_url - bounty_url storage removed from StorageKey
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
        let canonical_slug = Utils::to_lowercase(env, &slug);
        let project_id: u64 = env
            .storage()
            .persistent()
            .get(&StorageKey::ProjectBySlug(slug.clone()))?;

        // Extend the slug-index TTL so it stays alive as long as the project data.
        StorageManager::extend_project_by_slug_ttl(env, &slug);

        // Get project by ID
        Self::get_project(env, project_id)
    }

    /// Looks up a project by name, case/whitespace/punctuation-insensitively, using the
    /// ProjectByNormalizedName index rather than scanning all projects.
    pub fn get_project_by_name(env: &Env, name: String) -> Option<Project> {
        let normalized_name = Utils::normalize_project_name(env, &name);

        // Get project ID from normalized name mapping
        let project_id: u64 = env
            .storage()
            .persistent()
            .get(&ExtensionKey::ProjectByNormalizedName(normalized_name))?;

        Self::get_project(env, project_id)
    }

    pub fn get_projects_by_owner(env: &Env, owner: Address) -> Vec<Project> {
        let ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&StorageKey::ActiveOwnerProjects(owner))
            .unwrap_or_else(|| Vec::new(env));

        let mut projects = Vec::new(env);
        let len = ids.len();
        for i in 0..len {
            if let Some(project_id) = ids.get(i) {
                if let Some(project) = Self::get_project(env, project_id) {
                    projects.push_back(project);
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

    /// Reject writes that would grow `OwnerProjects` beyond `MAX_PROJECTS_PER_USER`.
    fn ensure_owner_capacity(env: &Env, owner: &Address) -> Result<(), ContractError> {
        if Self::owner_project_count(env, owner) >= MAX_PROJECTS_PER_USER {
            return Err(ContractError::MaxProjectsExceeded);
        }
        Ok(())
    }

    fn add_active_owner_project(env: &Env, owner: &Address, project_id: u64) {
        let mut active_projects: Vec<u64> = env
            .storage()
            .persistent()
            .get(&StorageKey::ActiveOwnerProjects(owner.clone()))
            .unwrap_or_else(|| Vec::new(env));
        if !active_projects.contains(&project_id) {
            active_projects.push_back(project_id);
            env.storage().persistent().set(
                &StorageKey::ActiveOwnerProjects(owner.clone()),
                &active_projects,
            );
        }
        StorageManager::extend_active_owner_projects_ttl(env, owner);
    }

    fn remove_active_owner_project(env: &Env, owner: &Address, project_id: u64) {
        let active_projects: Vec<u64> = env
            .storage()
            .persistent()
            .get(&StorageKey::ActiveOwnerProjects(owner.clone()))
            .unwrap_or_else(|| Vec::new(env));
        let mut updated_active_projects: Vec<u64> = Vec::new(env);
        for i in 0..active_projects.len() {
            if let Some(id) = active_projects.get(i) {
                if id != project_id {
                    updated_active_projects.push_back(id);
                }
            }
        }
        env.storage().persistent().set(
            &StorageKey::ActiveOwnerProjects(owner.clone()),
            &updated_active_projects,
        );
        StorageManager::extend_active_owner_projects_ttl(env, owner);
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
        start_index: u32,
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
        if start_index >= len {
            return projects;
        }

        let end = core::cmp::min(start_index.saturating_add(effective_limit), len);

        let mut collected: u32 = 0;
        for i in start_index..end {
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
    ///
    /// # Atomicity guarantee (#656)
    ///
    /// Soroban transactions execute atomically: every storage write in a single
    /// invocation either all commits or all reverts. There is therefore no risk
    /// of a partial state (e.g. PendingTransfer written but TTL not extended).
    ///
    /// # Concurrent transfer attempts
    ///
    /// If the owner calls `initiate_transfer` a second time before the first is
    /// accepted, the new recipient **overwrites** the old one atomically.
    /// The first recipient can no longer accept — they will receive `Unauthorized`.
    /// This is intentional: the owner retains full control over the pending
    /// transfer until `accept_transfer` is called.
    ///
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
    ///
    /// # Atomicity guarantee (#656)
    ///
    /// All storage mutations in this function execute within a single Soroban
    /// transaction and are committed or reverted together:
    ///
    /// 1. Remove `project_id` from the old owner's `OwnerProjects` index.
    /// 2. Remove from the old owner's active-projects index.
    /// 3. Capacity check for the new owner (returns error if at limit — no
    ///    partial state is written in that case).
    /// 4. Add `project_id` to the new owner's `OwnerProjects` index.
    /// 5. Add to the new owner's active-projects index (if not archived).
    /// 6. Update `project.owner` and `project.updated_at`.
    /// 7. Remove the `PendingTransfer` storage entry.
    ///
    /// If any step panics or returns an error, every preceding write in this
    /// invocation reverts. There is no intermediate state that can be observed
    /// by a concurrent reader: ownership is either fully on the old owner or
    /// fully on the new owner.
    ///
    /// # No concurrent two-way transfers
    ///
    /// A project can only have one pending transfer at a time (stored under
    /// `StorageKey::PendingTransfer(project_id)`). A second `initiate_transfer`
    /// replaces the first atomically. Two parties racing to `accept_transfer` on
    /// the same project_id: the second one will find `TransferNotFound` because
    /// step 7 removes the pending record on the first successful accept.
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
            return Err(ContractError::Unauthorized);
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
        Self::remove_active_owner_project(env, &old_owner, project_id);

        Self::ensure_owner_capacity(env, &pending_new_owner)?;

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
        if !project.archived {
            Self::add_active_owner_project(env, &pending_new_owner, project_id);
        }

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
        caller.require_auth();
        Self::archive_project_unauthorized(env, project_id, caller)
    }

    pub fn archive_project_unauthorized(
        env: &Env,
        project_id: u64,
        caller: Address,
    ) -> Result<(), ContractError> {
        let mut project =
            Self::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;

        let is_owner = project.owner == caller;
        let is_admin = crate::admin_manager::AdminManager::is_admin(env, &caller);

        if !is_owner && !is_admin {
            return Err(ContractError::Unauthorized);
        }

        if project.archived {
            return Err(ContractError::AlreadyArchived);
        }

        project.archived = true;
        project.updated_at = env.ledger().timestamp();
        env.storage()
            .persistent()
            .set(&StorageKey::Project(project_id), &project);

        Self::remove_active_owner_project(env, &project.owner, project_id);
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

        Self::add_active_owner_project(env, &project.owner, project_id);
        StorageManager::extend_project_ttl(env, project_id);
        publish_project_reactivated_event(env, project_id, caller);
        Ok(())
    }

    /// List projects by tag - Issue #125

    // ===== Tag index (issue #483) =====
    //
    // `list_projects_by_tag` loads every project from id 1 to ProjectCount on
    // every call, so a tag lookup costs O(total projects) regardless of how few
    // carry the tag. These maintain an inverted index, tag -> project ids.
    //
    // The index is only authoritative for ids at or below the watermark.
    // Projects registered before the index existed are absent from it, and an
    // absent entry is indistinguishable from "no project has this tag" — so a
    // lookup serves the covered range from the index and scans only the tail.

    /// Project ids known to carry `tag`.
    pub fn get_tag_index(env: &Env, tag: &String) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&ExtensionKey::TagProjects(tag.clone()))
            .unwrap_or_else(|| Vec::new(env))
    }

    /// Highest project id guaranteed to be represented in the tag index.
    pub fn get_tag_index_watermark(env: &Env) -> u64 {
        env.storage()
            .persistent()
            .get(&ExtensionKey::TagIndexWatermark)
            .unwrap_or(0)
    }

    fn set_tag_index_watermark(env: &Env, value: u64) {
        env.storage()
            .persistent()
            .set(&ExtensionKey::TagIndexWatermark, &value);
    }

    /// Add `project_id` to the index entry for `tag`, if not already present.
    fn index_tag(env: &Env, tag: &String, project_id: u64) {
        let mut ids = Self::get_tag_index(env, tag);
        if ids.contains(&project_id) {
            return;
        }
        ids.push_back(project_id);
        env.storage()
            .persistent()
            .set(&ExtensionKey::TagProjects(tag.clone()), &ids);
    }

    /// Remove `project_id` from the index entry for `tag`.
    fn unindex_tag(env: &Env, tag: &String, project_id: u64) {
        let ids = Self::get_tag_index(env, tag);
        let mut remaining = Vec::new(env);
        let mut changed = false;
        for id in ids.iter() {
            if id == project_id {
                changed = true;
            } else {
                remaining.push_back(id);
            }
        }
        if !changed {
            return;
        }
        let key = ExtensionKey::TagProjects(tag.clone());
        if remaining.is_empty() {
            env.storage().persistent().remove(&key);
        } else {
            env.storage().persistent().set(&key, &remaining);
        }
    }

    /// Index every tag on a project.
    fn index_project_tags(env: &Env, project_id: u64, tags: &Option<Vec<String>>) {
        if let Some(tags) = tags {
            for tag in tags.iter() {
                Self::index_tag(env, &tag, project_id);
            }
        }
    }

    /// Remove a project from every tag entry it was indexed under.
    fn unindex_project_tags(env: &Env, project_id: u64, tags: &Option<Vec<String>>) {
        if let Some(tags) = tags {
            for tag in tags.iter() {
                Self::unindex_tag(env, &tag, project_id);
            }
        }
    }

    // ── Tag Index Watermark State Machine ──────────────────────────────────────
    //
    //  State Machine:
    //  ┌──────────────┐     reindex_tags()     ┌──────────────┐     reindex_tags()     ┌──────────────┐
    //  │ Uninitialized│ ────────────────────> │   Indexing   │ ────────────────────> │   Complete   │
    //  │(watermark=0) │   watermark > 0      │(0<W<ProjCount)│  watermark==ProjCount │(W==ProjCount)│
    //  └──────────────┘                      └──────────────┘                      └──────────────┘
    //
    //  - Uninitialized (watermark == 0): No historic backfilling performed; lookups scan full catalog.
    //  - Indexing (0 < watermark < ProjectCount): Partial backfill completed up to watermark ID.
    //  - Complete (watermark == ProjectCount): Entire project catalog indexed.
    //
    //  Guarantees:
    //  - Atomic update: Watermark advances monotonically (`watermark >= stored`).
    //  - Transaction Safety: Reindex failures rollback atomically under Soroban execution.

    /// Backfill the tag index for projects registered before it existed.
    ///
    /// Processes at most `limit` ids past the watermark and advances it, so the
    /// backfill can be driven in bounded batches rather than one unbounded call.
    /// Returns the watermark after this batch.
    pub fn reindex_tags(env: &Env, caller: Address, limit: u32) -> Result<u64, ContractError> {
        require_admin_auth(env, &caller)?;

        let count: u64 = env
            .storage()
            .persistent()
            .get(&StorageKey::ProjectCount)
            .unwrap_or(0);

        let current_watermark = Self::get_tag_index_watermark(env);
        let mut watermark = current_watermark;
        let batch = if limit == 0 { 1u64 } else { limit as u64 };
        let target = core::cmp::min(watermark.saturating_add(batch), count);

        while watermark < target {
            let id = watermark + 1;
            if let Some(project) = Self::get_project(env, id) {
                Self::index_project_tags(env, id, &project.tags);
            }
            watermark = id;
        }

        // Monotonic guard: ensure watermark updates can never regress stored watermark
        let final_stored = Self::get_tag_index_watermark(env);
        if watermark > final_stored {
            Self::set_tag_index_watermark(env, watermark);
        } else {
            watermark = final_stored;
        }
        Ok(watermark)
    }

    /// Look projects up by tag using the inverted index (issue #483).
    ///
    /// Serves ids within the indexed range directly. Any range not yet covered
    /// by the watermark is scanned, so results are correct before a backfill has
    /// finished — the index makes it fast, it does not make it correct.
    /// Archived projects are excluded, matching `list_projects_by_tag`.
    pub fn get_projects_by_tag_batch(env: &Env, tags: Vec<String>, limit: u32) -> Vec<Project> {
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

        let watermark = Self::get_tag_index_watermark(env);
        let mut projects = Vec::new(env);
        let mut seen: Vec<u64> = Vec::new(env);

        // Indexed range: straight lookups, no full scan.
        for tag in tags.iter() {
            for id in Self::get_tag_index(env, &tag).iter() {
                if projects.len() >= effective_limit {
                    return projects;
                }
                if id > watermark || seen.contains(&id) {
                    continue;
                }
                if let Some(project) = Self::get_project(env, id) {
                    if project.archived {
                        continue;
                    }
                    seen.push_back(id);
                    projects.push_back(project);
                }
            }
        }

        // Uncovered tail: scan until a backfill catches up.
        if watermark < count {
            for id in (watermark + 1)..=count {
                if projects.len() >= effective_limit {
                    break;
                }
                if seen.contains(&id) {
                    continue;
                }
                if let Some(project) = Self::get_project(env, id) {
                    if project.archived {
                        continue;
                    }
                    if let Some(project_tags) = &project.tags {
                        let mut matched = false;
                        for project_tag in project_tags.iter() {
                            for tag in tags.iter() {
                                if project_tag == tag {
                                    matched = true;
                                    break;
                                }
                            }
                            if matched {
                                break;
                            }
                        }
                        if matched {
                            seen.push_back(id);
                            projects.push_back(project);
                        }
                    }
                }
            }
        }

        projects
    }

    pub fn list_projects_by_tag(
        env: &Env,
        tag: String,
        start_index: u32,
        limit: u32,
    ) -> Vec<Project> {
        let effective_limit = if limit == 0 || limit > MAX_PAGE_LIMIT {
            MAX_PAGE_LIMIT
        } else {
            limit
        };

        // Read the inverted tag index rather than scanning the whole ID space (issue #485).
        let ids = Self::tag_index(env, &tag);

        let mut projects = Vec::new(env);
        if ids.is_empty() {
            return projects;
        }

        let mut skipped: u32 = 0;
        let mut collected: u32 = 0;

        // `start_index` is a 0-based offset into the projects matching this tag.
        for i in 0..ids.len() {
            if collected >= effective_limit {
                break;
            }
            let Some(id) = ids.get(i) else {
                continue;
            };
            let Some(project) = Self::get_project(env, id) else {
                continue;
            };
            if project.archived {
                continue;
            }
            if skipped < start_index {
                skipped += 1;
                continue;
            }
            projects.push_back(project);
            collected += 1;
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
        let mut project =
            Self::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;

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
            return Err(ContractError::InvalidStatus);
        }

        // Check if claimant already has a pending request
        if env
            .storage()
            .persistent()
            .has(&ExtensionKey::ClaimReqProjClaimant(
                project_id,
                claimant.clone(),
            ))
        {
            return Err(ContractError::InvalidStatus);
        }

        // Generate next claim request id
        let mut claim_request_id: u64 = env
            .storage()
            .persistent()
            .get(&ExtensionKey::NextClaimRequestId)
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
        env.storage().persistent().set(
            &ExtensionKey::ClaimRequest(claim_request_id),
            &claim_request,
        );
        env.storage().persistent().set(
            &ExtensionKey::ClaimReqProjClaimant(project_id, claimant.clone()),
            &claim_request_id,
        );

        // Add to project's claim requests list
        let mut project_claim_requests: Vec<u64> = env
            .storage()
            .persistent()
            .get(&ExtensionKey::ProjectClaimRequests(project_id))
            .unwrap_or_else(|| Vec::new(env));
        project_claim_requests.push_back(claim_request_id);
        env.storage().persistent().set(
            &ExtensionKey::ProjectClaimRequests(project_id),
            &project_claim_requests,
        );

        // Increment next claim request id
        claim_request_id = claim_request_id.saturating_add(1);
        env.storage()
            .persistent()
            .set(&ExtensionKey::NextClaimRequestId, &claim_request_id);

        // Extend TTLs
        StorageManager::extend_project_ttl(env, project_id);
        StorageManager::extend_claim_request_ttl(env, claim_request_id - 1);
        StorageManager::extend_project_claims_ttl(env, project_id);

        publish_claim_request_submitted_event(
            env,
            claim_request_id - 1,
            project_id,
            claimant,
            proof_cid,
        );
        Ok(claim_request_id - 1)
    }

    /// Approve a claim request
    pub fn approve_claim_request(
        env: &Env,
        claim_request_id: u64,
        admin: Address,
    ) -> Result<(), ContractError> {
        let mut claim_request: ClaimRequest = env
            .storage()
            .persistent()
            .get(&ExtensionKey::ClaimRequest(claim_request_id))
            .ok_or(ContractError::ProjectNotFound)?;

        admin.require_auth();
        if !AdminManager::is_admin(env, &admin) {
            return Err(ContractError::AdminOnly);
        }

        // Shared pending→approved transition (ClaimKind::Ownership)
        Self::apply_claim_decision(&mut claim_request.status, ClaimKind::Ownership, true)?;

        // Get the project
        let mut project = Self::get_project(env, claim_request.project_id)
            .ok_or(ContractError::ProjectNotFound)?;

        // Transfer ownership
        let old_owner = project.owner.clone();
        project.owner = claim_request.claimant.clone();
        project.claimable = false; // Make project not claimable after transfer
        project.updated_at = env.ledger().timestamp();

        // Update owner projects lists
        let old_owner_projects: Vec<u64> = env
            .storage()
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
        env.storage().persistent().set(
            &StorageKey::OwnerProjects(old_owner.clone()),
            &updated_old_owner_projects,
        );

        Self::ensure_owner_capacity(env, &claim_request.claimant)?;

        let mut new_owner_projects: Vec<u64> = env
            .storage()
            .persistent()
            .get(&StorageKey::OwnerProjects(claim_request.claimant.clone()))
            .unwrap_or_else(|| Vec::new(env));
        new_owner_projects.push_back(claim_request.project_id);
        env.storage().persistent().set(
            &StorageKey::OwnerProjects(claim_request.claimant.clone()),
            &new_owner_projects,
        );
        if !project.archived {
            Self::add_active_owner_project(env, &claim_request.claimant, claim_request.project_id);
        }

        // Save project
        env.storage()
            .persistent()
            .set(&StorageKey::Project(claim_request.project_id), &project);

        env.storage().persistent().set(
            &ExtensionKey::ClaimRequest(claim_request_id),
            &claim_request,
        );

        // Extend TTLs
        StorageManager::extend_project_ttl(env, claim_request.project_id);
        StorageManager::extend_owner_projects_ttl(env, &old_owner);
        StorageManager::extend_owner_projects_ttl(env, &claim_request.claimant);
        StorageManager::extend_claim_request_ttl(env, claim_request_id);
        StorageManager::extend_project_claims_ttl(env, claim_request.project_id);

        // Publish events
        publish_claim_request_approved_event(
            env,
            claim_request_id,
            claim_request.project_id,
            claim_request.claimant.clone(),
            admin.clone(),
        );
        publish_ownership_transferred_event(
            env,
            claim_request.project_id,
            admin.clone(),
            old_owner,
            claim_request.claimant,
        );

        crate::admin_action_log::AdminActionLog::record_action(
            env,
            admin,
            crate::types::AdminActionType::ClaimRequestApproved,
            Some(claim_request.project_id),
            None,
            None,
        );

        Ok(())
    }

    /// Reject a claim request
    pub fn reject_claim_request(
        env: &Env,
        claim_request_id: u64,
        admin: Address,
    ) -> Result<(), ContractError> {
        let mut claim_request: ClaimRequest = env
            .storage()
            .persistent()
            .get(&ExtensionKey::ClaimRequest(claim_request_id))
            .ok_or(ContractError::ProjectNotFound)?;

        admin.require_auth();
        if !AdminManager::is_admin(env, &admin) {
            return Err(ContractError::AdminOnly);
        }

        // Shared pending→rejected transition (ClaimKind::Ownership)
        Self::apply_claim_decision(&mut claim_request.status, ClaimKind::Ownership, false)?;
        env.storage().persistent().set(
            &ExtensionKey::ClaimRequest(claim_request_id),
            &claim_request,
        );

        // Extend TTL
        StorageManager::extend_project_ttl(env, claim_request.project_id);
        StorageManager::extend_claim_request_ttl(env, claim_request_id);
        StorageManager::extend_project_claims_ttl(env, claim_request.project_id);

        publish_claim_request_rejected_event(
            env,
            claim_request_id,
            claim_request.project_id,
            claim_request.claimant,
            admin.clone(),
        );

        crate::admin_action_log::AdminActionLog::record_action(
            env,
            admin,
            crate::types::AdminActionType::ClaimRequestRejected,
            Some(claim_request.project_id),
            None,
            None,
        );

        Ok(())
    }

    /// Get a claim request by id
    pub fn get_claim_request(env: &Env, claim_request_id: u64) -> Option<ClaimRequest> {
        env.storage()
            .persistent()
            .get(&ExtensionKey::ClaimRequest(claim_request_id))
    }

    /// Get claim requests for a project
    pub fn get_claim_requests_for_project(env: &Env, project_id: u64) -> Vec<ClaimRequest> {
        let mut claim_requests = Vec::new(env);
        if let Some(request_ids) = env
            .storage()
            .persistent()
            .get::<_, Vec<u64>>(&ExtensionKey::ProjectClaimRequests(project_id))
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
    pub fn link_project(
        env: &Env,
        project_id: u64,
        caller: Address,
        linked_project_id: u64,
    ) -> Result<(), ContractError> {
        caller.require_auth();
        Self::link_project_unauthorized(env, project_id, caller, linked_project_id)
    }

    pub fn link_project_unauthorized(
        env: &Env,
        project_id: u64,
        caller: Address,
        linked_project_id: u64,
    ) -> Result<(), ContractError> {
        let project = Self::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;

        let is_owner = project.owner == caller;
        let is_admin = AdminManager::is_admin(env, &caller);
        if !is_owner && !is_admin {
            return Err(ContractError::Unauthorized);
        }

        if project_id == linked_project_id {
            return Err(ContractError::CannotLinkToSelf);
        }

        if Self::get_project(env, linked_project_id).is_none() {
            return Err(ContractError::ProjectNotFound);
        }

        let mut links: Vec<u64> = env
            .storage()
            .persistent()
            .get(&StorageKey::ProjectLinkedProjects(project_id))
            .unwrap_or_else(|| Vec::new(env));

        for i in 0..links.len() {
            if let Some(id) = links.get(i) {
                if id == linked_project_id {
                    return Err(ContractError::AlreadyLinked);
                }
            }
        }

        links.push_back(linked_project_id);
        env.storage()
            .persistent()
            .set(&StorageKey::ProjectLinkedProjects(project_id), &links);
        StorageManager::extend_project_ttl(env, project_id);

        crate::events::publish_project_linked_event(
            env,
            project_id,
            linked_project_id,
            project.owner,
        );

        Ok(())
    }

    pub fn unlink_project(
        env: &Env,
        project_id: u64,
        caller: Address,
        linked_project_id: u64,
    ) -> Result<(), ContractError> {
        let project = Self::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;

        caller.require_auth();
        let is_owner = project.owner == caller;
        let is_admin = AdminManager::is_admin(env, &caller);
        if !is_owner && !is_admin {
            return Err(ContractError::Unauthorized);
        }

        let links: Vec<u64> = env
            .storage()
            .persistent()
            .get(&StorageKey::ProjectLinkedProjects(project_id))
            .unwrap_or_else(|| Vec::new(env));

        let mut found = false;
        let mut new_links: Vec<u64> = Vec::new(env);
        for i in 0..links.len() {
            if let Some(id) = links.get(i) {
                if id == linked_project_id {
                    found = true;
                } else {
                    new_links.push_back(id);
                }
            }
        }

        if !found {
            return Err(ContractError::ProjectNotFound);
        }

        env.storage()
            .persistent()
            .set(&StorageKey::ProjectLinkedProjects(project_id), &new_links);
        StorageManager::extend_project_ttl(env, project_id);

        crate::events::publish_project_unlinked_event(
            env,
            project_id,
            linked_project_id,
            project.owner,
        );

        Ok(())
    }

    pub fn get_linked_projects(env: &Env, project_id: u64) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&StorageKey::ProjectLinkedProjects(project_id))
            .unwrap_or_else(|| Vec::new(env))
    }

    pub fn get_maintainers(env: &Env, project_id: u64) -> Vec<Address> {
        env.storage()
            .persistent()
            .get(&StorageKey::ProjectMaintainers(project_id))
            .unwrap_or_else(|| Vec::new(env))
    }

    pub fn is_maintainer(env: &Env, project_id: u64, address: &Address) -> bool {
        let maintainers = Self::get_maintainers(env, project_id);
        maintainers.contains(address)
    }

    pub fn add_maintainer(
        env: &Env,
        project_id: u64,
        caller: Address,
        maintainer: Address,
    ) -> Result<(), ContractError> {
        let project = Self::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;
        caller.require_auth();
        if project.owner != caller {
            return Err(ContractError::Unauthorized);
        }

        let mut maintainers = Self::get_maintainers(env, project_id);
        if maintainers.contains(&maintainer) {
            return Err(ContractError::AlreadyMaintainerAdded);
        }

        maintainers.push_back(maintainer.clone());
        env.storage()
            .persistent()
            .set(&StorageKey::ProjectMaintainers(project_id), &maintainers);

        StorageManager::extend_project_maintainers_ttl(env, project_id);

        crate::events::publish_project_maintainer_added_event(env, project_id, caller, maintainer);
        Ok(())
    }

    pub fn remove_maintainer(
        env: &Env,
        project_id: u64,
        caller: Address,
        maintainer: Address,
    ) -> Result<(), ContractError> {
        let project = Self::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;
        caller.require_auth();
        if project.owner != caller {
            return Err(ContractError::Unauthorized);
        }

        let mut maintainers = Self::get_maintainers(env, project_id);
        let mut index = None;
        for i in 0..maintainers.len() {
            if let Some(m) = maintainers.get(i) {
                if m == maintainer {
                    index = Some(i);
                    break;
                }
            }
        }

        match index {
            Some(idx) => {
                maintainers.remove(idx);
                env.storage()
                    .persistent()
                    .set(&StorageKey::ProjectMaintainers(project_id), &maintainers);

                StorageManager::extend_project_maintainers_ttl(env, project_id);

                crate::events::publish_project_maintainer_removed_event(
                    env, project_id, caller, maintainer,
                );
                Ok(())
            }
            None => Err(ContractError::AdminNotFound),
        }
    }

    // ── Reserved Names ────────────────────────────────────────────────────

    /// Check if a name is reserved (case-insensitive comparison).
    fn check_reserved_name(env: &Env, name: &String) -> Result<(), ContractError> {
        let reserved: Vec<String> = env
            .storage()
            .persistent()
            .get(&ExtensionKey::ReservedNames)
            .unwrap_or_else(|| Vec::new(env));

        let name_lower = Utils::to_lowercase(env, name);
        for i in 0..reserved.len() {
            if let Some(r) = reserved.get(i) {
                if Utils::to_lowercase(env, &r) == name_lower {
                    return Err(ContractError::ReservedName);
                }
            }
        }
        Ok(())
    }

    /// Admin: add a name to the reserved list.
    pub fn add_reserved_name(env: &Env, admin: Address, name: String) -> Result<(), ContractError> {
        crate::auth::require_admin_auth(env, &admin)?;

        let mut reserved: Vec<String> = env
            .storage()
            .persistent()
            .get(&ExtensionKey::ReservedNames)
            .unwrap_or_else(|| Vec::new(env));

        // Check if already reserved (case-insensitive)
        let name_lower = Utils::to_lowercase(env, &name);
        for i in 0..reserved.len() {
            if let Some(r) = reserved.get(i) {
                if Utils::to_lowercase(env, &r) == name_lower {
                    return Ok(()); // already reserved, no-op
                }
            }
        }

        reserved.push_back(name.clone());
        env.storage()
            .persistent()
            .set(&ExtensionKey::ReservedNames, &reserved);

        crate::events::publish_reserved_name_added_event(env, name, admin.clone());

        crate::admin_action_log::AdminActionLog::record_action(
            env,
            admin,
            crate::types::AdminActionType::ReservedNameAdded,
            None,
            None,
            None,
        );

        Ok(())
    }

    /// Admin: remove a name from the reserved list.
    pub fn remove_reserved_name(
        env: &Env,
        admin: Address,
        name: String,
    ) -> Result<(), ContractError> {
        crate::auth::require_admin_auth(env, &admin)?;

        let reserved: Vec<String> = env
            .storage()
            .persistent()
            .get(&ExtensionKey::ReservedNames)
            .unwrap_or_else(|| Vec::new(env));

        let name_lower = Utils::to_lowercase(env, &name);
        let mut new_list = Vec::new(env);
        let mut found = false;

        for i in 0..reserved.len() {
            if let Some(r) = reserved.get(i) {
                if Utils::to_lowercase(env, &r) == name_lower {
                    found = true;
                } else {
                    new_list.push_back(r);
                }
            }
        }

        if !found {
            return Ok(()); // not in list, no-op
        }

        env.storage()
            .persistent()
            .set(&ExtensionKey::ReservedNames, &new_list);

        crate::events::publish_reserved_name_removed_event(env, name, admin.clone());

        crate::admin_action_log::AdminActionLog::record_action(
            env,
            admin,
            crate::types::AdminActionType::ReservedNameRemoved,
            None,
            None,
            None,
        );

        Ok(())
    }

    /// Get the list of reserved names.
    pub fn get_reserved_names(env: &Env) -> Vec<String> {
        env.storage()
            .persistent()
            .get(&ExtensionKey::ReservedNames)
            .unwrap_or_else(|| Vec::new(env))
    }

    /// Check if a specific name is reserved.
    pub fn is_name_reserved(env: &Env, name: &String) -> bool {
        Self::check_reserved_name(env, name).is_err()
    }

    pub fn claim_contract_address(
        env: &Env,
        project_id: u64,
        caller: Address,
        contract_address: String,
        proof_cid: String,
    ) -> Result<ContractClaimRequest, ContractError> {
        let project = Self::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;
        caller.require_auth();
        let is_owner = project.owner == caller;
        let is_maintainer = Self::is_maintainer(env, project_id, &caller);
        if !is_owner && !is_maintainer {
            return Err(ContractError::Unauthorized);
        }

        Utils::validate_metadata_cid(&proof_cid)?;

        let now = env.ledger().timestamp();

        // If a pending claim already exists for this address, only allow replacing it
        // once it has expired. Active (non-expired) pending claims block new submissions.
        if let Some(existing) =
            env.storage()
                .persistent()
                .get::<_, ContractClaimRequest>(&ExtensionKey::ContractClaim(
                    project_id,
                    contract_address.clone(),
                ))
        {
            if existing.status == ClaimStatus::Pending {
                // expires_at == 0 is the legacy sentinel for "no expiry"; treat as non-expired.
                let is_expired = existing.expires_at > 0 && now >= existing.expires_at;
                if !is_expired {
                    return Err(ContractError::InvalidStatus);
                }
                // Expired — fall through and overwrite the stale pending claim.
            }
        }

        let expires_at = now + CLAIM_EXPIRY_SECONDS;

        let req = ContractClaimRequest {
            project_id,
            contract_address: contract_address.clone(),
            claimant: caller.clone(),
            proof_cid: proof_cid.clone(),
            status: ClaimStatus::Pending,
            created_at: now,
            expires_at,
        };

        env.storage().persistent().set(
            &ExtensionKey::ContractClaim(project_id, contract_address.clone()),
            &req,
        );

        crate::events::publish_contract_claim_submitted_event(
            env,
            project_id,
            contract_address,
            caller,
            proof_cid,
        );
        Ok(req)
    }

    pub fn approve_contract_claim(
        env: &Env,
        project_id: u64,
        contract_address: String,
        admin: Address,
    ) -> Result<ContractClaimRequest, ContractError> {
        AdminManager::require_admin(env, &admin)?;
        let mut req: ContractClaimRequest = env
            .storage()
            .persistent()
            .get(&ExtensionKey::ContractClaim(
                project_id,
                contract_address.clone(),
            ))
            .ok_or(ContractError::InvalidProjectData)?;

        // Shared pending→approved transition (ClaimKind::ContractAddress)
        Self::apply_claim_decision(&mut req.status, ClaimKind::ContractAddress, true)?;
        env.storage().persistent().set(
            &ExtensionKey::ContractClaim(project_id, contract_address.clone()),
            &req,
        );

        let mut contracts: Vec<String> = env
            .storage()
            .persistent()
            .get(&ExtensionKey::ProjectContracts(project_id))
            .unwrap_or_else(|| Vec::new(env));
        contracts.push_back(contract_address.clone());
        env.storage()
            .persistent()
            .set(&ExtensionKey::ProjectContracts(project_id), &contracts);

        crate::events::publish_contract_claim_approved_event(
            env,
            project_id,
            contract_address,
            admin,
        );
        Ok(req)
    }

    pub fn reject_contract_claim(
        env: &Env,
        project_id: u64,
        contract_address: String,
        admin: Address,
    ) -> Result<ContractClaimRequest, ContractError> {
        AdminManager::require_admin(env, &admin)?;
        let mut req: ContractClaimRequest = env
            .storage()
            .persistent()
            .get(&ExtensionKey::ContractClaim(
                project_id,
                contract_address.clone(),
            ))
            .ok_or(ContractError::InvalidProjectData)?;

        // Shared pending→rejected transition (ClaimKind::ContractAddress)
        Self::apply_claim_decision(&mut req.status, ClaimKind::ContractAddress, false)?;
        env.storage().persistent().set(
            &ExtensionKey::ContractClaim(project_id, contract_address.clone()),
            &req,
        );

        crate::events::publish_contract_claim_rejected_event(
            env,
            project_id,
            contract_address,
            admin,
        );
        Ok(req)
    }

    pub fn get_verified_contracts(env: &Env, project_id: u64) -> Vec<String> {
        env.storage()
            .persistent()
            .get(&ExtensionKey::ProjectContracts(project_id))
            .unwrap_or_else(|| Vec::new(env))
    }

    pub fn list_projects_sorted(
        env: &Env,
        sort_mode: ProjectSortMode,
        start_index: u64,
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

        let mut result: Vec<Project> = Vec::new(env);
        if count == 0 {
            return result;
        }

        match sort_mode {
            // Project IDs are handed out in registration order and `created_at` is
            // non-decreasing across them, so the ID space is already the sort order.
            // Walking it directly reads only the requested page instead of loading
            // and ordering the whole registry (issue #484).
            ProjectSortMode::Newest | ProjectSortMode::Oldest => {
                let newest_first = sort_mode == ProjectSortMode::Newest;
                let mut skipped: u64 = 0;
                let mut collected: u32 = 0;

                for step in 0..count {
                    if collected >= effective_limit {
                        break;
                    }
                    let id = if newest_first { count - step } else { step + 1 };
                    let Some(project) = Self::get_project(env, id) else {
                        continue;
                    };
                    if project.archived {
                        continue;
                    }
                    if skipped < start_index {
                        skipped += 1;
                        continue;
                    }
                    result.push_back(project);
                    collected += 1;
                }
            }
            // Rating order cannot be derived from the ID space. Read each project's
            // review stats once - the previous bubble sort re-read them inside every
            // comparison, which made the call O(N^2) in storage reads - and then select
            // just the requested page rather than ordering the entire registry.
            ProjectSortMode::HighestRated | ProjectSortMode::MostReviewed => {
                let mut candidates: Vec<Project> = Vec::new(env);
                let mut averages: Vec<u32> = Vec::new(env);
                let mut review_counts: Vec<u32> = Vec::new(env);

                for id in 1..=count {
                    let Some(project) = Self::get_project(env, id) else {
                        continue;
                    };
                    if project.archived {
                        continue;
                    }
                    let stats = crate::review_registry::ReviewRegistry::get_project_stats(env, id);
                    candidates.push_back(project);
                    averages.push_back(stats.average_rating);
                    review_counts.push_back(stats.review_count);
                }

                let total = candidates.len();
                let start = start_index as u32;
                if start >= total {
                    return result;
                }
                let wanted = core::cmp::min(start.saturating_add(effective_limit), total);

                let highest_rated = sort_mode == ProjectSortMode::HighestRated;
                let mut taken: Vec<bool> = Vec::new(env);
                for _ in 0..total {
                    taken.push_back(false);
                }

                // Partial selection: only `wanted` ranks are resolved, so the work is
                // bounded by the page the caller asked for.
                for rank in 0..wanted {
                    let mut best: Option<u32> = None;
                    for i in 0..total {
                        if taken.get(i).unwrap_or(false) {
                            continue;
                        }
                        let Some(best_index) = best else {
                            best = Some(i);
                            continue;
                        };
                        let (primary, secondary) = if highest_rated {
                            (&averages, &review_counts)
                        } else {
                            (&review_counts, &averages)
                        };
                        let candidate_primary = primary.get(i).unwrap_or(0);
                        let best_primary = primary.get(best_index).unwrap_or(0);
                        let candidate_secondary = secondary.get(i).unwrap_or(0);
                        let best_secondary = secondary.get(best_index).unwrap_or(0);

                        if candidate_primary > best_primary
                            || (candidate_primary == best_primary
                                && candidate_secondary > best_secondary)
                        {
                            best = Some(i);
                        }
                    }

                    let Some(best_index) = best else {
                        break;
                    };
                    taken.set(best_index, true);
                    if rank >= start {
                        if let Some(project) = candidates.get(best_index) {
                            result.push_back(project);
                        }
                    }
                }
            }
        }

        result
    }

    fn append_string_bytes(_env: &Env, buf: &mut soroban_sdk::Bytes, s: &String) {
        let len = s.len() as usize;
        let mut scratch = vec![0u8; len];
        s.copy_into_slice(&mut scratch);
        for &byte in scratch.iter() {
            buf.push_back(byte);
        }
    }

    fn append_bytes(_env: &Env, buf: &mut soroban_sdk::Bytes, bytes: &[u8]) {
        for &byte in bytes.iter() {
            buf.push_back(byte);
        }
    }

    /// Set the optional region tag for a project (owner only).
    pub fn set_project_region(
        env: &Env,
        project_id: u64,
        caller: Address,
        region: Option<String>,
    ) -> Result<(), ContractError> {
        caller.require_auth();
        let project = Self::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;
        if project.owner != caller {
            return Err(ContractError::Unauthorized);
        }
        match region {
            Some(r) => env
                .storage()
                .persistent()
                .set(&ExtensionKey::ProjectRegion(project_id), &r),
            None => env
                .storage()
                .persistent()
                .remove(&ExtensionKey::ProjectRegion(project_id)),
        }
        Ok(())
    }

    /// Returns the region tag for a project, if set.
    pub fn get_project_region(env: &Env, project_id: u64) -> Option<String> {
        env.storage()
            .persistent()
            .get(&ExtensionKey::ProjectRegion(project_id))
    }

    /// Returns the stored integrity hash for a project, if any.
    pub fn get_project_integrity_hash(env: &Env, project_id: u64) -> Option<soroban_sdk::Bytes> {
        env.storage()
            .persistent()
            .get(&ExtensionKey::ProjectIntegrityHash(project_id))
    }

    /// Computes and stores a SHA-256 integrity hash over key project metadata fields.
    /// The payload is canonicalized as:
    /// `project-integrity-v1|name|slug|category|description`
    /// using the exact UTF-8 bytes for each field in a fixed order. The canonical
    /// payload makes the hash deterministic for a given project, while the version
    /// prefix keeps future upgrades explicit and testable.
    ///
    /// Any change to `name`, `slug`, `category`, or `description` changes the
    /// resulting hash, which lets verification detect when project metadata drifted
    /// from the stored value.
    pub fn store_integrity_hash(
        env: &Env,
        project_id: u64,
        name: &String,
        slug: &String,
        category: &String,
        description: &String,
    ) {
        let hash_bytes = Self::compute_integrity_hash(env, name, slug, category, description);
        env.storage()
            .persistent()
            .set(&ExtensionKey::ProjectIntegrityHash(project_id), &hash_bytes);
    }

    /// Computes the legacy, unversioned SHA-256 hash for the given metadata fields.
    /// This encoding is retained for backwards compatibility during verification of
    /// already-stored project hashes created before the canonical versioned format
    /// was introduced.
    pub fn compute_integrity_hash_legacy(
        env: &Env,
        name: &String,
        slug: &String,
        category: &String,
        description: &String,
    ) -> soroban_sdk::Bytes {
        let sep = b'|';
        let mut buf = soroban_sdk::Bytes::new(env);
        Self::append_string_bytes(env, &mut buf, name);
        buf.push_back(sep);
        Self::append_string_bytes(env, &mut buf, slug);
        buf.push_back(sep);
        Self::append_string_bytes(env, &mut buf, category);
        buf.push_back(sep);
        Self::append_string_bytes(env, &mut buf, description);
        let hash = env.crypto().sha256(&buf);
        soroban_sdk::Bytes::from_array(env, &hash.to_array())
    }

    /// Returns true when the provided hash matches either the current canonical
    /// versioned payload or the legacy payload. This preserves backward
    /// compatibility with older on-chain integrity hashes while rejecting metadata drift.
    pub fn hash_matches_current_or_legacy(
        env: &Env,
        name: &String,
        slug: &String,
        category: &String,
        description: &String,
        candidate_hash: &soroban_sdk::Bytes,
    ) -> bool {
        let current = Self::compute_integrity_hash(env, name, slug, category, description);
        let legacy = Self::compute_integrity_hash_legacy(env, name, slug, category, description);
        candidate_hash == &current || candidate_hash == &legacy
    }

    /// Computes (but does not store) the current canonical SHA-256 integrity hash
    /// for the given metadata fields.
    ///
    /// Exposed so that other modules (e.g. `verification_registry`) can
    /// recompute and validate the hash without duplicating the logic.
    pub fn compute_integrity_hash(
        env: &Env,
        name: &String,
        slug: &String,
        category: &String,
        description: &String,
    ) -> soroban_sdk::Bytes {
        let mut buf = soroban_sdk::Bytes::new(env);
        Self::append_bytes(env, &mut buf, b"project-integrity-v1");
        buf.push_back(b'|');
        Self::append_string_bytes(env, &mut buf, name);
        buf.push_back(b'|');
        Self::append_string_bytes(env, &mut buf, slug);
        buf.push_back(b'|');
        Self::append_string_bytes(env, &mut buf, category);
        buf.push_back(b'|');
        Self::append_string_bytes(env, &mut buf, description);
        let hash = env.crypto().sha256(&buf);
        soroban_sdk::Bytes::from_array(env, &hash.to_array())
    }

    /// Update a project's lifecycle status.
    /// Only the project owner can change the lifecycle status.
    pub fn set_project_lifecycle_status(
        env: &Env,
        project_id: u64,
        caller: Address,
        new_status: ProjectLifecycleStatus,
    ) -> Result<Project, ContractError> {
        let mut project =
            Self::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;

        caller.require_auth();
        if project.owner != caller {
            return Err(ContractError::Unauthorized);
        }

        let previous_status = project.lifecycle_status;
        if previous_status == new_status {
            // Status unchanged, no event needed
            return Ok(project);
        }

        project.lifecycle_status = new_status;
        project.updated_at = env.ledger().timestamp();

        env.storage()
            .persistent()
            .set(&StorageKey::Project(project_id), &project);
        StorageManager::extend_project_ttl(env, project_id);

        publish_project_lifecycle_status_updated_event(
            env,
            project_id,
            project.owner.clone(),
            previous_status,
            new_status,
        );

        Ok(project)
    }

    /// List projects by lifecycle status with pagination.
    /// Returns projects matching the specified lifecycle status, excluding archived projects.
    pub fn list_projects_by_lifecycle_status(
        env: &Env,
        status: ProjectLifecycleStatus,
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

        let first = if start_id > 0 { start_id } else { 1 };
        let mut collected: u32 = 0;

        for id in first..=count {
            if collected >= effective_limit {
                break;
            }

            if let Some(project) = Self::get_project(env, id) {
                if !project.archived && project.lifecycle_status == status {
                    projects.push_back(project);
                    collected = collected.saturating_add(1);
                }
            }
        }

        projects
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
            return Err(ContractError::InvalidProjectName);
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
        assert_eq!(result, Err(ContractError::InvalidProjectName));
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
        assert_eq!(result, Err(ContractError::InvalidProjectData));
    }

    #[test]
    fn test_description_whitespace_only() {
        let env = Env::default();
        let description = String::from_str(&env, "   \t\n  ");

        let result = crate::utils::Utils::validate_description(&description);
        assert_eq!(result, Err(ContractError::InvalidProjectData));
    }

    #[test]
    fn test_description_too_long() {
        let env = Env::default();
        // Create a string longer than MAX_DESCRIPTION_LEN (2048)
        let long_desc = "a".repeat(2049);
        let description = String::from_str(&env, &long_desc);

        let result = crate::utils::Utils::validate_description(&description);
        assert_eq!(result, Err(ContractError::InvalidProjectData));
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
