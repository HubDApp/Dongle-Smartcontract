use crate::errors::ContractError;
use crate::storage_keys::{ExtensionKey, StorageKey};
use crate::storage_manager::StorageManager;
use crate::types::{DependencyRef, ProjectDependency};
use crate::utils::Utils;
use soroban_sdk::{Address, Env, String, Vec};

pub struct DependencyRegistry;

impl DependencyRegistry {
    fn normalize_url(url: &String) -> Result<(), ContractError> {
        let len = url.len();
        if len == 0 || len > crate::constants::MAX_SOCIAL_LINK_URL_LEN as u32 {
            return Err(ContractError::InvalidWebsite);
        }
        let mut buf = [0u8; crate::constants::MAX_SOCIAL_LINK_URL_LEN];
        let slice = &mut buf[..len as usize];
        url.copy_into_slice(slice);
        if !slice.starts_with(b"http://") && !slice.starts_with(b"https://") {
            return Err(ContractError::InvalidWebsite);
        }
        Ok(())
    }

    /// Validate a Stellar contract address: 56 uppercase base32 chars starting with 'C'.
    fn validate_contract_address(addr: &String) -> Result<(), ContractError> {
        let len = addr.len();
        if len != 56 {
            return Err(ContractError::InvalidProjectData);
        }
        let mut buf = [0u8; 56];
        addr.copy_into_slice(&mut buf);
        if buf[0] != b'C' {
            return Err(ContractError::InvalidProjectData);
        }
        for &c in buf.iter() {
            if !(c.is_ascii_uppercase() || (c >= b'2' && c <= b'7')) {
                return Err(ContractError::InvalidProjectData);
            }
        }
        Ok(())
    }

    fn validate_dependency_ref(env: &Env, dep: &DependencyRef) -> Result<(), ContractError> {
        let has_pid = dep.project_id.is_some();
        let has_cid = dep.external_cid.is_some();
        let has_url = dep.external_url.is_some();
        let has_contract = dep.external_contract.is_some();

        // Exactly one reference kind must be set.
        let cnt = (has_pid as u8) + (has_cid as u8) + (has_url as u8) + (has_contract as u8);
        if cnt != 1 {
            return Err(ContractError::InvalidProjectData);
        }

        if let Some(cid) = &dep.external_cid {
            Utils::validate_metadata_cid(cid).map_err(|_| ContractError::InvalidLogoCid)?;
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
                return Err(ContractError::ProjectNotFound);
            }
        }

        if let Some(contract) = &dep.external_contract {
            Self::validate_contract_address(contract)?;
        }

        Ok(())
    }

    fn dependency_key(env: &Env, dep: &DependencyRef) -> Result<String, ContractError> {
        if let Some(pid) = dep.project_id {
            let mut num_buf = [0u8; 20];
            let mut val = pid;
            let mut idx = 20;
            if val == 0 {
                idx -= 1;
                num_buf[idx] = b'0';
            } else {
                while val > 0 {
                    idx -= 1;
                    num_buf[idx] = b'0' + (val % 10) as u8;
                    val /= 10;
                }
            }
            let num_len = 20 - idx;
            let mut buf = [0u8; 24];
            buf[0..4].copy_from_slice(b"PID:");
            buf[4..4 + num_len].copy_from_slice(&num_buf[idx..20]);
            let key_str = core::str::from_utf8(&buf[..4 + num_len]).unwrap();
            return Ok(String::from_str(env, key_str));
        }
        if let Some(cid) = &dep.external_cid {
            let cid_len = cid.len();
            if cid_len > crate::constants::MAX_CID_LEN as u32 {
                return Err(ContractError::InvalidProjectData);
            }
            let mut buf = [0u8; 4 + 128]; // "CID:" (4) + max cid (128)
            buf[0..4].copy_from_slice(b"CID:");
            cid.copy_into_slice(&mut buf[4..4 + cid_len as usize]);
            let key_str = core::str::from_utf8(&buf[..4 + cid_len as usize]).unwrap();
            return Ok(String::from_str(env, key_str));
        }
        if let Some(url) = &dep.external_url {
            let url_len = url.len();
            if url_len > crate::constants::MAX_SOCIAL_LINK_URL_LEN as u32 {
                return Err(ContractError::InvalidProjectData);
            }
            let mut buf = [0u8; 4 + 256]; // "URL:" (4) + max url (256)
            buf[0..4].copy_from_slice(b"URL:");
            url.copy_into_slice(&mut buf[4..4 + url_len as usize]);
            let key_str = core::str::from_utf8(&buf[..4 + url_len as usize]).unwrap();
            return Ok(String::from_str(env, key_str));
        }
        if let Some(contract) = &dep.external_contract {
            // Contract addresses are always 56 chars; "CTR:" prefix ensures no collision.
            let mut buf = [0u8; 4 + 56];
            buf[0..4].copy_from_slice(b"CTR:");
            contract.copy_into_slice(&mut buf[4..60]);
            let key_str = core::str::from_utf8(&buf[..60]).unwrap();
            return Ok(String::from_str(env, key_str));
        }
        Err(ContractError::InvalidProjectData)
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

        let key = Self::dependency_key(env, &dependency.reference)?;

        // Reject duplicates
        if env
            .storage()
            .persistent()
            .has(&ExtensionKey::ProjectDependency(project_id, key.clone()))
        {
            return Err(ContractError::AlreadyLinked);
        }

        env.storage().persistent().set(
            &ExtensionKey::ProjectDependency(project_id, key.clone()),
            &dependency,
        );

        // Track key list for fetch.
        let mut keys: Vec<String> = env
            .storage()
            .persistent()
            .get(&ExtensionKey::ProjectDependencyKeys(project_id))
            .unwrap_or_else(|| Vec::new(env));
        keys.push_back(key);
        env.storage()
            .persistent()
            .set(&ExtensionKey::ProjectDependencyKeys(project_id), &keys);

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

        let key = Self::dependency_key(env, &dependency_key)?;

        if !env
            .storage()
            .persistent()
            .has(&ExtensionKey::ProjectDependency(project_id, key.clone()))
        {
            return Err(ContractError::ProjectNotFound);
        }

        // Keep same key slot; allow updating metadata.
        let mut stored = new_dependency;
        // Ensure dependency reference doesn't change the identity slot (still uses key)
        stored.reference = dependency_key;

        env.storage()
            .persistent()
            .set(&ExtensionKey::ProjectDependency(project_id, key), &stored);

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
        let key = Self::dependency_key(env, &dependency_key)?;

        if !env
            .storage()
            .persistent()
            .has(&ExtensionKey::ProjectDependency(project_id, key.clone()))
        {
            return Err(ContractError::ProjectNotFound);
        }

        env.storage()
            .persistent()
            .remove(&ExtensionKey::ProjectDependency(project_id, key.clone()));

        let mut keys: Vec<String> = env
            .storage()
            .persistent()
            .get(&ExtensionKey::ProjectDependencyKeys(project_id))
            .unwrap_or_else(|| Vec::new(env));
        let mut new_keys: Vec<String> = Vec::new(env);
        for i in 0..keys.len() {
            if let Some(k) = keys.get(i) {
                if k != key {
                    new_keys.push_back(k);
                }
            }
        }

        env.storage()
            .persistent()
            .set(&ExtensionKey::ProjectDependencyKeys(project_id), &new_keys);

        StorageManager::extend_project_dependency_ttl(env, project_id);
        Ok(())
    }

    pub fn get_dependencies(env: &Env, project_id: u64) -> Vec<ProjectDependency> {
        let mut out = Vec::new(env);
        let keys: Vec<String> = env
            .storage()
            .persistent()
            .get(&ExtensionKey::ProjectDependencyKeys(project_id))
            .unwrap_or_else(|| Vec::new(env));

        for i in 0..keys.len() {
            if let Some(k) = keys.get(i) {
                let dep = env
                    .storage()
                    .persistent()
                    .get(&ExtensionKey::ProjectDependency(project_id, k));
                if let Some(d) = dep {
                    out.push_back(d);
                }
            }
        }
        StorageManager::extend_project_dependency_ttl(env, project_id);
        out
    }
}
