//! Notification preference management for project followers (closes #811).
//!
//! ## Overview
//!
//! This module lets users control *how* and *when* they receive notifications
//! about projects they follow. It does **not** deliver notifications directly
//! — it emits on-chain events that off-chain indexers and services consume to
//! fan out messages (email, push, webhook, etc.).
//!
//! ## Notification flow
//!
//! 1. A user follows a project via `subscription_registry::follow_project`.
//! 2. When a project is updated (metadata, verification status, archival, etc.)
//!    the relevant registry calls
//!    `NotificationRegistry::emit_project_notification`. This reads the
//!    follower count and emits a `ProjectUpdateNotificationEvent` with the
//!    update kind.
//! 3. If the user has `DigestFrequency::None`, indexers treat every
//!    `ProjectUpdateNotificationEvent` as an immediate notification per follower.
//! 4. If the user has `DigestFrequency::Daily` or `::Weekly`, the project ID is
//!    appended to `UserDigestQueue(user)` instead of immediate dispatch.
//!    The project owner or an off-chain cron job calls
//!    `flush_digest_queue` periodically; this emits `UserDigestScheduledEvent`
//!    and clears the queue.
//!
//! ## Opt-out
//!
//! Users can globally opt out via `set_notification_prefs(opted_out: true)`.
//! They can also opt out per-project via `set_project_notification_override`.
//!
//! ## Storage layout
//!
//! | Key | Type | Description |
//! |-----|------|-------------|
//! | `NotificationKey::UserNotificationPrefs(address)` | `UserNotificationPrefs` | Global prefs |
//! | `NotificationKey::UserProjectNotifOverride(address, project_id)` | `ProjectNotificationOverride` | Per-project override |
//! | `NotificationKey::UserDigestQueue(address)` | `Vec<u64>` | Pending project IDs for digest |

use crate::constants::MAX_PAGE_LIMIT;
use crate::constants::MAX_PAUSE_SUBSCRIBERS;
use crate::errors::ContractError;
use crate::events::{
    publish_notification_prefs_updated_event, publish_pause_notification_event,
    publish_pause_subscriber_event, publish_project_notif_override_set_event,
    publish_project_update_notification_event, publish_user_digest_scheduled_event,
};
use crate::storage_keys::{NotificationKey, PauseKey};
use crate::storage_manager::StorageManager;
use crate::subscription_registry::SubscriptionRegistry;
use crate::types::{
    DigestFrequency, NotificationKind, ProjectNotificationOverride, UserNotificationPrefs,
};
use soroban_sdk::{Address, Env, String, Vec};

pub struct NotificationRegistry;

impl NotificationRegistry {
    // ── Preference management ─────────────────────────────────────────────

    /// Set (or replace) the global notification preferences for a user.
    ///
    /// Requires auth from the caller. Any address may set their own prefs.
    pub fn set_notification_prefs(
        env: &Env,
        user: Address,
        opted_out: bool,
        notify_on_all: bool,
        digest_frequency: DigestFrequency,
        kinds: Vec<NotificationKind>,
    ) -> Result<(), ContractError> {
        user.require_auth();

        let prefs = UserNotificationPrefs {
            digest_frequency: digest_frequency.clone(),
            notify_on_all,
            opted_out,
            kinds,
        };

        env.storage().persistent().set(
            &NotificationKey::UserNotificationPrefs(user.clone()),
            &prefs,
        );
        StorageManager::extend_notification_prefs_ttl(env, &user);

        publish_notification_prefs_updated_event(env, user, opted_out, digest_frequency);
        Ok(())
    }

    /// Get the global notification preferences for a user.
    ///
    /// Returns `None` when no preferences have been set (caller should
    /// apply their own default logic, e.g. treat as opted-in with no digest).
    pub fn get_notification_prefs(env: &Env, user: Address) -> Option<UserNotificationPrefs> {
        env.storage()
            .persistent()
            .get(&NotificationKey::UserNotificationPrefs(user))
    }

    /// Set a per-project notification override for a user.
    ///
    /// Pass `opted_out: true` to silence notifications for this project
    /// regardless of global preferences. Pass `kinds: None` to fall back
    /// to the global `kinds` list for this project.
    pub fn set_project_notification_override(
        env: &Env,
        user: Address,
        project_id: u64,
        opted_out: bool,
        kinds: Option<Vec<NotificationKind>>,
    ) -> Result<(), ContractError> {
        user.require_auth();

        let override_prefs = ProjectNotificationOverride { opted_out, kinds };

        env.storage().persistent().set(
            &NotificationKey::UserProjectNotifOverride(user.clone(), project_id),
            &override_prefs,
        );
        StorageManager::extend_notification_prefs_ttl(env, &user);

        publish_project_notif_override_set_event(env, user, project_id, opted_out);
        Ok(())
    }

    /// Get the per-project notification override for a user, if set.
    pub fn get_project_notification_override(
        env: &Env,
        user: Address,
        project_id: u64,
    ) -> Option<ProjectNotificationOverride> {
        env.storage()
            .persistent()
            .get(&NotificationKey::UserProjectNotifOverride(user, project_id))
    }

    // ── Notification emission ─────────────────────────────────────────────

    /// Called by project registries when a significant project event occurs.
    ///
    /// Reads the cached follower count and emits a
    /// `ProjectUpdateNotificationEvent`. Off-chain indexers consume this event
    /// to fan out immediate notifications to opted-in followers, or to enqueue
    /// entries for digest followers.
    ///
    /// This is a pure event-emission helper; it does **not** iterate follower
    /// lists on-chain (that would be O(n) and unacceptably expensive on
    /// Soroban). The off-chain layer is responsible for per-user filtering
    /// against `UserNotificationPrefs` / `ProjectNotificationOverride`.
    pub fn emit_project_notification(env: &Env, project_id: u64, update_kind: NotificationKind) {
        let follower_count = SubscriptionRegistry::get_follower_count(env, project_id);
        publish_project_update_notification_event(
            env,
            project_id,
            update_kind.clone(),
            follower_count,
        );
        crate::changelog_generator::ChangelogGenerator::record_project_event(
            env,
            project_id,
            update_kind,
        );
    }

    // ── Pause notifications (issue #901) ───────────────────────────────────

    /// Subscribe an address to emergency-pause notifications.
    ///
    /// When the contract is paused (with or without a reason) a
    /// `CONTRACT/NOTIFY` event is emitted carrying the reason, the estimated
    /// resume time and the number of subscribers.  Off-chain indexers use
    /// that event to fan the alert out to every address in
    /// [`NotificationRegistry::list_pause_subscribers`].
    ///
    /// Subscribing twice is a no-op.
    pub fn subscribe_pause_notifications(
        env: &Env,
        subscriber: Address,
    ) -> Result<(), ContractError> {
        subscriber.require_auth();

        let mut subscribers = Self::list_pause_subscribers(env);
        if subscribers.iter().any(|a| a == subscriber) {
            return Ok(());
        }

        if subscribers.len() >= MAX_PAUSE_SUBSCRIBERS {
            return Err(ContractError::PauseSubscriberLimitExceeded);
        }

        subscribers.push_back(subscriber.clone());
        env.storage()
            .persistent()
            .set(&PauseKey::PauseSubscribers, &subscribers);

        publish_pause_subscriber_event(env, subscriber, true);
        Ok(())
    }

    /// Unsubscribe an address from emergency-pause notifications.
    ///
    /// Unsubscribing an address that is not subscribed is a no-op.
    pub fn unsubscribe_pause_notifications(
        env: &Env,
        subscriber: Address,
    ) -> Result<(), ContractError> {
        subscriber.require_auth();

        let subscribers = Self::list_pause_subscribers(env);
        if !subscribers.iter().any(|a| a == subscriber) {
            return Ok(());
        }

        let mut remaining = Vec::new(env);
        for addr in subscribers.iter() {
            if addr != subscriber {
                remaining.push_back(addr);
            }
        }
        env.storage()
            .persistent()
            .set(&PauseKey::PauseSubscribers, &remaining);

        publish_pause_subscriber_event(env, subscriber, false);
        Ok(())
    }

    /// All addresses currently subscribed to pause notifications.
    pub fn list_pause_subscribers(env: &Env) -> Vec<Address> {
        env.storage()
            .persistent()
            .get(&PauseKey::PauseSubscribers)
            .unwrap_or_else(|| Vec::new(env))
    }

    /// Whether `subscriber` receives pause notifications.
    pub fn is_pause_notification_subscriber(env: &Env, subscriber: &Address) -> bool {
        Self::list_pause_subscribers(env)
            .iter()
            .any(|a| a == subscriber.clone())
    }

    /// Emit the pause notification event (issue #901).
    ///
    /// Called by the emergency-pause module whenever the contract is paused.
    /// The event carries the reason and the estimated resume time so that
    /// subscribers understand *why* the contract is halted.
    pub fn emit_pause_notification(
        env: &Env,
        reason: Option<String>,
        estimated_resume: Option<u64>,
    ) {
        let subscriber_count = Self::list_pause_subscribers(env).len();
        publish_pause_notification_event(env, reason, estimated_resume, subscriber_count);
    }

    // ── Digest queue management ───────────────────────────────────────────

    /// Append a project to the user's digest queue.
    ///
    /// Called when a `ProjectUpdateNotificationEvent` is emitted and the user
    /// is known (off-chain) to prefer digest delivery. This is a low-cost
    /// append; deduplication is the indexer's responsibility.
    ///
    /// The queue is capped at `MAX_PAGE_LIMIT` entries; older entries are
    /// silently dropped when the cap is reached so the ledger entry stays
    /// bounded.
    // Off-chain fanout decides who is queued, so this producer currently has no
    // on-chain caller; kept alongside `flush_digest_queue`/`get_digest_queue`.
    #[allow(dead_code)]
    pub fn enqueue_digest(env: &Env, user: Address, project_id: u64) {
        let mut queue: Vec<u64> = env
            .storage()
            .persistent()
            .get(&NotificationKey::UserDigestQueue(user.clone()))
            .unwrap_or_else(|| Vec::new(env));

        // Deduplication: don't add if already queued.
        if !queue.contains(&project_id) {
            // Evict oldest entry when at capacity.
            if queue.len() >= MAX_PAGE_LIMIT {
                queue.remove(0);
            }
            queue.push_back(project_id);
        }

        env.storage()
            .persistent()
            .set(&NotificationKey::UserDigestQueue(user.clone()), &queue);
        StorageManager::extend_notification_prefs_ttl(env, &user);
    }

    /// Get the current digest queue for a user (paginated).
    pub fn get_digest_queue(env: &Env, user: Address, start_index: u32, limit: u32) -> Vec<u64> {
        let effective_limit = if limit == 0 || limit > MAX_PAGE_LIMIT {
            MAX_PAGE_LIMIT
        } else {
            limit
        };

        let queue: Vec<u64> = env
            .storage()
            .persistent()
            .get(&NotificationKey::UserDigestQueue(user))
            .unwrap_or_else(|| Vec::new(env));

        let len = queue.len();
        if start_index >= len {
            return Vec::new(env);
        }

        let end = core::cmp::min(start_index.saturating_add(effective_limit), len);
        let mut page = Vec::new(env);
        for i in start_index..end {
            if let Some(pid) = queue.get(i) {
                page.push_back(pid);
            }
        }
        page
    }

    /// Flush the digest queue for a user and emit a `UserDigestScheduledEvent`.
    ///
    /// Called by an off-chain cron job or the user themselves on the schedule
    /// matching their `DigestFrequency`. The queue is cleared atomically so
    /// projects are not double-delivered.
    ///
    /// Returns `Ok(())` when the queue was non-empty and was flushed.
    /// Returns `Ok(())` silently when the queue is empty (no-op).
    pub fn flush_digest_queue(env: &Env, user: Address) -> Result<(), ContractError> {
        user.require_auth();

        let queue: Vec<u64> = env
            .storage()
            .persistent()
            .get(&NotificationKey::UserDigestQueue(user.clone()))
            .unwrap_or_else(|| Vec::new(env));

        if queue.is_empty() {
            return Ok(());
        }

        // Read the user's digest frequency preference.
        let frequency: DigestFrequency = env
            .storage()
            .persistent()
            .get(&NotificationKey::UserNotificationPrefs(user.clone()))
            .map(|p: UserNotificationPrefs| p.digest_frequency)
            .unwrap_or(DigestFrequency::None);

        // Clear the queue.
        env.storage()
            .persistent()
            .remove(&NotificationKey::UserDigestQueue(user.clone()));

        publish_user_digest_scheduled_event(env, user, queue, frequency);
        Ok(())
    }
}
