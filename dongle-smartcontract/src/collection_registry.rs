use crate::admin_action_log::AdminActionLog;
use crate::admin_manager::AdminManager;
use crate::auth::require_admin_auth;
use crate::constants::{
    MAX_COLLECTIONS, MAX_COLLECTION_DESCRIPTION_LEN, MAX_COLLECTION_NAME_LEN,
    MAX_PROJECTS_PER_COLLECTION,
};
use crate::errors::ContractError;
use crate::events::{
    publish_collection_created_event, publish_collection_deleted_event,
    publish_collection_share_link_generated_event, publish_collection_share_link_revoked_event,
    publish_collection_updated_event, publish_collection_visibility_toggled_event,
    publish_project_added_to_collection_event, publish_project_removed_from_collection_event,
};
use crate::pagination::paginate;
use crate::storage_keys::{CollectionVisibilityKey, StorageKey};
use crate::types::{AdminActionType, Collection};
use crate::utils::Utils;
use soroban_sdk::{Address, Bytes, Env, String, Vec};

pub struct CollectionRegistry;

impl CollectionRegistry {
    /// Create a collection (admin-curated, public by default).
    pub fn create_collection(
        env: &Env,
        admin: Address,
        name: String,
        description: String,
    ) -> Result<u64, ContractError> {
        require_admin_auth(env, &admin)?;
        Self::create_collection_internal(env, admin, name, description, true)
    }

    /// Create a collection with explicit visibility (admin-only).
    pub fn create_collection_with_visibility(
        env: &Env,
        admin: Address,
        name: String,
        description: String,
        is_public: bool,
    ) -> Result<u64, ContractError> {
        require_admin_auth(env, &admin)?;
        Self::create_collection_internal(env, admin, name, description, is_public)
    }

    /// Create a collection owned by the authenticated caller.
    pub fn create_user_collection(
        env: &Env,
        creator: Address,
        name: String,
        description: String,
        is_public: bool,
    ) -> Result<u64, ContractError> {
        creator.require_auth();
        Self::create_collection_internal(env, creator, name, description, is_public)
    }

    fn create_collection_internal(
        env: &Env,
        creator: Address,
        name: String,
        description: String,
        is_public: bool,
    ) -> Result<u64, ContractError> {
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
            owner: creator.clone(),
            name: name.clone(),
            description,
            is_public,
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
        env.storage()
            .persistent()
            .set(&CollectionVisibilityKey::CollectionOwner(id), &creator);
        env.storage()
            .persistent()
            .set(&CollectionVisibilityKey::CollectionIsPublic(id), &is_public);

        let mut list: Vec<u64> = env
            .storage()
            .persistent()
            .get(&StorageKey::CollectionList)
            .unwrap_or_else(|| Vec::new(env));
        list.push_back(id);
        env.storage()
            .persistent()
            .set(&StorageKey::CollectionList, &list);

        let mut user_list: Vec<u64> = env
            .storage()
            .persistent()
            .get(&CollectionVisibilityKey::UserCollections(creator.clone()))
            .unwrap_or_else(|| Vec::new(env));
        user_list.push_back(id);
        env.storage()
            .persistent()
            .set(&CollectionVisibilityKey::UserCollections(creator.clone()), &user_list);

        if is_public {
            let mut pub_list: Vec<u64> = env
                .storage()
                .persistent()
                .get(&CollectionVisibilityKey::PublicCollectionList)
                .unwrap_or_else(|| Vec::new(env));
            pub_list.push_back(id);
            env.storage()
                .persistent()
                .set(&CollectionVisibilityKey::PublicCollectionList, &pub_list);
        }

        env.storage()
            .persistent()
            .set(&StorageKey::NextCollectionId, &(id + 1));

        publish_collection_created_event(env, id, name, creator.clone());

        if AdminManager::is_admin(env, &creator) {
            AdminActionLog::record_action(
                env,
                creator,
                AdminActionType::CollectionCreated,
                Some(id),
                None,
                None,
            );
        }

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

        let collection = Self::require_collection(env, collection_id)?;

        let project_ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&StorageKey::CollectionProjectIds(collection_id))
            .unwrap_or_else(|| Vec::new(env));
        for project_id in project_ids.iter() {
            publish_project_removed_from_collection_event(
                env,
                collection_id,
                project_id,
                admin.clone(),
            );
        }

        let list: Vec<u64> = env
            .storage()
            .persistent()
            .get(&StorageKey::CollectionList)
            .unwrap_or_else(|| Vec::new(env));
        let updated = Utils::remove_item_from_vec(env, &list, &collection_id);
        env.storage()
            .persistent()
            .set(&StorageKey::CollectionList, &updated);

        // Remove from public collection list if present
        let pub_list: Vec<u64> = env
            .storage()
            .persistent()
            .get(&CollectionVisibilityKey::PublicCollectionList)
            .unwrap_or_else(|| Vec::new(env));
        let updated_pub = Utils::remove_item_from_vec(env, &pub_list, &collection_id);
        env.storage()
            .persistent()
            .set(&CollectionVisibilityKey::PublicCollectionList, &updated_pub);

        // Remove from user collections list
        let user_list: Vec<u64> = env
            .storage()
            .persistent()
            .get(&CollectionVisibilityKey::UserCollections(collection.owner.clone()))
            .unwrap_or_else(|| Vec::new(env));
        let updated_user = Utils::remove_item_from_vec(env, &user_list, &collection_id);
        env.storage()
            .persistent()
            .set(&CollectionVisibilityKey::UserCollections(collection.owner.clone()), &updated_user);

        env.storage()
            .persistent()
            .remove(&StorageKey::Collection(collection_id));
        env.storage()
            .persistent()
            .remove(&StorageKey::CollectionNameById(collection_id));
        env.storage()
            .persistent()
            .remove(&StorageKey::CollectionProjectIds(collection_id));
        env.storage()
            .persistent()
            .remove(&CollectionVisibilityKey::CollectionOwner(collection_id));
        env.storage()
            .persistent()
            .remove(&CollectionVisibilityKey::CollectionIsPublic(collection_id));
        env.storage()
            .persistent()
            .remove(&CollectionVisibilityKey::CollectionShareToken(collection_id));

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

    // ── Visibility & Privacy Controls (Issue #819 AC1, AC2, AC3) ──────────

    /// Toggle visibility between public and private.
    /// Only the owner or an admin may toggle visibility.
    pub fn toggle_collection_visibility(
        env: &Env,
        caller: Address,
        collection_id: u64,
    ) -> Result<bool, ContractError> {
        caller.require_auth();
        let mut collection = Self::require_collection(env, collection_id)?;
        if collection.owner != caller && !AdminManager::is_admin(env, &caller) {
            return Err(ContractError::NotCollectionOwner);
        }

        let new_visibility = !collection.is_public;
        collection.is_public = new_visibility;
        collection.updated_at = env.ledger().timestamp();

        env.storage()
            .persistent()
            .set(&StorageKey::Collection(collection_id), &collection);
        env.storage()
            .persistent()
            .set(&CollectionVisibilityKey::CollectionIsPublic(collection_id), &new_visibility);

        // Update PublicCollectionList
        let pub_list: Vec<u64> = env
            .storage()
            .persistent()
            .get(&CollectionVisibilityKey::PublicCollectionList)
            .unwrap_or_else(|| Vec::new(env));
        if new_visibility {
            let mut updated = pub_list;
            if !updated.iter().any(|x| x == collection_id) {
                updated.push_back(collection_id);
            }
            env.storage()
                .persistent()
                .set(&CollectionVisibilityKey::PublicCollectionList, &updated);
        } else {
            let updated = Utils::remove_item_from_vec(env, &pub_list, &collection_id);
            env.storage()
                .persistent()
                .set(&CollectionVisibilityKey::PublicCollectionList, &updated);
        }

        publish_collection_visibility_toggled_event(env, collection_id, new_visibility, caller);
        Ok(new_visibility)
    }

    /// Set explicit visibility for a collection.
    pub fn set_collection_visibility(
        env: &Env,
        caller: Address,
        collection_id: u64,
        is_public: bool,
    ) -> Result<(), ContractError> {
        caller.require_auth();
        let mut collection = Self::require_collection(env, collection_id)?;
        if collection.owner != caller && !AdminManager::is_admin(env, &caller) {
            return Err(ContractError::NotCollectionOwner);
        }

        if collection.is_public == is_public {
            return Ok(());
        }

        collection.is_public = is_public;
        collection.updated_at = env.ledger().timestamp();

        env.storage()
            .persistent()
            .set(&StorageKey::Collection(collection_id), &collection);
        env.storage()
            .persistent()
            .set(&CollectionVisibilityKey::CollectionIsPublic(collection_id), &is_public);

        let pub_list: Vec<u64> = env
            .storage()
            .persistent()
            .get(&CollectionVisibilityKey::PublicCollectionList)
            .unwrap_or_else(|| Vec::new(env));
        if is_public {
            let mut updated = pub_list;
            if !updated.iter().any(|x| x == collection_id) {
                updated.push_back(collection_id);
            }
            env.storage()
                .persistent()
                .set(&CollectionVisibilityKey::PublicCollectionList, &updated);
        } else {
            let updated = Utils::remove_item_from_vec(env, &pub_list, &collection_id);
            env.storage()
                .persistent()
                .set(&CollectionVisibilityKey::PublicCollectionList, &updated);
        }

        publish_collection_visibility_toggled_event(env, collection_id, is_public, caller);
        Ok(())
    }

    /// Retrieve a collection with public visibility guard.
    /// Returns `Some(Collection)` if public, `None` if private.
    pub fn get_collection(env: &Env, collection_id: u64) -> Option<Collection> {
        let collection: Option<Collection> = env
            .storage()
            .persistent()
            .get(&StorageKey::Collection(collection_id));
        match collection {
            Some(col) if col.is_public => Some(col),
            _ => None,
        }
    }

    /// Retrieve a collection with caller authorization.
    /// Public collections are visible to anyone. Private collections are
    /// only returned if `caller` is the owner or an admin.
    pub fn get_collection_for_caller(
        env: &Env,
        caller: Address,
        collection_id: u64,
    ) -> Result<Collection, ContractError> {
        caller.require_auth();
        let collection = Self::require_collection(env, collection_id)?;
        if collection.is_public || collection.owner == caller || AdminManager::is_admin(env, &caller) {
            Ok(collection)
        } else {
            Err(ContractError::CollectionPrivate)
        }
    }

    /// List all public collections with pagination (AC2 - searchable/discoverable).
    pub fn list_collections(env: &Env, start_index: u32, limit: u32) -> Vec<Collection> {
        let ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&CollectionVisibilityKey::PublicCollectionList)
            .unwrap_or_else(|| {
                let all_ids: Vec<u64> = env
                    .storage()
                    .persistent()
                    .get(&StorageKey::CollectionList)
                    .unwrap_or_else(|| Vec::new(env));
                let mut pubs = Vec::new(env);
                for id in all_ids.iter() {
                    if let Some(col) = env
                        .storage()
                        .persistent()
                        .get::<_, Collection>(&StorageKey::Collection(id))
                    {
                        if col.is_public {
                            pubs.push_back(id);
                        }
                    }
                }
                pubs
            });

        let page_ids = paginate(env, &ids, start_index, limit);
        let mut result = Vec::new(env);
        for collection_id in page_ids.iter() {
            if let Some(collection) = env
                .storage()
                .persistent()
                .get::<_, Collection>(&StorageKey::Collection(collection_id))
            {
                result.push_back(collection);
            }
        }
        result
    }

    /// Explicit alias for listing public collections.
    pub fn list_public_collections(env: &Env, start_index: u32, limit: u32) -> Vec<Collection> {
        Self::list_collections(env, start_index, limit)
    }

    /// List all collections owned by a specific user (both public and private).
    /// Requires owner authentication (AC3 - private collections only for owner).
    pub fn list_user_collections(
        env: &Env,
        owner: Address,
        start_index: u32,
        limit: u32,
    ) -> Result<Vec<Collection>, ContractError> {
        owner.require_auth();
        let ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&CollectionVisibilityKey::UserCollections(owner.clone()))
            .unwrap_or_else(|| Vec::new(env));

        let page_ids = paginate(env, &ids, start_index, limit);
        let mut result = Vec::new(env);
        for collection_id in page_ids.iter() {
            if let Some(collection) = env
                .storage()
                .persistent()
                .get::<_, Collection>(&StorageKey::Collection(collection_id))
            {
                result.push_back(collection);
            }
        }
        Ok(result)
    }

    // ── Share Link Generation (Issue #819 AC4) ────────────────────────────

    /// Generate a cryptographic share link for a collection.
    /// Only the owner or an admin can generate a share link.
    pub fn generate_collection_share_link(
        env: &Env,
        caller: Address,
        collection_id: u64,
    ) -> Result<String, ContractError> {
        caller.require_auth();
        let collection = Self::require_collection(env, collection_id)?;
        if collection.owner != caller && !AdminManager::is_admin(env, &caller) {
            return Err(ContractError::NotCollectionOwner);
        }

        // Generate capability token seeded by collection, caller, timestamp, and sequence
        let mut seed = [0u8; 40];
        let id_bytes = collection_id.to_be_bytes();
        let ts_bytes = env.ledger().timestamp().to_be_bytes();
        let seq_bytes = env.ledger().sequence().to_be_bytes();

        seed[0..8].copy_from_slice(&id_bytes);
        seed[8..16].copy_from_slice(&ts_bytes);
        seed[16..20].copy_from_slice(&seq_bytes[0..4]);

        let mut hash_buf = Bytes::new(env);
        for &b in &seed[0..20] {
            hash_buf.push_back(b);
        }
        let hash = env.crypto().sha256(&hash_buf);
        let hash_arr = hash.to_array();

        let token_hex = Self::bytes_to_hex_string(env, &hash_arr);
        env.storage().persistent().set(
            &CollectionVisibilityKey::CollectionShareToken(collection_id),
            &token_hex,
        );

        let share_link = Self::format_share_link(env, collection_id, &token_hex);
        publish_collection_share_link_generated_event(env, collection_id, caller);
        Ok(share_link)
    }

    /// Retrieve a collection using a valid share token (grants read access even if private).
    pub fn get_collection_by_share_token(
        env: &Env,
        collection_id: u64,
        share_token: String,
    ) -> Result<Collection, ContractError> {
        let collection = Self::require_collection(env, collection_id)?;
        let stored_token: Option<String> = env
            .storage()
            .persistent()
            .get(&CollectionVisibilityKey::CollectionShareToken(collection_id));

        let stored = match stored_token {
            Some(tok) => tok,
            None => return Err(ContractError::ShareTokenNotFound),
        };

        let full_link = Self::format_share_link(env, collection_id, &stored);
        if stored != share_token && full_link != share_token {
            return Err(ContractError::InvalidShareToken);
        }

        Ok(collection)
    }

    /// Revoke the active share link for a collection.
    pub fn revoke_collection_share_link(
        env: &Env,
        caller: Address,
        collection_id: u64,
    ) -> Result<(), ContractError> {
        caller.require_auth();
        let collection = Self::require_collection(env, collection_id)?;
        if collection.owner != caller && !AdminManager::is_admin(env, &caller) {
            return Err(ContractError::NotCollectionOwner);
        }

        env.storage()
            .persistent()
            .remove(&CollectionVisibilityKey::CollectionShareToken(collection_id));

        publish_collection_share_link_revoked_event(env, collection_id, caller);
        Ok(())
    }

    // ── Project Membership Management ─────────────────────────────────────

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
            .unwrap_or_else(|| Vec::new(env));

        if project_ids.iter().any(|id| id == project_id) {
            return Err(ContractError::AlreadyInCollection);
        }

        if project_ids.len() >= MAX_PROJECTS_PER_COLLECTION {
            return Err(ContractError::CollectionFull);
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
            .unwrap_or_else(|| Vec::new(env));

        if !project_ids.iter().any(|id| id == project_id) {
            return Err(ContractError::NotInCollection);
        }

        let updated = Utils::remove_item_from_vec(env, &project_ids, &project_id);
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

    pub fn list_collection_projects(
        env: &Env,
        collection_id: u64,
        start_index: u32,
        limit: u32,
    ) -> Vec<u64> {
        if let Some(col) = env.storage().persistent().get::<_, Collection>(&StorageKey::Collection(collection_id)) {
            if !col.is_public {
                return Vec::new(env);
            }
        } else {
            return Vec::new(env);
        }

        let ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&StorageKey::CollectionProjectIds(collection_id))
            .unwrap_or_else(|| Vec::new(env));
        paginate(env, &ids, start_index, limit)
    }

    pub fn list_collection_projects_for_caller(
        env: &Env,
        caller: Address,
        collection_id: u64,
        start_index: u32,
        limit: u32,
    ) -> Result<Vec<u64>, ContractError> {
        caller.require_auth();
        let col = Self::require_collection(env, collection_id)?;
        if !col.is_public && col.owner != caller && !AdminManager::is_admin(env, &caller) {
            return Err(ContractError::CollectionPrivate);
        }

        let ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&StorageKey::CollectionProjectIds(collection_id))
            .unwrap_or_else(|| Vec::new(env));
        Ok(paginate(env, &ids, start_index, limit))
    }

    pub fn get_collection_project_count(env: &Env, collection_id: u64) -> u32 {
        let ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&StorageKey::CollectionProjectIds(collection_id))
            .unwrap_or_else(|| Vec::new(env));
        ids.len()
    }

    pub fn get_collection_count(env: &Env) -> u64 {
        let ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&StorageKey::CollectionList)
            .unwrap_or_else(|| Vec::new(env));
        ids.len().into()
    }

    // ── Internal Helpers ──────────────────────────────────────────────────

    fn bytes_to_hex_string(env: &Env, bytes: &[u8]) -> String {
        const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";
        let mut buf = [0u8; 64];
        let len = core::cmp::min(bytes.len() * 2, buf.len());
        for (i, &b) in bytes.iter().take(buf.len() / 2).enumerate() {
            buf[i * 2] = HEX_CHARS[(b >> 4) as usize];
            buf[i * 2 + 1] = HEX_CHARS[(b & 0x0f) as usize];
        }
        let s = core::str::from_utf8(&buf[..len]).unwrap_or("");
        String::from_str(env, s)
    }

    fn format_share_link(env: &Env, id: u64, token: &String) -> String {
        let prefix = "https://dongle.hub/c/";
        let mid = "?key=";
        let mut buf = [0u8; 256];
        let mut cursor = 0;

        for &b in prefix.as_bytes() {
            if cursor < buf.len() {
                buf[cursor] = b;
                cursor += 1;
            }
        }

        let mut id_digits = [0u8; 20];
        let mut num = id;
        let mut id_len = 0;
        if num == 0 {
            id_digits[0] = b'0';
            id_len = 1;
        } else {
            while num > 0 {
                id_digits[id_len] = b'0' + (num % 10) as u8;
                num /= 10;
                id_len += 1;
            }
            id_digits[..id_len].reverse();
        }
        for i in 0..id_len {
            if cursor < buf.len() {
                buf[cursor] = id_digits[i];
                cursor += 1;
            }
        }

        for &b in mid.as_bytes() {
            if cursor < buf.len() {
                buf[cursor] = b;
                cursor += 1;
            }
        }

        let token_len = token.len() as usize;
        let mut token_buf = [0u8; 128];
        let t_cap = core::cmp::min(token_len, token_buf.len());
        token.copy_into_slice(&mut token_buf[..t_cap]);
        for i in 0..t_cap {
            if cursor < buf.len() {
                buf[cursor] = token_buf[i];
                cursor += 1;
            }
        }

        let res = core::str::from_utf8(&buf[..cursor]).unwrap_or("");
        String::from_str(env, res)
    }

    fn validate_name(name: &String) -> Result<(), ContractError> {
        let len = name.len();
        if len == 0 {
            return Err(ContractError::InvalidProjectData);
        }
        if len as usize > MAX_COLLECTION_NAME_LEN {
            return Err(ContractError::InvalidProjectName);
        }
        Ok(())
    }

    fn validate_description(description: &String) -> Result<(), ContractError> {
        let len = description.len();
        if len == 0 {
            return Err(ContractError::InvalidProjectData);
        }
        if len as usize > MAX_COLLECTION_DESCRIPTION_LEN {
            return Err(ContractError::InvalidProjectData);
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
            .unwrap_or_else(|| Vec::new(env));

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
                    return Err(ContractError::CollectionExists);
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
