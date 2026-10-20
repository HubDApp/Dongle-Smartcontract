//! Bulk subscription management (closes #932).
//!
//! Operators can create many subscriptions in one call, move a user's
//! subscription list between wallets (export / import), and, as admins, copy
//! one address's subscriptions onto another.
//!
//! ## Deduplication
//!
//! Every bulk entry point runs its input through [`SubscriptionBulk::dedup`]
//! before writing. An entry is a duplicate when it repeats an earlier
//! `(subscriber, project_id)` pair in the same batch, or when the subscriber
//! already follows the project. Duplicates are reported in
//! `BatchSubscriptionResult::duplicates` rather than failing the batch.
//!
//! ## Failure model
//!
//! The batch as a whole fails only on auth, admin or size errors, all checked
//! before any write. A single entry that cannot be applied (for example a
//! missing project) is reported in `BatchSubscriptionResult::failed` without
//! rolling back the rest.

use crate::constants::{MAX_SUBSCRIPTION_BATCH_SIZE, SUBSCRIPTION_EXPORT_FORMAT_VERSION};
use crate::errors::ContractError;
use crate::events::{publish_subscriptions_batched_event, publish_subscriptions_cloned_event};
use crate::storage_keys::ExtensionKey;
use crate::subscription_registry::SubscriptionRegistry;
use crate::types::{
    BatchSubscriptionResult, EventSubscription, SubscriptionExport, SubscriptionFailure,
};
use soroban_sdk::{Address, Env, Vec};

pub struct SubscriptionBulk;

impl SubscriptionBulk {
    /// Subscribe every `(subscriber, project_id)` pair in `subscriptions`.
    ///
    /// Each distinct subscriber must authorize the call once, however many
    /// entries name them.
    pub fn subscribe_to_events_batch(
        env: &Env,
        subscriptions: Vec<EventSubscription>,
    ) -> Result<BatchSubscriptionResult, ContractError> {
        if subscriptions.is_empty() || subscriptions.len() > MAX_SUBSCRIPTION_BATCH_SIZE {
            return Err(ContractError::InvalidInput);
        }

        let mut subscribers: Vec<Address> = Vec::new(env);
        for sub in subscriptions.iter() {
            if !subscribers.contains(&sub.subscriber) {
                subscribers.push_back(sub.subscriber.clone());
            }
        }
        for subscriber in subscribers.iter() {
            subscriber.require_auth();
        }

        let result = Self::apply_batch(env, subscriptions);
        publish_subscriptions_batched_event(
            env,
            result.subscribed.len(),
            result.duplicates.len(),
            result.failed.len(),
        );
        Ok(result)
    }

    /// Snapshot `subscriber`'s full subscription list in a portable format.
    pub fn export_subscriptions(env: &Env, subscriber: Address) -> SubscriptionExport {
        let project_ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&ExtensionKey::UserSubscriptions(subscriber.clone()))
            .unwrap_or_else(|| Vec::new(env));

        SubscriptionExport {
            format_version: SUBSCRIPTION_EXPORT_FORMAT_VERSION,
            subscriber,
            project_ids,
            exported_at: env.ledger().timestamp(),
        }
    }

    /// Re-create the subscriptions in `export` for `subscriber`.
    ///
    /// `subscriber` need not match `export.subscriber`, so a user can move
    /// their list to a new wallet. Subscription lists are public, so importing
    /// another address's export reveals nothing.
    pub fn import_subscriptions(
        env: &Env,
        subscriber: Address,
        export: SubscriptionExport,
    ) -> Result<BatchSubscriptionResult, ContractError> {
        subscriber.require_auth();

        if export.format_version != SUBSCRIPTION_EXPORT_FORMAT_VERSION {
            return Err(ContractError::InvalidInput);
        }
        if export.project_ids.len() > MAX_SUBSCRIPTION_BATCH_SIZE {
            return Err(ContractError::InvalidInput);
        }

        let result = Self::apply_batch(env, Self::pairs_for(env, &subscriber, &export.project_ids));
        publish_subscriptions_batched_event(
            env,
            result.subscribed.len(),
            result.duplicates.len(),
            result.failed.len(),
        );
        Ok(result)
    }

    /// Admin-only: subscribe `target` to every project `source` follows.
    ///
    /// `source` keeps its subscriptions. Projects `target` already follows
    /// are reported as duplicates.
    pub fn clone_subscriptions_from_address(
        env: &Env,
        admin: Address,
        source: Address,
        target: Address,
    ) -> Result<BatchSubscriptionResult, ContractError> {
        admin.require_auth();

        if !crate::admin_manager::AdminManager::is_admin(env, &admin) {
            return Err(ContractError::AdminOnly);
        }
        if source == target {
            return Err(ContractError::InvalidInput);
        }

        let project_ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&ExtensionKey::UserSubscriptions(source.clone()))
            .unwrap_or_else(|| Vec::new(env));
        if project_ids.len() > MAX_SUBSCRIPTION_BATCH_SIZE {
            return Err(ContractError::InvalidInput);
        }

        let result = Self::apply_batch(env, Self::pairs_for(env, &target, &project_ids));
        publish_subscriptions_cloned_event(
            env,
            admin,
            source,
            target,
            result.subscribed.len(),
            result.duplicates.len(),
            result.failed.len(),
        );
        Ok(result)
    }

    /// Split `subscriptions` into entries still to apply and duplicates.
    ///
    /// Order is preserved; the first occurrence of a pair wins.
    pub fn dedup(
        env: &Env,
        subscriptions: &Vec<EventSubscription>,
    ) -> (Vec<EventSubscription>, Vec<EventSubscription>) {
        let mut unique = Vec::new(env);
        let mut duplicates = Vec::new(env);
        for sub in subscriptions.iter() {
            if unique.contains(&sub)
                || SubscriptionRegistry::is_following(env, sub.project_id, &sub.subscriber)
            {
                duplicates.push_back(sub);
            } else {
                unique.push_back(sub);
            }
        }
        (unique, duplicates)
    }

    fn apply_batch(env: &Env, subscriptions: Vec<EventSubscription>) -> BatchSubscriptionResult {
        let (unique, duplicates) = Self::dedup(env, &subscriptions);

        let mut subscribed = Vec::new(env);
        let mut failed = Vec::new(env);
        for sub in unique.iter() {
            match SubscriptionRegistry::apply_follow(env, sub.project_id, sub.subscriber.clone()) {
                Ok(()) => subscribed.push_back(sub),
                Err(err) => failed.push_back(SubscriptionFailure {
                    subscription: sub,
                    error_code: err as u32,
                }),
            }
        }

        BatchSubscriptionResult {
            subscribed,
            duplicates,
            failed,
        }
    }

    fn pairs_for(env: &Env, subscriber: &Address, project_ids: &Vec<u64>) -> Vec<EventSubscription> {
        let mut out = Vec::new(env);
        for project_id in project_ids.iter() {
            out.push_back(EventSubscription {
                subscriber: subscriber.clone(),
                project_id,
            });
        }
        out
    }
}
