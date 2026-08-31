use crate::errors::ContractError;
use crate::events::{publish_project_endorsed_event, publish_project_unendorsed_event};
use crate::project_registry::ProjectRegistry;
use crate::storage_keys::ExtensionKey;
use crate::storage_manager::StorageManager;
use crate::constants::MAX_PAGE_LIMIT;
use crate::pagination::paginate;
use soroban_sdk::{Address, Env, Vec};

pub struct EndorsementRegistry;

impl EndorsementRegistry {
    pub fn endorse_project(env: &Env, project_id: u64, user: Address) -> Result<(), ContractError> {
        user.require_auth();

        ProjectRegistry::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;

        Self::ensure_indexed(env, project_id);
        if Self::has_endorsed(env, project_id, &user) {
            return Err(ContractError::AlreadyEndorsed);
        }

        let count = Self::current_count(env, project_id);
        env.storage().persistent().set(
            &ExtensionKey::EndorsementAt(project_id, count),
            &user,
        );
        env.storage().persistent().set(
            &ExtensionKey::EndorsementIndex(project_id, user.clone()),
            &count,
        );
        let new_count = count + 1;
        env.storage()
            .persistent()
            .set(&ExtensionKey::EndorsementCount(project_id), &new_count);
        StorageManager::extend_endorsement_entry_ttl(env, project_id, &user, count);

        StorageManager::extend_project_ttl(env, project_id);
        StorageManager::extend_endorsements_ttl(env, project_id);

        publish_project_endorsed_event(env, project_id, user);
        Ok(())
    }

    pub fn unendorse_project(
        env: &Env,
        project_id: u64,
        user: Address,
    ) -> Result<(), ContractError> {
        user.require_auth();

        ProjectRegistry::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;

        Self::ensure_indexed(env, project_id);
        if !Self::has_endorsed(env, project_id, &user) {
            return Err(ContractError::NotEndorsed);
        }

        let index: u32 = env
            .storage()
            .persistent()
            .get(&ExtensionKey::EndorsementIndex(project_id, user.clone()))
            .expect("endorsed user must have an index");
        let count = Self::current_count(env, project_id);
        let last_index = count - 1;
        if index != last_index {
            let last_user: Address = env
                .storage()
                .persistent()
                .get(&ExtensionKey::EndorsementAt(project_id, last_index))
                .expect("endorsement index must be populated");
            env.storage().persistent().set(
                &ExtensionKey::EndorsementAt(project_id, index),
                &last_user,
            );
            env.storage().persistent().set(
                &ExtensionKey::EndorsementIndex(project_id, last_user.clone()),
                &index,
            );
            StorageManager::extend_endorsement_entry_ttl(env, project_id, &last_user, index);
        }
        env.storage()
            .persistent()
            .remove(&ExtensionKey::EndorsementAt(project_id, last_index));
        env.storage()
            .persistent()
            .remove(&ExtensionKey::EndorsementIndex(project_id, user.clone()));
        let new_count = last_index;
        env.storage()
            .persistent()
            .set(&ExtensionKey::EndorsementCount(project_id), &new_count);

        StorageManager::extend_project_ttl(env, project_id);
        StorageManager::extend_endorsements_ttl(env, project_id);

        publish_project_unendorsed_event(env, project_id, user);
        Ok(())
    }

    pub fn get_endorsement_count(env: &Env, project_id: u64) -> u32 {
        Self::current_count(env, project_id)
    }

    pub fn get_project_endorsements(
        env: &Env,
        project_id: u64,
        start_index: u32,
        limit: u32,
    ) -> Vec<Address> {
        let count = Self::current_count(env, project_id);
        let effective_limit = if limit == 0 {
            MAX_PAGE_LIMIT
        } else {
            limit.min(MAX_PAGE_LIMIT)
        };
        let mut result = Vec::new(env);
        let end = start_index.saturating_add(effective_limit).min(count);
        let mut index = start_index;
        while index < end {
            if let Some(user) = env
                .storage()
                .persistent()
                .get(&ExtensionKey::EndorsementAt(project_id, index))
            {
                result.push_back(user);
            }
            index += 1;
        }
        if result.is_empty() && count > 0 && start_index < count {
            let legacy: Vec<Address> = env
                .storage()
                .persistent()
                .get(&ExtensionKey::ProjectEndorsements(project_id))
                .unwrap_or_else(|| Vec::new(env));
            return paginate(env, &legacy, start_index, effective_limit);
        }
        result
    }

    pub fn has_endorsed(env: &Env, project_id: u64, user: &Address) -> bool {
        env.storage()
            .persistent()
            .has(&ExtensionKey::EndorsementIndex(project_id, user.clone()))
            || env
                .storage()
                .persistent()
                .get::<_, Vec<Address>>(&ExtensionKey::ProjectEndorsements(project_id))
                .map(|endorsements| endorsements.contains(user))
                .unwrap_or(false)
    }

    fn current_count(env: &Env, project_id: u64) -> u32 {
        env.storage()
            .persistent()
            .get(&ExtensionKey::EndorsementCount(project_id))
            .or_else(|| {
                env.storage()
                    .persistent()
                    .get::<_, Vec<Address>>(&ExtensionKey::ProjectEndorsements(project_id))
                    .map(|endorsements| endorsements.len())
            })
            .unwrap_or(0)
    }

    fn ensure_indexed(env: &Env, project_id: u64) {
        if Self::current_count(env, project_id) == 0
            || env
                .storage()
                .persistent()
                .has(&ExtensionKey::EndorsementAt(project_id, 0))
        {
            return;
        }

        let legacy: Vec<Address> = env
            .storage()
            .persistent()
            .get(&ExtensionKey::ProjectEndorsements(project_id))
            .unwrap_or_else(|| Vec::new(env));
        for index in 0..legacy.len() {
            let user = legacy.get(index).expect("legacy endorsement index");
            env.storage().persistent().set(
                &ExtensionKey::EndorsementAt(project_id, index),
                &user,
            );
            env.storage().persistent().set(
                &ExtensionKey::EndorsementIndex(project_id, user),
                &index,
            );
        }
    }
}
