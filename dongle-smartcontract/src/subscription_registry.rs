use crate::errors::ContractError;
use crate::events::{publish_project_followed_event, publish_project_unfollowed_event};
use crate::project_registry::ProjectRegistry;
use crate::storage_keys::ExtensionKey;
use crate::storage_manager::StorageManager;
use soroban_sdk::{Address, Env, Vec};

pub const MAX_PAGE_LIMIT: u32 = 100;

pub struct SubscriptionRegistry;

impl SubscriptionRegistry {
    pub fn follow_project(
        env: &Env,
        project_id: u64,
        follower: Address,
    ) -> Result<(), ContractError> {
        follower.require_auth();

        ProjectRegistry::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;

        if Self::is_following(env, project_id, &follower) {
            return Err(ContractError::AlreadyFollowing);
        }

        let mut followers: Vec<Address> = env
            .storage()
            .persistent()
            .get(&ExtensionKey::ProjectFollowers(project_id))
            .unwrap_or_else(|| Vec::new(env));
        followers.push_back(follower.clone());
        env.storage()
            .persistent()
            .set(&ExtensionKey::ProjectFollowers(project_id), &followers);

        let count: u32 = followers.len();
        env.storage()
            .persistent()
            .set(&ExtensionKey::FollowerCount(project_id), &count);

        let mut subscriptions: Vec<u64> = env
            .storage()
            .persistent()
            .get(&ExtensionKey::UserSubscriptions(follower.clone()))
            .unwrap_or_else(|| Vec::new(env));
        subscriptions.push_back(project_id);
        env.storage().persistent().set(
            &ExtensionKey::UserSubscriptions(follower.clone()),
            &subscriptions,
        );

        StorageManager::extend_project_ttl(env, project_id);
        StorageManager::extend_followers_ttl(env, project_id);
        StorageManager::extend_user_subscriptions_ttl(env, &follower);

        publish_project_followed_event(env, project_id, follower);
        Ok(())
    }

    pub fn unfollow_project(
        env: &Env,
        project_id: u64,
        follower: Address,
    ) -> Result<(), ContractError> {
        follower.require_auth();

        if !Self::is_following(env, project_id, &follower) {
            return Err(ContractError::NotFollowing);
        }

        let mut followers: Vec<Address> = env
            .storage()
            .persistent()
            .get(&ExtensionKey::ProjectFollowers(project_id))
            .unwrap_or_else(|| Vec::new(env));

        let mut new_followers: Vec<Address> = Vec::new(env);
        for i in 0..followers.len() {
            if let Some(f) = followers.get(i) {
                if f != follower {
                    new_followers.push_back(f);
                }
            }
        }
        env.storage()
            .persistent()
            .set(&ExtensionKey::ProjectFollowers(project_id), &new_followers);

        let count: u32 = new_followers.len();
        env.storage()
            .persistent()
            .set(&ExtensionKey::FollowerCount(project_id), &count);

        let mut subscriptions: Vec<u64> = env
            .storage()
            .persistent()
            .get(&ExtensionKey::UserSubscriptions(follower.clone()))
            .unwrap_or_else(|| Vec::new(env));

        let mut new_subscriptions: Vec<u64> = Vec::new(env);
        for i in 0..subscriptions.len() {
            if let Some(pid) = subscriptions.get(i) {
                if pid != project_id {
                    new_subscriptions.push_back(pid);
                }
            }
        }
        env.storage().persistent().set(
            &ExtensionKey::UserSubscriptions(follower.clone()),
            &new_subscriptions,
        );

        StorageManager::extend_project_ttl(env, project_id);
        StorageManager::extend_followers_ttl(env, project_id);
        StorageManager::extend_user_subscriptions_ttl(env, &follower);

        publish_project_unfollowed_event(env, project_id, follower);
        Ok(())
    }

    pub fn get_follower_count(env: &Env, project_id: u64) -> u32 {
        env.storage()
            .persistent()
            .get(&ExtensionKey::FollowerCount(project_id))
            .unwrap_or(0)
    }

    pub fn is_following(env: &Env, project_id: u64, user: &Address) -> bool {
        let followers: Vec<Address> = env
            .storage()
            .persistent()
            .get(&ExtensionKey::ProjectFollowers(project_id))
            .unwrap_or_else(|| Vec::new(env));
        followers.contains(user)
    }

    pub fn get_project_followers(
        env: &Env,
        project_id: u64,
        start: u32,
        limit: u32,
    ) -> Vec<Address> {
        let effective_limit = if limit == 0 || limit > MAX_PAGE_LIMIT {
            MAX_PAGE_LIMIT
        } else {
            limit
        };

        let followers: Vec<Address> = env
            .storage()
            .persistent()
            .get(&ExtensionKey::ProjectFollowers(project_id))
            .unwrap_or_else(|| Vec::new(env));

        let len = followers.len();
        if start >= len {
            return Vec::new(env);
        }

        let end = core::cmp::min(start.saturating_add(effective_limit), len);
        let mut page = Vec::new(env);
        for i in start..end {
            if let Some(f) = followers.get(i) {
                page.push_back(f);
            }
        }
        page
    }

    pub fn get_user_subscriptions(env: &Env, user: Address, start: u32, limit: u32) -> Vec<u64> {
        let effective_limit = if limit == 0 || limit > MAX_PAGE_LIMIT {
            MAX_PAGE_LIMIT
        } else {
            limit
        };

        let subscriptions: Vec<u64> = env
            .storage()
            .persistent()
            .get(&ExtensionKey::UserSubscriptions(user))
            .unwrap_or_else(|| Vec::new(env));

        let len = subscriptions.len();
        if start >= len {
            return Vec::new(env);
        }

        let end = core::cmp::min(start.saturating_add(effective_limit), len);
        let mut page = Vec::new(env);
        for i in start..end {
            if let Some(pid) = subscriptions.get(i) {
                page.push_back(pid);
            }
        }
        page
    }
}
