//! Public contract types for automatic inactivity archival (issue #753).

use crate::types::Project;
use soroban_sdk::{contracttype, Address, Vec};

/// Global keeper-driven auto-archive configuration.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AutoArchiveConfig {
    pub enabled: bool,
    /// Maximum project age without an update before archival eligibility.
    pub inactivity_threshold_secs: u64,
}

/// Delivery state for the owner-facing 30-day pre-archival notice.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AutoArchiveNoticeStatus {
    Pending,
    Delivered,
    Failed,
}

/// Durable owner notice and eligibility timestamp.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AutoArchiveNotice {
    pub project_id: u64,
    pub owner: Address,
    pub last_activity_at: u64,
    pub notified_at: u64,
    pub archive_eligible_at: u64,
    pub status: AutoArchiveNoticeStatus,
}

/// Audit record for an automatic archival and optional owner restoration.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AutoArchiveRecord {
    pub project_id: u64,
    pub owner: Address,
    pub last_activity_at: u64,
    pub notified_at: u64,
    pub archived_at: u64,
    pub archived_by: Address,
    pub restored_at: Option<u64>,
}

/// Result of one bounded keeper archival scan.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AutoArchiveBatchResult {
    pub scanned_projects: u32,
    pub archived_count: u32,
    pub archived_project_ids: Vec<u64>,
    /// Next unvisited project ID, or `None` when the scan reached the end.
    pub next_project_id: Option<u64>,
    pub has_more: bool,
}

/// Cursor-based search over archived projects.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArchivedProjectSearchResult {
    pub projects: Vec<Project>,
    pub next_cursor: Option<u64>,
    pub has_more: bool,
    pub scanned_projects: u32,
}
