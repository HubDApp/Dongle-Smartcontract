//! Admin project deprecation and dependent alerts (issue #925).
//!
//! `deprecate_project` marks a project as deprecated, stores a
//! [`DeprecationRecord`] and emits a `DependencyDeprecated` event for each
//! project that depends on it, using the reverse dependency index maintained
//! by `DependencyRegistry`. To keep a single call bounded, at most
//! [`MAX_DEPRECATION_NOTIFY_BATCH`] dependents are alerted per call; the rest
//! are alerted page by page through `notify_deprecation_dependents`.

use crate::admin_action_log::AdminActionLog;
use crate::auth::require_admin_auth;
use crate::constants::{MAX_DEPRECATION_NOTIFY_BATCH, MAX_DEPRECATION_REASON_LEN};
use crate::dependency_registry::DependencyRegistry;
use crate::errors::ContractError;
use crate::events::{publish_dependency_deprecated_event, publish_project_deprecated_event};
use crate::pagination::paginate;
use crate::project_registry::ProjectRegistry;
use crate::storage_keys::{DeprecationKey, StorageKey};
use crate::types::{AdminActionType, DeprecationRecord, ProjectLifecycleStatus};
use crate::utils::Utils;
use soroban_sdk::{Address, Env, String, Vec};

pub struct DeprecationRegistry;

impl DeprecationRegistry {
    /// Deprecate a project and alert its direct dependents. Admin-only.
    ///
    /// Sets `Project::deprecated` / `deprecation_reason`, moves the project's
    /// lifecycle status to `Deprecated`, and emits a `DependencyDeprecated`
    /// event for the first `MAX_DEPRECATION_NOTIFY_BATCH` dependents.
    pub fn deprecate_project(
        env: &Env,
        admin: Address,
        project_id: u64,
        reason: String,
        migration_guide_cid: Option<String>,
    ) -> Result<DeprecationRecord, ContractError> {
        require_admin_auth(env, &admin)?;

        let mut project =
            ProjectRegistry::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;
        if project.deprecated {
            return Err(ContractError::InvalidStatus);
        }
        if reason.is_empty() || reason.len() > MAX_DEPRECATION_REASON_LEN {
            return Err(ContractError::InvalidInput);
        }
        if let Some(cid) = &migration_guide_cid {
            Utils::validate_metadata_cid(cid).map_err(|_| ContractError::InvalidCid)?;
        }

        let now = env.ledger().timestamp();
        project.deprecated = true;
        project.deprecation_reason = reason.clone();
        project.lifecycle_status = ProjectLifecycleStatus::Deprecated;
        project.updated_at = now;
        env.storage()
            .persistent()
            .set(&StorageKey::Project(project_id), &project);

        let dependents = DependencyRegistry::get_dependent_projects(env, project_id);
        let record = DeprecationRecord {
            project_id,
            reason: reason.clone(),
            migration_guide_cid: migration_guide_cid.clone(),
            deprecated_by: admin.clone(),
            deprecated_at: now,
            dependent_count: dependents.len(),
        };
        env.storage()
            .persistent()
            .set(&DeprecationKey::Record(project_id), &record);

        let mut deprecated: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DeprecationKey::DeprecatedProjects)
            .unwrap_or_else(|| Vec::new(env));
        deprecated.push_back(project_id);
        env.storage()
            .persistent()
            .set(&DeprecationKey::DeprecatedProjects, &deprecated);

        publish_project_deprecated_event(
            env,
            project_id,
            admin.clone(),
            reason,
            migration_guide_cid,
            record.dependent_count,
        );
        Self::alert_dependents(env, &record, &dependents, 0, MAX_DEPRECATION_NOTIFY_BATCH);

        AdminActionLog::record_action(
            env,
            admin,
            AdminActionType::ProjectDeprecated,
            Some(project_id),
            None,
            record.migration_guide_cid.clone(),
        );

        Ok(record)
    }

    /// Emit `DependencyDeprecated` alerts for one page of a deprecated
    /// project's dependents. Admin-only.
    ///
    /// Used to alert dependents beyond the first batch handled by
    /// `deprecate_project`, or to re-send alerts. `limit` is clamped to
    /// `MAX_DEPRECATION_NOTIFY_BATCH`. Returns the number of alerts emitted;
    /// `0` means `start_index` is past the end of the dependent list.
    pub fn notify_deprecation_dependents(
        env: &Env,
        admin: Address,
        deprecated_project_id: u64,
        start_index: u32,
        limit: u32,
    ) -> Result<u32, ContractError> {
        require_admin_auth(env, &admin)?;
        let record = Self::get_deprecation_record(env, deprecated_project_id)
            .ok_or(ContractError::InvalidStatus)?;
        let dependents = DependencyRegistry::get_dependent_projects(env, deprecated_project_id);
        Ok(Self::alert_dependents(
            env,
            &record,
            &dependents,
            start_index,
            limit.min(MAX_DEPRECATION_NOTIFY_BATCH),
        ))
    }

    /// Projects that directly depend on a deprecated project. Returns an
    /// empty list when the project is not deprecated.
    pub fn get_projects_depending_on(env: &Env, deprecated_project_id: u64) -> Vec<u64> {
        if !env
            .storage()
            .persistent()
            .has(&DeprecationKey::Record(deprecated_project_id))
        {
            return Vec::new(env);
        }
        DependencyRegistry::get_dependent_projects(env, deprecated_project_id)
    }

    pub fn get_deprecation_record(env: &Env, project_id: u64) -> Option<DeprecationRecord> {
        env.storage()
            .persistent()
            .get(&DeprecationKey::Record(project_id))
    }

    /// Deprecated project IDs in deprecation order, paginated.
    pub fn list_deprecated_projects(env: &Env, start_index: u32, limit: u32) -> Vec<u64> {
        let ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DeprecationKey::DeprecatedProjects)
            .unwrap_or_else(|| Vec::new(env));
        paginate(env, &ids, start_index, limit)
    }

    fn alert_dependents(
        env: &Env,
        record: &DeprecationRecord,
        dependents: &Vec<u64>,
        start_index: u32,
        limit: u32,
    ) -> u32 {
        let mut emitted = 0;
        let end = start_index.saturating_add(limit).min(dependents.len());
        for i in start_index..end {
            if let Some(dependent_id) = dependents.get(i) {
                publish_dependency_deprecated_event(
                    env,
                    record.project_id,
                    dependent_id,
                    record.reason.clone(),
                    record.migration_guide_cid.clone(),
                );
                emitted += 1;
            }
        }
        emitted
    }
}
