//! Shareable bookmark collections (#916).
//!
//! A bookmark collection is a named list of projects owned by one user.
//! Collections are private on creation; only the owner can read or edit them.
//! `make_bookmark_collection_public` publishes a collection: it becomes
//! readable by anyone, receives a deterministic `share_url`, is added to the
//! public index and can be followed by other users.
//!
//! ## Access control
//!
//! * Mutations (`add`/`remove` project, publish, unpublish) require the
//!   owner's auth.
//! * Reads of a private collection succeed only when `viewer == owner`.
//! * Following requires the collection to be public and the follower to not
//!   be the owner.  Unfollowing is always allowed so followers can clean up
//!   after a collection goes private.

use crate::constants::{
    BOOKMARK_COLLECTION_SHARE_URL_PREFIX, MAX_BOOKMARK_COLLECTIONS_PER_USER,
    MAX_FOLDER_NAME_LEN, MAX_FOLLOWED_BOOKMARK_COLLECTIONS, MAX_PAGE_LIMIT,
    MAX_PROJECTS_PER_BOOKMARK_COLLECTION,
};
use crate::errors::ContractError;
use crate::events::{
    publish_bookmark_collection_followed_event, publish_bookmark_collection_published_event,
    publish_bookmark_collection_unfollowed_event, publish_bookmark_collection_unpublished_event,
};
use crate::pagination::paginate;
use crate::project_registry::ProjectRegistry;
use crate::storage_keys::BookmarkCollectionKey;
use crate::types::BookmarkCollection;
use crate::utils::Utils;
use soroban_sdk::{Address, Env, String, Vec};

pub struct BookmarkCollectionRegistry;

impl BookmarkCollectionRegistry {
    // ── Collection CRUD ───────────────────────────────────────────────────

    /// Create a private bookmark collection owned by `owner`.  Returns its ID.
    pub fn create_bookmark_collection(
        env: &Env,
        owner: Address,
        name: String,
    ) -> Result<u64, ContractError> {
        owner.require_auth();
        Self::validate_name(&name)?;

        let mut owned = Self::get_owner_collection_ids(env, &owner);
        if owned.len() >= MAX_BOOKMARK_COLLECTIONS_PER_USER {
            return Err(ContractError::MaxBookmarkCollectionsExceeded);
        }

        let id: u64 = env
            .storage()
            .persistent()
            .get(&BookmarkCollectionKey::NextCollectionId)
            .unwrap_or(1u64);
        env.storage()
            .persistent()
            .set(&BookmarkCollectionKey::NextCollectionId, &(id + 1));

        let ts = env.ledger().timestamp();
        let collection = BookmarkCollection {
            id,
            owner: owner.clone(),
            name,
            project_ids: Vec::new(env),
            public: false,
            share_url: String::from_str(env, ""),
            follower_count: 0,
            created_at: ts,
            updated_at: ts,
            published_at: None,
        };
        Self::save(env, &collection);

        owned.push_back(id);
        env.storage()
            .persistent()
            .set(&BookmarkCollectionKey::OwnerCollections(owner), &owned);

        Ok(id)
    }

    /// Add a project to a collection the caller owns.
    pub fn add_project_to_bookmark_collection(
        env: &Env,
        owner: Address,
        collection_id: u64,
        project_id: u64,
    ) -> Result<(), ContractError> {
        owner.require_auth();
        let mut collection = Self::require_owned(env, &owner, collection_id)?;

        if ProjectRegistry::get_project(env, project_id).is_none() {
            return Err(ContractError::ProjectNotFound);
        }
        if collection.project_ids.contains(project_id) {
            return Err(ContractError::ProjectAlreadyInBookmarkCollection);
        }
        if collection.project_ids.len() >= MAX_PROJECTS_PER_BOOKMARK_COLLECTION {
            return Err(ContractError::BookmarkCollectionFull);
        }

        collection.project_ids.push_back(project_id);
        collection.updated_at = env.ledger().timestamp();
        Self::save(env, &collection);
        Ok(())
    }

    /// Remove a project from a collection the caller owns.
    pub fn remove_project_from_bookmark_collection(
        env: &Env,
        owner: Address,
        collection_id: u64,
        project_id: u64,
    ) -> Result<(), ContractError> {
        owner.require_auth();
        let mut collection = Self::require_owned(env, &owner, collection_id)?;

        if !collection.project_ids.contains(project_id) {
            return Err(ContractError::ProjectNotInBookmarkCollection);
        }

        collection.project_ids = Utils::remove_item_from_vec(env, &collection.project_ids, &project_id);
        collection.updated_at = env.ledger().timestamp();
        Self::save(env, &collection);
        Ok(())
    }

    // ── Publishing ────────────────────────────────────────────────────────

    /// Publish a collection so others can view and follow it.  Returns the
    /// generated share URL.
    pub fn make_bookmark_collection_public(
        env: &Env,
        owner: Address,
        collection_id: u64,
    ) -> Result<String, ContractError> {
        owner.require_auth();
        let mut collection = Self::require_owned(env, &owner, collection_id)?;

        if collection.public {
            return Err(ContractError::BookmarkCollectionAlreadyPublic);
        }

        let ts = env.ledger().timestamp();
        collection.public = true;
        collection.share_url = Self::build_share_url(env, collection_id);
        collection.published_at = Some(ts);
        collection.updated_at = ts;
        Self::save(env, &collection);

        let mut public_ids = Self::get_public_collection_ids(env);
        if !public_ids.contains(collection_id) {
            public_ids.push_back(collection_id);
            env.storage()
                .persistent()
                .set(&BookmarkCollectionKey::PublicCollections, &public_ids);
        }

        publish_bookmark_collection_published_event(
            env,
            collection_id,
            owner,
            collection.share_url.clone(),
        );
        Ok(collection.share_url)
    }

    /// Make a public collection private again.  The share URL is cleared;
    /// followers are retained but lose read access until it is re-published.
    pub fn make_bookmark_collection_private(
        env: &Env,
        owner: Address,
        collection_id: u64,
    ) -> Result<(), ContractError> {
        owner.require_auth();
        let mut collection = Self::require_owned(env, &owner, collection_id)?;

        if !collection.public {
            return Err(ContractError::BookmarkCollectionNotPublic);
        }

        collection.public = false;
        collection.share_url = String::from_str(env, "");
        collection.updated_at = env.ledger().timestamp();
        Self::save(env, &collection);

        let public_ids = Self::get_public_collection_ids(env);
        let remaining = Utils::remove_item_from_vec(env, &public_ids, &collection_id);
        env.storage()
            .persistent()
            .set(&BookmarkCollectionKey::PublicCollections, &remaining);

        publish_bookmark_collection_unpublished_event(env, collection_id, owner);
        Ok(())
    }

    // ── Followers ─────────────────────────────────────────────────────────

    /// Follow a public collection.
    pub fn follow_bookmark_collection(
        env: &Env,
        follower: Address,
        collection_id: u64,
    ) -> Result<(), ContractError> {
        follower.require_auth();
        let mut collection = Self::require_collection(env, collection_id)?;

        if !collection.public {
            return Err(ContractError::BookmarkCollectionNotPublic);
        }
        if collection.owner == follower {
            return Err(ContractError::CannotFollowOwnBookmarkCollection);
        }

        let mut followers = Self::get_follower_list(env, collection_id);
        if followers.contains(&follower) {
            return Err(ContractError::AlreadyFollowingBookmarkCollection);
        }
        let mut followed = Self::get_followed_ids(env, &follower);
        if followed.len() >= MAX_FOLLOWED_BOOKMARK_COLLECTIONS {
            return Err(ContractError::MaxFollowedBookmarkCollectionsExceeded);
        }

        followers.push_back(follower.clone());
        env.storage()
            .persistent()
            .set(&BookmarkCollectionKey::Followers(collection_id), &followers);
        followed.push_back(collection_id);
        env.storage()
            .persistent()
            .set(&BookmarkCollectionKey::FollowedBy(follower.clone()), &followed);

        collection.follower_count = followers.len();
        Self::save(env, &collection);

        publish_bookmark_collection_followed_event(env, collection_id, follower);
        Ok(())
    }

    /// Stop following a collection.  Allowed even if it has gone private.
    pub fn unfollow_bookmark_collection(
        env: &Env,
        follower: Address,
        collection_id: u64,
    ) -> Result<(), ContractError> {
        follower.require_auth();
        let mut collection = Self::require_collection(env, collection_id)?;

        let followers = Self::get_follower_list(env, collection_id);
        if !followers.contains(&follower) {
            return Err(ContractError::NotFollowingBookmarkCollection);
        }

        let remaining = Utils::remove_item_from_vec(env, &followers, &follower);
        env.storage()
            .persistent()
            .set(&BookmarkCollectionKey::Followers(collection_id), &remaining);
        let followed = Self::get_followed_ids(env, &follower);
        let followed = Utils::remove_item_from_vec(env, &followed, &collection_id);
        env.storage()
            .persistent()
            .set(&BookmarkCollectionKey::FollowedBy(follower.clone()), &followed);

        collection.follower_count = remaining.len();
        Self::save(env, &collection);

        publish_bookmark_collection_unfollowed_event(env, collection_id, follower);
        Ok(())
    }

    pub fn is_following_bookmark_collection(
        env: &Env,
        follower: &Address,
        collection_id: u64,
    ) -> bool {
        Self::get_follower_list(env, collection_id).contains(follower)
    }

    /// Paginated followers of a collection.  Private collections only expose
    /// their followers to the owner.
    pub fn get_bookmark_collection_followers(
        env: &Env,
        viewer: Address,
        collection_id: u64,
        start_index: u32,
        limit: u32,
    ) -> Result<Vec<Address>, ContractError> {
        Self::get_bookmark_collection(env, viewer, collection_id)?;
        let followers = Self::get_follower_list(env, collection_id);
        Ok(paginate(env, &followers, start_index, Self::effective_limit(limit)))
    }

    /// Collections followed by `user` that are currently public.
    pub fn get_followed_bookmark_collections(env: &Env, user: Address) -> Vec<BookmarkCollection> {
        let mut result = Vec::new(env);
        for id in Self::get_followed_ids(env, &user).iter() {
            if let Some(collection) = Self::load(env, id) {
                if collection.public {
                    result.push_back(collection);
                }
            }
        }
        result
    }

    // ── Reads ─────────────────────────────────────────────────────────────

    /// Read a collection.  Private collections are visible only to the owner.
    pub fn get_bookmark_collection(
        env: &Env,
        viewer: Address,
        collection_id: u64,
    ) -> Result<BookmarkCollection, ContractError> {
        let collection = Self::require_collection(env, collection_id)?;
        if !collection.public && collection.owner != viewer {
            return Err(ContractError::BookmarkCollectionNotPublic);
        }
        Ok(collection)
    }

    /// Every collection owned by `owner` (public and private).
    pub fn list_user_bookmark_collections(env: &Env, owner: Address) -> Vec<BookmarkCollection> {
        let mut result = Vec::new(env);
        for id in Self::get_owner_collection_ids(env, &owner).iter() {
            if let Some(collection) = Self::load(env, id) {
                result.push_back(collection);
            }
        }
        result
    }

    /// Paginated list of public collection IDs, oldest publish first.
    pub fn list_public_bookmark_collections(env: &Env, start_index: u32, limit: u32) -> Vec<u64> {
        let ids = Self::get_public_collection_ids(env);
        paginate(env, &ids, start_index, Self::effective_limit(limit))
    }

    // ── Internal helpers ──────────────────────────────────────────────────

    fn validate_name(name: &String) -> Result<(), ContractError> {
        let len = name.len();
        if len == 0 {
            return Err(ContractError::InvalidProjectData);
        }
        if len as usize > MAX_FOLDER_NAME_LEN {
            return Err(ContractError::InvalidProjectName);
        }
        Ok(())
    }

    fn effective_limit(limit: u32) -> u32 {
        if limit == 0 || limit > MAX_PAGE_LIMIT {
            MAX_PAGE_LIMIT
        } else {
            limit
        }
    }

    fn load(env: &Env, collection_id: u64) -> Option<BookmarkCollection> {
        env.storage()
            .persistent()
            .get(&BookmarkCollectionKey::Collection(collection_id))
    }

    fn save(env: &Env, collection: &BookmarkCollection) {
        env.storage()
            .persistent()
            .set(&BookmarkCollectionKey::Collection(collection.id), collection);
    }

    fn require_collection(env: &Env, collection_id: u64) -> Result<BookmarkCollection, ContractError> {
        Self::load(env, collection_id).ok_or(ContractError::BookmarkCollectionNotFound)
    }

    fn require_owned(
        env: &Env,
        owner: &Address,
        collection_id: u64,
    ) -> Result<BookmarkCollection, ContractError> {
        let collection = Self::require_collection(env, collection_id)?;
        if collection.owner != *owner {
            return Err(ContractError::Unauthorized);
        }
        Ok(collection)
    }

    fn get_owner_collection_ids(env: &Env, owner: &Address) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&BookmarkCollectionKey::OwnerCollections(owner.clone()))
            .unwrap_or_else(|| Vec::new(env))
    }

    fn get_public_collection_ids(env: &Env) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&BookmarkCollectionKey::PublicCollections)
            .unwrap_or_else(|| Vec::new(env))
    }

    fn get_follower_list(env: &Env, collection_id: u64) -> Vec<Address> {
        env.storage()
            .persistent()
            .get(&BookmarkCollectionKey::Followers(collection_id))
            .unwrap_or_else(|| Vec::new(env))
    }

    fn get_followed_ids(env: &Env, user: &Address) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&BookmarkCollectionKey::FollowedBy(user.clone()))
            .unwrap_or_else(|| Vec::new(env))
    }

    /// `BOOKMARK_COLLECTION_SHARE_URL_PREFIX` followed by the decimal ID.
    fn build_share_url(env: &Env, collection_id: u64) -> String {
        let prefix = BOOKMARK_COLLECTION_SHARE_URL_PREFIX.as_bytes();
        let mut buf = [0u8; 64];
        buf[..prefix.len()].copy_from_slice(prefix);
        let mut cursor = prefix.len();

        let mut digits = [0u8; 20];
        let mut num = collection_id;
        let mut len = 0;
        loop {
            digits[len] = b'0' + (num % 10) as u8;
            num /= 10;
            len += 1;
            if num == 0 {
                break;
            }
        }
        for i in (0..len).rev() {
            buf[cursor] = digits[i];
            cursor += 1;
        }

        String::from_bytes(env, &buf[..cursor])
    }
}
