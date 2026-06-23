use crate::errors::ContractError;
use crate::storage_keys::StorageKey;
use crate::storage_manager::StorageManager;
use crate::types::{DependencyRef, ProjectDependency};
use crate::utils::Utils;
use soroban_sdk::{Address, Env, String, Vec};

pub struct DependencyRegistry;

impl DependencyRegistry {
    fn normalize_url(url: &String) -> Result<(), ContractError> {
        let len = url.len();
        if len == 0 || len > crate::constants::MAX_SOCIAL_LINK_URL_LEN as u32 {
            return Err(ContractError::InvalidDependencyUrl);
        }
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err(ContractError::InvalidDependencyUrl);
        }
        Ok(())
    }

    fn validate_dependency_ref(env: &Env, dep: &DependencyRef) -> Result<(), ContractError> {
        // Acceptance criteria: dependency references support project IDs and external CIDs/URLs.
        // Reject invalid or duplicate dependencies.
        let has_pid = dep.project_id.is_some();
        let has_cid = dep.external_cid.is_some();
        let has_url = dep.external_url.is_some();

        // Must have at least one and only one (to keep uniqueness simple and deterministic)
        let cnt = (has_pid as u8) + (has_cid as u8) + (has_url as u8);
        if cnt != 1 {
            return Err(ContractError::InvalidDependencyReference);
        }

        if let Some(cid) = &dep.external_cid {
            // use existing CID validation
            Utils::validate_metadata_cid(cid).map_err(|_| ContractError::InvalidDependencyCid)?;
        }

        if let Some(url) = &dep.external_url {
            Self::normalize_url(url)?;
        }

        if let Some(project_id) = dep.project_id {
            if !env
                .storage()
                .persistent()
                .has(&StorageKey::Project(project_id))
            {
                return Err(ContractError::DependencyProjectNotFound);
            }
        }

        Ok(())
    }

    fn dependency_key(env: &Env, dep: &DependencyRef) -> Result<String, ContractError> {
        if let Some(pid) = dep.project_id {
            return Ok(String::from_str(env, &format!("PID:{}", pid)));
        }
        if let Some(cid) = &dep.external_cid {
            return Ok(String::from_str(env, &format!("CID:{}", cid)));
        }
        if let Some(url) = &dep.external_url {
            return Ok(String::from_str(env, &format!("URL:{}", url)));
        }
        Err(ContractError::InvalidDependencyReference)
    }


    pub fn add_dependency(
        env: &Env,
        project_id: u64,
        caller: Address,
        dependency: ProjectDependency,
    ) -> Result<(), ContractError> {
        let project = crate::project_registry::ProjectRegistry::get_project(env, project_id)
            .ok_or(ContractError::ProjectNotFound)?;
        caller.require_auth();
        if project.owner != caller {
            return Err(ContractError::Unauthorized);
        }
        Self::validate_dependency_ref(env, &dependency.reference)?;

        let key = Self::dependency_key(&dependency.reference);

        // Reject duplicates
        if env
            .storage()
            .persistent()
            .has(&StorageKey::ProjectDependency(project_id, key.clone()))
        {
            return Err(ContractError::DuplicateDependency);
        }

        env.storage().persistent().set(
            &StorageKey::ProjectDependency(project_id, key.clone()),
            &dependency,
        );

        // Track key list for fetch.
        let mut keys: Vec<String> = env
            .storage()
            .persistent()
            .get(&StorageKey::ProjectDependencyKeys(project_id))
            .unwrap_or_else(|| Vec::new(env));
        keys.push_back(key);
        env.storage()
            .persistent()
            .set(&StorageKey::ProjectDependencyKeys(project_id), &keys);

        StorageManager::extend_project_dependency_ttl(env, project_id);
        Ok(())
    }

    pub fn update_dependency(
        env: &Env,
        project_id: u64,
        caller: Address,
        dependency_key: DependencyRef,
        new_dependency: ProjectDependency,
    ) -> Result<(), ContractError> {
        let project = crate::project_registry::ProjectRegistry::get_project(env, project_id)
            .ok_or(ContractError::ProjectNotFound)?;
        caller.require_auth();
        if project.owner != caller {
            return Err(ContractError::Unauthorized);
        }

        Self::validate_dependency_ref(env, &dependency_key)?;
        Self::validate_dependency_ref(env, &new_dependency.reference)?;

        let key = Self::dependency_key(&dependency_key);

        if !env
            .storage()
            .persistent()
            .has(&StorageKey::ProjectDependency(project_id, key.clone()))
        {
            return Err(ContractError::DependencyNotFound);
        }

        // Keep same key slot; allow updating metadata.
        let mut stored = new_dependency;
        // Ensure dependency reference doesn't change the identity slot (still uses key)
        stored.reference = dependency_key;

        env.storage().persistent().set(
            &StorageKey::ProjectDependency(project_id, key),
            &stored,
        );

        StorageManager::extend_project_dependency_ttl(env, project_id);
        Ok(())
    }

    pub fn remove_dependency(
        env: &Env,
        project_id: u64,
        caller: Address,
        dependency_key: DependencyRef,
    ) -> Result<(), ContractError> {
        let project = crate::project_registry::ProjectRegistry::get_project(env, project_id)
            .ok_or(ContractError::ProjectNotFound)?;
        caller.require_auth();
        if project.owner != caller {
            return Err(ContractError::Unauthorized);
        }

        Self::validate_dependency_ref(env, &dependency_key)?;
        let key = Self::dependency_key(&dependency_key);

        if !env
            .storage()
            .persistent()
            .has(&StorageKey::ProjectDependency(project_id, key.clone()))
        {
            return Err(ContractError::DependencyNotFound);
        }

        env.storage()
            .persistent()
            .remove(&StorageKey::ProjectDependency(project_id, key.clone()));

        let mut keys: Vec<String> = env
            .storage()
            .persistent()
            .get(&StorageKey::ProjectDependencyKeys(project_id))
            .unwrap_or_else(|| Vec::new(env));
        let mut new_keys: Vec<String> = Vec::new(env);
        for i in 0..keys.len() {
            if let Some(k) = keys.get(i) {
                if k != &key {
                    new_keys.push_back(k);
                }
            }
        }

        env.storage()
            .persistent()
            .set(&StorageKey::ProjectDependencyKeys(project_id), &new_keys);

        StorageManager::extend_project_dependency_ttl(env, project_id);
        Ok(())
    }

    pub fn get_dependencies(env: &Env, project_id: u64) -> Vec<ProjectDependency> {
        let mut out = Vec::new(env);
        let keys: Vec<String> = env
            .storage()
            .persistent()
            .get(&StorageKey::ProjectDependencyKeys(project_id))
            .unwrap_or_else(|| Vec::new(env));

        for i in 0..keys.len() {
            if let Some(k) = keys.get(i) {
                let dep = env
                    .storage()
                    .persistent()
                    .get(&StorageKey::ProjectDependency(project_id, k));
                if let Some(d) = dep {
                    out.push_back(d);
                }
            }
        }
        StorageManager::extend_project_dependency_ttl(env, project_id);
        out
    }
}

