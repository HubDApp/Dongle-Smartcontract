//! On-chain subscriptions to contract events (closes #931).
//!
//! Off-chain systems (notification relays, indexers, bots) register which
//! event families they care about with [`EventSubscriptionRegistry::subscribe_to_events`].
//! Subscriptions are stored on-chain so a relay can query who to push each
//! event to, either per topic ([`EventSubscriptionRegistry::get_subscriptions_for_topic`])
//! or per address ([`EventSubscriptionRegistry::get_subscriptions_for_address`],
//! admin-only).
//!
//! This is distinct from project follows (`subscription_registry`), which
//! track user interest in a single project.
//!
//! ## Storage layout
//!
//! | Key | Type | Description |
//! |-----|------|-------------|
//! | `EventSubscriptionKey::Subscription(id)` | `EventSubscription` | Subscription record |
//! | `EventSubscriptionKey::AddressSubscriptions(address)` | `Vec<u64>` | IDs owned by an address |
//! | `EventSubscriptionKey::TopicSubscriptions(topic)` | `Vec<u64>` | IDs listening to a topic |
//!
//! Both indexes are updated atomically with the record on subscribe,
//! filter update and unsubscribe.

use crate::auth::require_admin_auth;
use crate::constants::{
    MAX_EVENT_FILTER_PROJECTS, MAX_EVENT_SUBSCRIPTIONS_PER_ADDRESS, MAX_SUBSCRIPTIONS_PER_TOPIC,
};
use crate::errors::ContractError;
use crate::events::{publish_event_subscription_changed_event, publish_event_unsubscribed_event};
use crate::pagination::paginate;
use crate::project_registry::ProjectRegistry;
use crate::storage_keys::EventSubscriptionKey;
use crate::types::{EventFilters, EventSubscription, EventTopic};
use crate::utils::Utils;
use soroban_sdk::{symbol_short, Address, Env, Vec};

pub struct EventSubscriptionRegistry;

impl EventSubscriptionRegistry {
    /// Subscribe `address` to the given event topics. Returns the new
    /// subscription ID.
    pub fn subscribe_to_events(
        env: &Env,
        address: Address,
        topics: Vec<EventTopic>,
        filters: EventFilters,
    ) -> Result<u64, ContractError> {
        address.require_auth();
        Self::validate(env, &topics, &filters)?;

        let mut owned = Self::get_address_subscription_ids(env, &address);
        if owned.len() >= MAX_EVENT_SUBSCRIPTIONS_PER_ADDRESS {
            return Err(ContractError::EventSubscriptionLimitReached);
        }

        let id: u64 = env
            .storage()
            .persistent()
            .get(&EventSubscriptionKey::NextId)
            .unwrap_or(1);
        let now = env.ledger().timestamp();
        let subscription = EventSubscription {
            id,
            address: address.clone(),
            event_topics: topics.clone(),
            filters,
            created_at: now,
            updated_at: now,
        };

        for topic in topics.iter() {
            Self::add_to_topic(env, topic, id)?;
        }
        owned.push_back(id);
        env.storage()
            .persistent()
            .set(&EventSubscriptionKey::AddressSubscriptions(address), &owned);
        env.storage()
            .persistent()
            .set(&EventSubscriptionKey::Subscription(id), &subscription);
        env.storage().persistent().set(
            &EventSubscriptionKey::NextId,
            &id.checked_add(1).ok_or(ContractError::ArithmeticOverflow)?,
        );

        publish_event_subscription_changed_event(env, symbol_short!("SUBSCRIBE"), &subscription);
        Ok(id)
    }

    /// Replace a subscription's topics and filters (subscriber only).
    pub fn update_event_subscription(
        env: &Env,
        address: Address,
        subscription_id: u64,
        topics: Vec<EventTopic>,
        filters: EventFilters,
    ) -> Result<(), ContractError> {
        address.require_auth();
        let mut subscription = Self::get_owned(env, &address, subscription_id)?;
        Self::validate(env, &topics, &filters)?;

        for old in subscription.event_topics.iter() {
            if !topics.contains(old) {
                Self::remove_from_topic(env, old, subscription_id);
            }
        }
        for new in topics.iter() {
            if !subscription.event_topics.contains(new) {
                Self::add_to_topic(env, new, subscription_id)?;
            }
        }

        subscription.event_topics = topics;
        subscription.filters = filters;
        subscription.updated_at = env.ledger().timestamp();
        env.storage()
            .persistent()
            .set(&EventSubscriptionKey::Subscription(subscription_id), &subscription);

        publish_event_subscription_changed_event(env, symbol_short!("UPDATED"), &subscription);
        Ok(())
    }

    /// Delete a subscription (subscriber only).
    pub fn unsubscribe_from_events(
        env: &Env,
        address: Address,
        subscription_id: u64,
    ) -> Result<(), ContractError> {
        address.require_auth();
        let subscription = Self::get_owned(env, &address, subscription_id)?;

        for topic in subscription.event_topics.iter() {
            Self::remove_from_topic(env, topic, subscription_id);
        }
        let owned = Utils::remove_item_from_vec(
            env,
            &Self::get_address_subscription_ids(env, &address),
            &subscription_id,
        );
        env.storage().persistent().set(
            &EventSubscriptionKey::AddressSubscriptions(address.clone()),
            &owned,
        );
        env.storage()
            .persistent()
            .remove(&EventSubscriptionKey::Subscription(subscription_id));

        publish_event_unsubscribed_event(env, subscription_id, address);
        Ok(())
    }

    /// All subscriptions held by `address`. Admin-only.
    pub fn get_subscriptions_for_address(
        env: &Env,
        admin: Address,
        address: Address,
    ) -> Result<Vec<EventSubscription>, ContractError> {
        require_admin_auth(env, &admin)?;
        Ok(Self::load_all(
            env,
            &Self::get_address_subscription_ids(env, &address),
        ))
    }

    /// Subscriptions listening to `topic`, paginated in subscription order.
    pub fn get_subscriptions_for_topic(
        env: &Env,
        topic: EventTopic,
        start_index: u32,
        limit: u32,
    ) -> Vec<EventSubscription> {
        let ids = Self::get_topic_subscription_ids(env, topic);
        Self::load_all(env, &paginate(env, &ids, start_index, limit))
    }

    pub fn get_event_subscription(env: &Env, subscription_id: u64) -> Option<EventSubscription> {
        env.storage()
            .persistent()
            .get(&EventSubscriptionKey::Subscription(subscription_id))
    }

    fn validate(
        env: &Env,
        topics: &Vec<EventTopic>,
        filters: &EventFilters,
    ) -> Result<(), ContractError> {
        if topics.is_empty() || Self::has_duplicates(topics) {
            return Err(ContractError::InvalidEventSubscription);
        }
        if filters.project_ids.len() > MAX_EVENT_FILTER_PROJECTS
            || Self::has_duplicates(&filters.project_ids)
        {
            return Err(ContractError::InvalidEventSubscription);
        }
        for project_id in filters.project_ids.iter() {
            ProjectRegistry::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;
        }
        Ok(())
    }

    fn has_duplicates<T>(items: &Vec<T>) -> bool
    where
        T: soroban_sdk::IntoVal<Env, soroban_sdk::Val>
            + soroban_sdk::TryFromVal<Env, soroban_sdk::Val>
            + PartialEq,
    {
        let len = items.len();
        for i in 0..len {
            for j in (i + 1)..len {
                if items.get(i) == items.get(j) {
                    return true;
                }
            }
        }
        false
    }

    fn get_owned(
        env: &Env,
        address: &Address,
        subscription_id: u64,
    ) -> Result<EventSubscription, ContractError> {
        Self::get_event_subscription(env, subscription_id)
            .filter(|s| s.address == *address)
            .ok_or(ContractError::EventSubscriptionNotFound)
    }

    fn add_to_topic(env: &Env, topic: EventTopic, subscription_id: u64) -> Result<(), ContractError> {
        let mut ids = Self::get_topic_subscription_ids(env, topic);
        if ids.len() >= MAX_SUBSCRIPTIONS_PER_TOPIC {
            return Err(ContractError::EventSubscriptionLimitReached);
        }
        ids.push_back(subscription_id);
        env.storage()
            .persistent()
            .set(&EventSubscriptionKey::TopicSubscriptions(topic), &ids);
        Ok(())
    }

    fn remove_from_topic(env: &Env, topic: EventTopic, subscription_id: u64) {
        let ids = Utils::remove_item_from_vec(
            env,
            &Self::get_topic_subscription_ids(env, topic),
            &subscription_id,
        );
        env.storage()
            .persistent()
            .set(&EventSubscriptionKey::TopicSubscriptions(topic), &ids);
    }

    fn load_all(env: &Env, ids: &Vec<u64>) -> Vec<EventSubscription> {
        let mut result = Vec::new(env);
        for id in ids.iter() {
            if let Some(subscription) = Self::get_event_subscription(env, id) {
                result.push_back(subscription);
            }
        }
        result
    }

    fn get_address_subscription_ids(env: &Env, address: &Address) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&EventSubscriptionKey::AddressSubscriptions(address.clone()))
            .unwrap_or(Vec::new(env))
    }

    fn get_topic_subscription_ids(env: &Env, topic: EventTopic) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&EventSubscriptionKey::TopicSubscriptions(topic))
            .unwrap_or(Vec::new(env))
    }
}
