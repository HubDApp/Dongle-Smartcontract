use crate::admin_action_log::AdminActionLog;
use crate::admin_manager::AdminManager;
use crate::auth::require_admin_auth;
use crate::constants::{
    MAX_COLLECTIONS, MAX_COLLECTION_DEPTH, MAX_COLLECTION_DESCRIPTION_LEN, MAX_COLLECTION_NAME_LEN,
    MAX_COLLECTION_TREE_NODES, MAX_PROJECTS_PER_COLLECTION,
};
use crate::errors::ContractError;
use crate::events::{
    publish_collection_created_event, publish_collection_deleted_event,
    publish_collection_share_link_generated_event, publish_collection_share_link_revoked_event,
    publish_collection_snapshot_created_event, publish_collection_updated_event,
    publish_collection_visibility_toggled_event, publish_project_added_to_collection_event,
    publish_project_removed_from_collection_event,
};
use crate::pagination::paginate;
use crate::storage_keys::{CollectionHierarchyKey, CollectionVisibilityKey, StorageKey};
use crate::types::{AdminActionType, Collection, CollectionTreeNode};
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
        Self::create_collection_internal(env, admin, name, description, true, None)
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
        Self::create_collection_internal(env, admin, name, description, is_public, None)
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
        Self::create_collection_internal(env, creator, name, description, is_public, None)
    }

    /// Create a collection **nested inside** an existing collection
    /// (issue #905).
    ///
    /// The parent must exist and the resulting hierarchy depth must not
    /// exceed [`MAX_COLLECTION_DEPTH`].  Because a brand-new collection has
    /// no children of its own, the depth of the new node is
    /// `depth(parent) + 1`.
    pub fn create_nested_collection(
        env: &Env,
        creator: Address,
        name: String,
        description: String,
        is_public: bool,
        parent_collection_id: u64,
    ) -> Result<u64, ContractError> {
        creator.require_auth();

        let parent = Self::require_collection(env, parent_collection_id)?;
        let parent_depth = Self::compute_depth(env, &parent)?;
        if parent_depth + 1 > MAX_COLLECTION_DEPTH {
            return Err(ContractError::CollectionDepthExceeded);
        }

        Self::create_collection_internal(
            env,
            creator,
            name,
            description,
            is_public,
            Some(parent_collection_id),
        )
    }

    fn create_collection_internal(
        env: &Env,
        creator: Address,
        name: String,
        description: String,
        is_public: bool,
        parent_collection_id: Option<u64>,
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
            parent_collection_id,
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

        // Register the collection in its parent's child index (issue #905).
        if let Some(parent_id) = parent_collection_id {
            Self::add_child(env, parent_id, id);
        }

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

        // A collection that still contains nested collections cannot be
        // deleted — the hierarchy must stay acyclic and fully reachable
        // (issue #905).
        if !Self::get_child_ids(env, collection_id).is_empty() {
            return Err(ContractError::CollectionHasChildren);
        }

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

        // Detach the collection from its parent (issue #905).
        if let Some(parent_id) = collection.parent_collection_id {
            Self::remove_child(env, parent_id, collection_id);
        }

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

    // ── Collection Hierarchy & Nesting (Issue #905) ────────────────────────

    /// Move a collection under a new parent (or promote it to the top level
    /// by passing `None`).
    ///
    /// Guards (all of them keep the hierarchy a **forest**):
    /// - the new parent must exist (`CollectionNotFound`),
    /// - a collection can never become its own ancestor
    ///   (`CollectionHierarchyCycle`),
    /// - the resulting depth of the moved subtree must not exceed
    ///   [`MAX_COLLECTION_DEPTH`] (`CollectionDepthExceeded`).
    pub fn set_collection_parent(
        env: &Env,
        caller: Address,
        collection_id: u64,
        parent_collection_id: Option<u64>,
    ) -> Result<(), ContractError> {
        caller.require_auth();

        let mut collection = Self::require_collection(env, collection_id)?;
        if collection.owner != caller && !AdminManager::is_admin(env, &caller) {
            return Err(ContractError::NotCollectionOwner);
        }

        let old_parent = collection.parent_collection_id;

        if let Some(parent_id) = parent_collection_id {
            if parent_id == collection_id {
                return Err(ContractError::CollectionHierarchyCycle);
            }

            let parent = Self::require_collection(env, parent_id)?;

            // The new parent must not be a descendant of the collection
            // itself — otherwise the hierarchy would contain a cycle.
            if Self::is_descendant(env, parent_id, collection_id)? {
                return Err(ContractError::CollectionHierarchyCycle);
            }

            // The whole subtree is re-parented, so the deepest leaf decides
            // whether the depth limit is still respected.
            let parent_depth = Self::compute_depth(env, &parent)?;
            let subtree_height = Self::subtree_height(env, collection_id)?;
            let deepest_leaf_depth = parent_depth + subtree_height - 1;
            if deepest_leaf_depth > MAX_COLLECTION_DEPTH {
                return Err(ContractError::CollectionDepthExceeded);
            }
        }

        collection.parent_collection_id = parent_collection_id;
        collection.updated_at = env.ledger().timestamp();
        env.storage()
            .persistent()
            .set(&StorageKey::Collection(collection_id), &collection);

        // Update the reverse (parent → children) index.
        if let Some(old) = old_parent {
            if Some(old) != parent_collection_id {
                Self::remove_child(env, old, collection_id);
            }
        }
        if let Some(new) = parent_collection_id {
            Self::add_child(env, new, collection_id);
        }

        publish_collection_updated_event(env, collection_id, caller);
        Ok(())
    }

    /// Return the subtree rooted at `root_id`.
    ///
    /// The nodes are returned in breadth-first order; every node carries its
    /// depth (relative to the root) and the identifiers of its direct
    /// children, which together describe the nested structure (issue #905).
    ///
    /// The walk is bounded by [`MAX_COLLECTION_TREE_NODES`] and by the
    /// acyclic-graph invariant, so it always terminates.
    pub fn get_collection_tree(
        env: &Env,
        root_id: u64,
    ) -> Result<Vec<CollectionTreeNode>, ContractError> {
        // Validate that the root exists.
        Self::require_collection(env, root_id)?;

        let mut nodes = Vec::new(env);
        let mut queue: Vec<(u64, u32)> = Vec::new(env);
        queue.push_back((root_id, 0u32));

        while !queue.is_empty() {
            if nodes.len() >= MAX_COLLECTION_TREE_NODES {
                break;
            }
            let (current_id, depth) = queue.remove(0);
            let collection = match env
                .storage()
                .persistent()
                .get::<_, Collection>(&StorageKey::Collection(current_id))
            {
                Some(c) => c,
                None => continue,
            };
            let parent_id = collection.parent_collection_id;
            let child_ids = Self::get_child_ids(env, current_id);
            for child_id in child_ids.iter() {
                queue.push_back((child_id, depth + 1));
            }
            nodes.push_back(CollectionTreeNode {
                collection,
                depth,
                parent_id,
                child_ids,
            });
        }

        Ok(nodes)
    }

    /// Return the breadcrumb path of a collection: the chain of ancestors
    /// from the root collection down to (and including) `collection_id`
    /// (issue #905).
    ///
    /// Returns an empty vector for an unknown collection.
    pub fn get_collection_path(
        env: &Env,
        collection_id: u64,
    ) -> Result<Vec<Collection>, ContractError> {
        let mut path: Vec<Collection> = Vec::new(env);
        let mut current_id = collection_id;
        let mut guard: u32 = 0;

        loop {
            guard += 1;
            if guard > MAX_COLLECTION_DEPTH {
                // The stored hierarchy violates the depth invariant — refuse
                // to walk further instead of looping forever.
                return Err(ContractError::CollectionDepthExceeded);
            }

            let collection = match env
                .storage()
                .persistent()
                .get::<_, Collection>(&StorageKey::Collection(current_id))
            {
                Some(c) => c,
                None => break,
            };
            let parent_id = collection.parent_collection_id;
            path.push_back(collection);

            match parent_id {
                Some(pid) => current_id = pid,
                None => break,
            }
        }

        // Reverse so that the root comes first.
        let mut ordered: Vec<Collection> = Vec::new(env);
        let mut i = path.len();
        while i > 0 {
            i -= 1;
            if let Some(c) = path.get(i) {
                ordered.push_back(c.clone());
            }
        }

        Ok(ordered)
    }

    /// Direct children of a collection (issue #905).
    pub fn get_collection_children(env: &Env, collection_id: u64) -> Vec<u64> {
        Self::get_child_ids(env, collection_id)
    }

    /// Depth of a collection in the hierarchy (`1` for a top-level
    /// collection).  Issue #905.
    pub fn get_collection_depth(env: &Env, collection_id: u64) -> Result<u32, ContractError> {
        let collection = Self::require_collection(env, collection_id)?;
        Self::compute_depth(env, &collection)
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

    // ── Hierarchy helpers (issue #905) ────────────────────────────────────

    /// Direct children of a collection (reverse index).
    fn get_child_ids(env: &Env, parent_id: u64) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&CollectionHierarchyKey::Children(parent_id))
            .unwrap_or_else(|| Vec::new(env))
    }

    /// Register `child_id` in the child index of `parent_id` (idempotent).
    fn add_child(env: &Env, parent_id: u64, child_id: u64) {
        let mut children = Self::get_child_ids(env, parent_id);
        if !children.iter().any(|c| c == child_id) {
            children.push_back(child_id);
            env.storage()
                .persistent()
                .set(&CollectionHierarchyKey::Children(parent_id), &children);
        }
    }

    /// Remove `child_id` from the child index of `parent_id` (idempotent).
    fn remove_child(env: &Env, parent_id: u64, child_id: u64) {
        let children = Self::get_child_ids(env, parent_id);
        let mut remaining: Vec<u64> = Vec::new(env);
        for c in children.iter() {
            if c != child_id {
                remaining.push_back(c);
            }
        }
        env.storage()
            .persistent()
            .set(&CollectionHierarchyKey::Children(parent_id), &remaining);
    }

    /// Depth of a collection: `1` for a top-level collection.
    ///
    /// Walks the parent chain; the walk is bounded by
    /// [`MAX_COLLECTION_DEPTH`] so a corrupted (cyclic) hierarchy can never
    /// loop forever.
    fn compute_depth(env: &Env, collection: &Collection) -> Result<u32, ContractError> {
        let mut depth: u32 = 1;
        let mut current = collection.parent_collection_id;
        let mut steps: u32 = 0;

        while let Some(parent_id) = current {
            steps += 1;
            if steps > MAX_COLLECTION_DEPTH {
                return Err(ContractError::CollectionDepthExceeded);
            }
            depth += 1;
            if depth > MAX_COLLECTION_DEPTH {
                return Err(ContractError::CollectionDepthExceeded);
            }
            let parent = Self::require_collection(env, parent_id)?;
            current = parent.parent_collection_id;
        }

        Ok(depth)
    }

    /// Returns `true` when `ancestor_id` is an ancestor of `candidate_id`.
    ///
    /// Used to reject parent assignments that would create a cycle.
    fn is_descendant(
        env: &Env,
        candidate_id: u64,
        ancestor_id: u64,
    ) -> Result<bool, ContractError> {
        let mut current_id = candidate_id;
        let mut steps: u32 = 0;

        loop {
            steps += 1;
            if steps > MAX_COLLECTION_DEPTH {
                return Ok(false);
            }

            let current = match env
                .storage()
                .persistent()
                .get::<_, Collection>(&StorageKey::Collection(current_id))
            {
                Some(c) => c,
                None => return Ok(false),
            };

            match current.parent_collection_id {
                Some(parent_id) => {
                    if parent_id == ancestor_id {
                        return Ok(true);
                    }
                    current_id = parent_id;
                }
                None => return Ok(false),
            }
        }
    }

    /// Height of the subtree rooted at `root_id` (`1` for a leaf).
    fn subtree_height(env: &Env, root_id: u64) -> Result<u32, ContractError> {
        let mut max_depth: u32 = 1;
        let mut queue: Vec<(u64, u32)> = Vec::new(env);
        queue.push_back((root_id, 1u32));
        let mut visited: u32 = 0;

        while !queue.is_empty() {
            visited += 1;
            if visited > MAX_COLLECTION_TREE_NODES {
                break;
            }
            let (current_id, depth) = queue.remove(0);
            if depth > max_depth {
                max_depth = depth;
            }
            let children = Self::get_child_ids(env, current_id);
            for child_id in children.iter() {
                queue.push_back((child_id, depth + 1));
            }
        }

        Ok(max_depth)
    }

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

    // ── Collection Immutable Snapshots (#908) ──────────────────────────────

    /// Create an immutable snapshot of the collection state with an IPFS CID.
    pub fn snapshot_collection(
        env: &Env,
        caller: Address,
        collection_id: u64,
        ipfs_cid: Option<String>,
    ) -> Result<CollectionSnapshot, ContractError> {
        caller.require_auth();

        let collection = Self::require_collection(env, collection_id)?;

        if collection.owner != caller && !AdminManager::is_admin(env, &caller) {
            return Err(ContractError::Unauthorized);
        }

        let cid = match ipfs_cid {
            Some(ref c) => {
                Utils::validate_metadata_cid(c)?;
                c.clone()
            }
            None => {
                // Canonical default CIDv1 representation for on-chain generated snapshot
                String::from_str(env, "bafybeigdyrzt5sfp7udm7ahu76uh7y26nf3efuylqabf3oclgtqy55fbkd")
            }
        };

        let project_ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&StorageKey::CollectionProjectIds(collection_id))
            .unwrap_or_else(|| Vec::new(env));

        let next_ver: u32 = env
            .storage()
            .persistent()
            .get(&CollectionSnapshotKey::NextSnapshotVersion(collection_id))
            .unwrap_or(1u32);

        let now = env.ledger().timestamp();
        let snapshot = CollectionSnapshot {
            collection_id,
            version: next_ver,
            ipfs_cid: cid.clone(),
            project_count: project_ids.len(),
            project_ids,
            created_at: now,
            created_by: caller.clone(),
        };

        env.storage().persistent().set(
            &CollectionSnapshotKey::Snapshot(collection_id, next_ver),
            &snapshot,
        );
        env.storage().persistent().set(
            &CollectionSnapshotKey::LatestSnapshotVersion(collection_id),
            &next_ver,
        );
        env.storage().persistent().set(
            &CollectionSnapshotKey::NextSnapshotVersion(collection_id),
            &next_ver.saturating_add(1),
        );

        let mut versions: Vec<u32> = env
            .storage()
            .persistent()
            .get(&CollectionSnapshotKey::CollectionSnapshotVersions(collection_id))
            .unwrap_or_else(|| Vec::new(env));
        versions.push_back(next_ver);
        env.storage().persistent().set(
            &CollectionSnapshotKey::CollectionSnapshotVersions(collection_id),
            &versions,
        );

        publish_collection_snapshot_created_event(
            env,
            collection_id,
            next_ver,
            cid,
            caller,
            now,
        );

        Ok(snapshot)
    }

    /// Retrieve a specific version snapshot of a collection.
    pub fn get_collection_snapshot(
        env: &Env,
        collection_id: u64,
        version: u32,
    ) -> Option<CollectionSnapshot> {
        env.storage()
            .persistent()
            .get(&CollectionSnapshotKey::Snapshot(collection_id, version))
    }

    /// Retrieve the most recent snapshot of a collection.
    pub fn get_latest_collection_snapshot(
        env: &Env,
        collection_id: u64,
    ) -> Option<CollectionSnapshot> {
        let latest_ver: u32 = env
            .storage()
            .persistent()
            .get(&CollectionSnapshotKey::LatestSnapshotVersion(collection_id))?;
        Self::get_collection_snapshot(env, collection_id, latest_ver)
    }

    /// List all snapshots for a collection with pagination.
    pub fn list_collection_snapshots(
        env: &Env,
        collection_id: u64,
        start_index: u32,
        limit: u32,
    ) -> Vec<CollectionSnapshot> {
        let effective_limit = if limit == 0 || limit > MAX_PAGE_LIMIT {
            MAX_PAGE_LIMIT
        } else {
            limit
        };

        let versions: Vec<u32> = env
            .storage()
            .persistent()
            .get(&CollectionSnapshotKey::CollectionSnapshotVersions(collection_id))
            .unwrap_or_else(|| Vec::new(env));

        let len = versions.len();
        if start_index >= len {
            return Vec::new(env);
        }

        let end = core::cmp::min(start_index.saturating_add(effective_limit), len);
        let mut list = Vec::new(env);
        for i in start_index..end {
            if let Some(ver) = versions.get(i) {
                if let Some(snap) = Self::get_collection_snapshot(env, collection_id, ver) {
                    list.push_back(snap);
                }
            }
        }
        list
    }

    /// Compare two snapshots of a collection to observe added, removed, and unchanged projects.
    pub fn compare_collection_snapshots(
        env: &Env,
        collection_id: u64,
        version_a: u32,
        version_b: u32,
    ) -> Result<CollectionSnapshotComparison, ContractError> {
        let snap_a = Self::get_collection_snapshot(env, collection_id, version_a)
            .ok_or(ContractError::SnapshotNotFound)?;
        let snap_b = Self::get_collection_snapshot(env, collection_id, version_b)
            .ok_or(ContractError::SnapshotNotFound)?;

        let mut added_projects = Vec::new(env);
        let mut removed_projects = Vec::new(env);
        let mut unchanged_projects = Vec::new(env);

        for i in 0..snap_b.project_ids.len() {
            if let Some(pid) = snap_b.project_ids.get(i) {
                if snap_a.project_ids.contains(pid) {
                    unchanged_projects.push_back(pid);
                } else {
                    added_projects.push_back(pid);
                }
            }
        }

        for i in 0..snap_a.project_ids.len() {
            if let Some(pid) = snap_a.project_ids.get(i) {
                if !snap_b.project_ids.contains(pid) {
                    removed_projects.push_back(pid);
                }
            }
        }

        Ok(CollectionSnapshotComparison {
            collection_id,
            version_a,
            version_b,
            cid_a: snap_a.ipfs_cid,
            cid_b: snap_b.ipfs_cid,
            added_projects,
            removed_projects,
            unchanged_projects,
        })
    }
}

