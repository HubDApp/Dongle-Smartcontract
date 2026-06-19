use crate::admin_action_log::AdminActionLog;
use crate::auth::require_admin_auth;
use crate::constants::{
    MAX_COLLECTIONS, MAX_COLLECTION_DESCRIPTION_LEN, MAX_COLLECTION_NAME_LEN,
    MAX_PROJECTS_PER_COLLECTION,
};
use crate::errors::ContractError;
use crate::events::{
    publish_collection_created_event, publish_collection_deleted_event,
    publish_collection_updated_event, publish_project_added_to_collection_event,
    publish_project_removed_from_collection_event,
};
use crate::storage_keys::StorageKey;
use crate::types::{AdminActionType, Collection};
use soroban_sdk::{Address, Env, String, Vec};

pub struct CollectionRegistry;

impl CollectionRegistry {
    pub fn create_collection(
        env: &Env,
        admin: Address,
        name: String,
        description: String,
    ) -> Result<u64, ContractError> {
        require_admin_auth(env, &admin)?;

        Self::validate_name(&name)?;
        Self::validate_description(&description)?;
        Self::ensure_name_unique(env, &name, None)?;

        let total = Self::get_collection_count(env);
        if total >= MAX_COLLECTIONS.into() {
            return Err(ContractError::MaxProjectsExceeded);
        }

        let id = Self::get_next_id(env);
        let timestamp = env.ledger().timestamp();
        let collection = Collection {
            id,
            name: name.clone(),
            description,
            created_at: timestamp,
            updated_at: timestamp,
        };

        env.storage()
            .persistent()
            .set(&StorageKey::Collection(id), &collection);
        env.storage()
            .persistent()
            .set(&StorageKey::CollectionNameById(id), &name);
        env.storage()
            .persistent()
            .set(&StorageKey::CollectionProjectIds(id), &Vec::<u64>::new(env));

        let mut list: Vec<u64> = env
            .storage()
            .persistent()
            .get(&StorageKey::CollectionList)
            .unwrap_or(Vec::new(env));
        list.push_back(id);
        env.storage()
            .persistent()
            .set(&StorageKey::CollectionList, &list);

        env.storage()
            .persistent()
            .set(&StorageKey::NextCollectionId, &(id + 1));

        publish_collection_created_event(env, id, name, admin.clone());

        AdminActionLog::record_action(
            env,
            admin,
            AdminActionType::CollectionCreated,
            Some(id),
            None,
            None,
        );

        Ok(id)
    }

    pub fn update_collection(
        env: &Env,
        admin: Address,
        collection_id: u64,
        name: String,
        description: String,
    ) -> Result<(), ContractError> {
        require_admin_auth(env, &admin)?;

        let mut collection = Self::require_collection(env, collection_id)?;

        Self::validate_name(&name)?;
        Self::validate_description(&description)?;

        if collection.name != name {
            Self::ensure_name_unique(env, &name, Some(collection_id))?;
        }

        collection.name = name;
        collection.description = description;
        collection.updated_at = env.ledger().timestamp();

        env.storage()
            .persistent()
            .set(&StorageKey::Collection(collection_id), &collection);
        env.storage().persistent().set(
            &StorageKey::CollectionNameById(collection_id),
            &collection.name,
        );

        publish_collection_updated_event(env, collection_id, admin.clone());

        AdminActionLog::record_action(
            env,
            admin,
            AdminActionType::CollectionUpdated,
            Some(collection_id),
            None,
            None,
        );

        Ok(())
    }

    pub fn delete_collection(
        env: &Env,
        admin: Address,
        collection_id: u64,
    ) -> Result<(), ContractError> {
        require_admin_auth(env, &admin)?;

        Self::require_collection(env, collection_id)?;

        let list: Vec<u64> = env
            .storage()
            .persistent()
            .get(&StorageKey::CollectionList)
            .unwrap_or(Vec::new(env));
        let mut updated = Vec::new(env);
        for id in list.iter() {
            if id != collection_id {
                updated.push_back(id);
            }
        }
        env.storage()
            .persistent()
            .set(&StorageKey::CollectionList, &updated);

        env.storage()
            .persistent()
            .remove(&StorageKey::Collection(collection_id));
        env.storage()
            .persistent()
            .remove(&StorageKey::CollectionNameById(collection_id));
        env.storage()
            .persistent()
            .remove(&StorageKey::CollectionProjectIds(collection_id));

        publish_collection_deleted_event(env, collection_id, admin.clone());

        AdminActionLog::record_action(
            env,
            admin,
            AdminActionType::CollectionDeleted,
            Some(collection_id),
            None,
            None,
        );

        Ok(())
    }

    pub fn add_project_to_collection(
        env: &Env,
        admin: Address,
        collection_id: u64,
        project_id: u64,
    ) -> Result<(), ContractError> {
        require_admin_auth(env, &admin)?;

        Self::require_collection(env, collection_id)?;

        if !env
            .storage()
            .persistent()
            .has(&StorageKey::Project(project_id))
        {
            return Err(ContractError::ProjectNotFound);
        }

        let mut project_ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&StorageKey::CollectionProjectIds(collection_id))
            .unwrap_or(Vec::new(env));

        if project_ids.iter().any(|id| id == project_id) {
            return Err(ContractError::AlreadyInCollection);
        }

        if project_ids.len() >= MAX_PROJECTS_PER_COLLECTION {
            return Err(ContractError::TooManyTags);
        }

        project_ids.push_back(project_id);
        env.storage().persistent().set(
            &StorageKey::CollectionProjectIds(collection_id),
            &project_ids,
        );

        publish_project_added_to_collection_event(env, collection_id, project_id, admin.clone());

        AdminActionLog::record_action(
            env,
            admin,
            AdminActionType::ProjectAddedToCollection,
            Some(collection_id),
            None,
            None,
        );

        Ok(())
    }

    pub fn remove_project_from_collection(
        env: &Env,
        admin: Address,
        collection_id: u64,
        project_id: u64,
    ) -> Result<(), ContractError> {
        require_admin_auth(env, &admin)?;

        Self::require_collection(env, collection_id)?;

        let project_ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&StorageKey::CollectionProjectIds(collection_id))
            .unwrap_or(Vec::new(env));

        if !project_ids.iter().any(|id| id == project_id) {
            return Err(ContractError::NotInCollection);
        }

        let mut updated = Vec::new(env);
        for id in project_ids.iter() {
            if id != project_id {
                updated.push_back(id);
            }
        }
        env.storage()
            .persistent()
            .set(&StorageKey::CollectionProjectIds(collection_id), &updated);

        publish_project_removed_from_collection_event(
            env,
            collection_id,
            project_id,
            admin.clone(),
        );

        AdminActionLog::record_action(
            env,
            admin,
            AdminActionType::ProjectRemovedFromCollection,
            Some(collection_id),
            None,
            None,
        );

        Ok(())
    }

    pub fn get_collection(env: &Env, collection_id: u64) -> Result<Collection, ContractError> {
        env.storage()
            .persistent()
            .get(&StorageKey::Collection(collection_id))
            .ok_or(ContractError::CollectionNotFound)
    }

    pub fn list_collections(env: &Env, start: u32, limit: u32) -> Vec<Collection> {
        let ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&StorageKey::CollectionList)
            .unwrap_or(Vec::new(env));

        let limit = limit.min(100);
        let mut result = Vec::new(env);
        let mut count = 0u32;

        for (i, collection_id) in ids.iter().enumerate() {
            if (i as u32) < start {
                continue;
            }
            if count >= limit {
                break;
            }
            if let Some(collection) = env
                .storage()
                .persistent()
                .get::<_, Collection>(&StorageKey::Collection(collection_id))
            {
                result.push_back(collection);
                count += 1;
            }
        }

        result
    }

    pub fn list_collection_projects(
        env: &Env,
        collection_id: u64,
        start: u32,
        limit: u32,
    ) -> Vec<u64> {
        let ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&StorageKey::CollectionProjectIds(collection_id))
            .unwrap_or(Vec::new(env));

        let limit = limit.min(100);
        let mut result = Vec::new(env);
        let mut count = 0u32;

        for (i, project_id) in ids.iter().enumerate() {
            if (i as u32) < start {
                continue;
            }
            if count >= limit {
                break;
            }
            result.push_back(project_id);
            count += 1;
        }

        result
    }

    pub fn get_collection_project_count(env: &Env, collection_id: u64) -> u32 {
        let ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&StorageKey::CollectionProjectIds(collection_id))
            .unwrap_or(Vec::new(env));
        ids.len()
    }

    pub fn get_collection_count(env: &Env) -> u64 {
        let ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&StorageKey::CollectionList)
            .unwrap_or(Vec::new(env));
        ids.len().into()
    }

    // ── Internal Helpers ──────────────────────────────────────────────────

    fn validate_name(name: &String) -> Result<(), ContractError> {
        let len = name.len();
        if len == 0 {
            return Err(ContractError::InvalidProjectData);
        }
        if len as usize > MAX_COLLECTION_NAME_LEN {
            return Err(ContractError::ProjectNameTooLong);
        }
        Ok(())
    }

    fn validate_description(description: &String) -> Result<(), ContractError> {
        let len = description.len();
        if len == 0 {
            return Err(ContractError::InvalidProjectData);
        }
        if len as usize > MAX_COLLECTION_DESCRIPTION_LEN {
            return Err(ContractError::ProjectDescriptionTooLong);
        }
        Ok(())
    }

    fn ensure_name_unique(
        env: &Env,
        name: &String,
        exclude_id: Option<u64>,
    ) -> Result<(), ContractError> {
        let ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&StorageKey::CollectionList)
            .unwrap_or(Vec::new(env));

        for id in ids.iter() {
            if let Some(exclude) = exclude_id {
                if id == exclude {
                    continue;
                }
            }
            if let Some(existing_name) = env
                .storage()
                .persistent()
                .get::<_, String>(&StorageKey::CollectionNameById(id))
            {
                if existing_name == *name {
                    return Err(ContractError::CollectionAlreadyExists);
                }
            }
        }
        Ok(())
    }

    fn require_collection(env: &Env, collection_id: u64) -> Result<Collection, ContractError> {
        env.storage()
            .persistent()
            .get(&StorageKey::Collection(collection_id))
            .ok_or(ContractError::CollectionNotFound)
    }

    fn get_next_id(env: &Env) -> u64 {
        env.storage()
            .persistent()
            .get(&StorageKey::NextCollectionId)
            .unwrap_or(1u64)
    }
}
