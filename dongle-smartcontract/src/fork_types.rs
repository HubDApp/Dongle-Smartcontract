//! Public contract types for project fork and migration relationships (#748).

use crate::types::{ProjectStats, VerificationRecord};
use soroban_sdk::{contracttype, Address, Vec};

/// Evidence that contributed to an on-chain fork/migration detection.
#[contracttype]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ForkSignal {
    /// The parent and child have the same owner address.
    SameOwner,
    /// The projects share at least one maintainer address.
    SharedMaintainer,
    /// Normalized project names are at least 60% edit-similar.
    SimilarName,
    /// Both projects publish the same repository URL.
    SameRepository,
}

/// Read-only result of comparing a likely child project with its parent.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForkDetection {
    pub child_project_id: u64,
    pub parent_project_id: u64,
    pub signals: Vec<ForkSignal>,
    /// Addresses shared by the parent and child (owner and/or maintainers).
    pub shared_team_members: Vec<Address>,
    /// Deterministic confidence score in basis points (0..=10_000).
    pub confidence_bps: u32,
}

/// Provenance-preserving parent data optionally attached to a fork link.
///
/// This records parent review aggregates and the parent's latest verification
/// record without granting the child the parent's authorization or duplicating
/// individual reviews. Consumers can display the inherited provenance while
/// keeping the parent's on-chain records authoritative.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForkDataMerge {
    pub review_stats: ProjectStats,
    pub verification_record: Option<VerificationRecord>,
    pub merged_at: u64,
}

/// Immutable parent relationship for a child project.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ForkRelationship {
    pub child_project_id: u64,
    pub parent_project_id: u64,
    pub signals: Vec<ForkSignal>,
    pub shared_team_members: Vec<Address>,
    pub confidence_bps: u32,
    pub linked_by: Address,
    pub created_at: u64,
    pub merged_data: Option<ForkDataMerge>,
}
