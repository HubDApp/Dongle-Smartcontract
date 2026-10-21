//! Automatic changelog generation from project events (closes #929).
//!
//! ## Event stream
//! Contracts cannot read back their own published events, so every project
//! notification (`NotificationRegistry::emit_project_notification`) is also
//! appended to a per-project on-chain event stream via
//! [`ChangelogGenerator::record_project_event`]. Each record gets a gap-free
//! `seq` starting at 0.
//!
//! ## Generation (on-demand trigger)
//! [`ChangelogGenerator::generate_changelog_from_events`] scans the stream
//! from the project's cursor, turns changelog-worthy events (verification
//! decisions, major updates, archive / reactivate) into
//! [`GeneratedChangelogEntry`] records and advances the cursor. It is
//! permissionless and idempotent: each event is processed exactly once, so an
//! off-chain keeper can call it on a schedule as a background job.
//!
//! ## Summaries
//! [`ChangelogGenerator::generate_changelog_summary`] groups generated
//! entries in `[start, end)` by type, and
//! [`ChangelogGenerator::group_changelog_by_period`] buckets them by time.

use crate::constants::{
    MAX_CHANGELOG_GEN_BATCH, MAX_CHANGELOG_PERIOD_BUCKETS, MAX_CHANGELOG_SUMMARY_SCAN,
};
use crate::errors::ContractError;
use crate::events::publish_changelog_generated_event;
use crate::project_registry::ProjectRegistry;
use crate::storage_keys::ChangelogGenKey;
use crate::types::{
    ChangelogEventType, ChangelogPeriodGroup, ChangelogSummary, ChangelogTypeGroup,
    GeneratedChangelogEntry, NotificationKind, ProjectEventRecord,
};
use soroban_sdk::{Env, Vec};

pub struct ChangelogGenerator;

impl ChangelogGenerator {
    /// Append an event to the project's event stream.
    pub fn record_project_event(env: &Env, project_id: u64, kind: NotificationKind) {
        let seq = Self::get_project_event_count(env, project_id);
        let record = ProjectEventRecord {
            seq,
            kind,
            timestamp: env.ledger().timestamp(),
        };
        env.storage()
            .persistent()
            .set(&ChangelogGenKey::Event(project_id, seq), &record);
        env.storage()
            .persistent()
            .set(&ChangelogGenKey::EventCount(project_id), &(seq + 1));
    }

    /// Map an event to the changelog entry type it produces, if any.
    pub fn changelog_type_for(kind: &NotificationKind) -> Option<ChangelogEventType> {
        match kind {
            NotificationKind::VerificationApproved => Some(ChangelogEventType::VerificationApproved),
            NotificationKind::VerificationRejected => Some(ChangelogEventType::VerificationRejected),
            NotificationKind::VerificationRevoked => Some(ChangelogEventType::VerificationRevoked),
            NotificationKind::ProjectUpdate => Some(ChangelogEventType::MajorUpdate),
            NotificationKind::ProjectArchived => Some(ChangelogEventType::Archived),
            NotificationKind::ProjectReactivated => Some(ChangelogEventType::Reactivated),
            NotificationKind::VerificationExpiringSoon => None,
        }
    }

    /// Process up to `max_events` unprocessed events (capped at
    /// [`MAX_CHANGELOG_GEN_BATCH`]; `0` means the cap) and return the number
    /// of changelog entries generated.
    pub fn generate_changelog_from_events(
        env: &Env,
        project_id: u64,
        max_events: u32,
    ) -> Result<u32, ContractError> {
        ProjectRegistry::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;

        let batch = if max_events == 0 || max_events > MAX_CHANGELOG_GEN_BATCH {
            MAX_CHANGELOG_GEN_BATCH
        } else {
            max_events
        };

        let event_count = Self::get_project_event_count(env, project_id);
        let start = Self::get_changelog_cursor(env, project_id);
        let stop = event_count.min(start + batch as u64);
        if start >= stop {
            return Ok(0);
        }

        let now = env.ledger().timestamp();
        let mut next_id: u64 = env
            .storage()
            .persistent()
            .get(&ChangelogGenKey::NextEntryId)
            .unwrap_or(1);
        let mut entry_ids = Self::get_generated_entry_ids(env, project_id);
        let mut generated: u32 = 0;

        for seq in start..stop {
            let record: Option<ProjectEventRecord> = env
                .storage()
                .persistent()
                .get(&ChangelogGenKey::Event(project_id, seq));
            let Some(record) = record else { continue };
            let Some(event_type) = Self::changelog_type_for(&record.kind) else {
                continue;
            };

            let entry = GeneratedChangelogEntry {
                id: next_id,
                project_id,
                event_type,
                source_event_seq: seq,
                occurred_at: record.timestamp,
                generated_at: now,
            };
            env.storage()
                .persistent()
                .set(&ChangelogGenKey::Entry(next_id), &entry);
            entry_ids.push_back(next_id);
            next_id = next_id
                .checked_add(1)
                .ok_or(ContractError::ArithmeticOverflow)?;
            generated += 1;
        }

        if generated > 0 {
            env.storage()
                .persistent()
                .set(&ChangelogGenKey::NextEntryId, &next_id);
            env.storage()
                .persistent()
                .set(&ChangelogGenKey::ProjectEntries(project_id), &entry_ids);
        }
        env.storage()
            .persistent()
            .set(&ChangelogGenKey::Cursor(project_id), &stop);

        publish_changelog_generated_event(
            env,
            project_id,
            (stop - start) as u32,
            generated,
            stop,
        );
        Ok(generated)
    }

    /// Summarise generated changelog entries whose event occurred in
    /// `[start, end)`, grouped by type.
    pub fn generate_changelog_summary(
        env: &Env,
        project_id: u64,
        start: u64,
        end: u64,
    ) -> Result<ChangelogSummary, ContractError> {
        ProjectRegistry::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;
        if start >= end {
            return Err(ContractError::InvalidChangelogWindow);
        }

        let entries = Self::entries_in_window(env, project_id, start, end);
        let all_types = [
            ChangelogEventType::VerificationApproved,
            ChangelogEventType::VerificationRejected,
            ChangelogEventType::VerificationRevoked,
            ChangelogEventType::MajorUpdate,
            ChangelogEventType::Archived,
            ChangelogEventType::Reactivated,
        ];
        let mut by_type = Vec::new(env);
        for event_type in all_types {
            let mut ids = Vec::new(env);
            for entry in entries.iter() {
                if entry.event_type == event_type {
                    ids.push_back(entry.id);
                }
            }
            if !ids.is_empty() {
                by_type.push_back(ChangelogTypeGroup {
                    event_type,
                    count: ids.len(),
                    entry_ids: ids,
                });
            }
        }

        let unprocessed_events = Self::get_project_event_count(env, project_id)
            .saturating_sub(Self::get_changelog_cursor(env, project_id));

        Ok(ChangelogSummary {
            project_id,
            start,
            end,
            total_entries: entries.len(),
            by_type,
            unprocessed_events,
        })
    }

    /// Bucket generated entries in `[start, end)` into consecutive periods of
    /// `period_seconds`. Empty buckets are omitted.
    pub fn group_changelog_by_period(
        env: &Env,
        project_id: u64,
        start: u64,
        end: u64,
        period_seconds: u64,
    ) -> Result<Vec<ChangelogPeriodGroup>, ContractError> {
        ProjectRegistry::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;
        if start >= end || period_seconds == 0 {
            return Err(ContractError::InvalidChangelogWindow);
        }
        let buckets = (end - start).div_ceil(period_seconds);
        if buckets > MAX_CHANGELOG_PERIOD_BUCKETS as u64 {
            return Err(ContractError::InvalidChangelogWindow);
        }

        // Entries are stored in event order, so occurred_at is non-decreasing
        // and each bucket is a contiguous run.
        let entries = Self::entries_in_window(env, project_id, start, end);
        let mut groups: Vec<ChangelogPeriodGroup> = Vec::new(env);
        for entry in entries.iter() {
            let index = (entry.occurred_at - start) / period_seconds;
            let period_start = start + index * period_seconds;
            let last = groups.len().checked_sub(1);
            match last.and_then(|i| groups.get(i).map(|g| (i, g))) {
                Some((i, mut group)) if group.period_start == period_start => {
                    group.count += 1;
                    group.entry_ids.push_back(entry.id);
                    groups.set(i, group);
                }
                _ => groups.push_back(ChangelogPeriodGroup {
                    period_start,
                    period_end: (period_start + period_seconds).min(end),
                    count: 1,
                    entry_ids: Vec::from_array(env, [entry.id]),
                }),
            }
        }
        Ok(groups)
    }

    /// Generated entries for a project in event order, paginated.
    pub fn get_generated_changelog(
        env: &Env,
        project_id: u64,
        start_index: u32,
        limit: u32,
    ) -> Vec<GeneratedChangelogEntry> {
        let ids = Self::get_generated_entry_ids(env, project_id);
        let page = crate::pagination::paginate(env, &ids, start_index, limit);
        let mut result = Vec::new(env);
        for id in page.iter() {
            if let Some(entry) = Self::get_generated_changelog_entry(env, id) {
                result.push_back(entry);
            }
        }
        result
    }

    pub fn get_generated_changelog_entry(
        env: &Env,
        entry_id: u64,
    ) -> Option<GeneratedChangelogEntry> {
        env.storage()
            .persistent()
            .get(&ChangelogGenKey::Entry(entry_id))
    }

    pub fn get_project_event(env: &Env, project_id: u64, seq: u64) -> Option<ProjectEventRecord> {
        env.storage()
            .persistent()
            .get(&ChangelogGenKey::Event(project_id, seq))
    }

    pub fn get_project_event_count(env: &Env, project_id: u64) -> u64 {
        env.storage()
            .persistent()
            .get(&ChangelogGenKey::EventCount(project_id))
            .unwrap_or(0)
    }

    pub fn get_changelog_cursor(env: &Env, project_id: u64) -> u64 {
        env.storage()
            .persistent()
            .get(&ChangelogGenKey::Cursor(project_id))
            .unwrap_or(0)
    }

    /// Generated entries with `start <= occurred_at < end`, scanning at most
    /// the newest [`MAX_CHANGELOG_SUMMARY_SCAN`] entries.
    fn entries_in_window(
        env: &Env,
        project_id: u64,
        start: u64,
        end: u64,
    ) -> Vec<GeneratedChangelogEntry> {
        let ids = Self::get_generated_entry_ids(env, project_id);
        let first = ids.len().saturating_sub(MAX_CHANGELOG_SUMMARY_SCAN);
        let mut result = Vec::new(env);
        for i in first..ids.len() {
            let Some(id) = ids.get(i) else { continue };
            if let Some(entry) = Self::get_generated_changelog_entry(env, id) {
                if entry.occurred_at >= start && entry.occurred_at < end {
                    result.push_back(entry);
                }
            }
        }
        result
    }

    fn get_generated_entry_ids(env: &Env, project_id: u64) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&ChangelogGenKey::ProjectEntries(project_id))
            .unwrap_or(Vec::new(env))
    }
}
