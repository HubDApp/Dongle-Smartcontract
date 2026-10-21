//! Community voting on collection membership — issue #907.
//!
//! Curation is decentralized: any community member with a track record may
//! vote on whether a project belongs to a collection. When the vote crosses
//! the collection's approval threshold (default **75 %**) the project is
//! added to the collection automatically; when the rejection ratio crosses the
//! same threshold the project is rejected (and removed if it was a member).
//!
//! # Anti-gaming rules
//!
//! 1. **One vote per address** — votes are keyed by
//!    `(collection_id, project_id, voter)`, so a voter cannot re-vote (or flip
//!    their vote) on the same proposal.
//! 2. **Reputation requirement** — a voter must have contributed to the
//!    registry before their vote counts: at least `MIN_VOTER_REVIEWS`
//!    authored reviews, at least `MIN_VOTER_REPUTATION_POINTS` reviewer
//!    points, or a verified reviewer identity.
//! 3. **No self-promotion** — the owner of the project being voted on cannot
//!    vote on it.
//! 4. **Quorum** — a proposal is only auto-resolved once at least
//!    `MIN_COLLECTION_VOTES_FOR_RESOLUTION` votes have been cast, so a single
//!    enthusiastic voter can never curate a collection on their own.
//! 5. **Finality** — a resolved proposal cannot be voted on again.

use crate::admin_manager::AdminManager;
use crate::constants::{
    DEFAULT_COLLECTION_VOTE_THRESHOLD_BPS, MAX_PROJECTS_PER_COLLECTION,
    MIN_COLLECTION_VOTES_FOR_RESOLUTION, MIN_VOTER_REPUTATION_POINTS, MIN_VOTER_REVIEWS,
};
use crate::errors::ContractError;
use crate::events::{
    publish_collection_membership_resolved_event, publish_collection_membership_vote_event,
    publish_project_added_to_collection_event, publish_project_removed_from_collection_event,
};
use crate::pagination::paginate;
use crate::project_registry::ProjectRegistry;
use crate::storage_keys::{CollectionVoteKey, StorageKey};
use crate::trust_and_safety::TrustAndSafety;
use crate::types::{
    Collection, CollectionMembershipVote, CollectionVoteTally, MembershipVoteOutcome, VoteDirection,
};
use crate::utils::Utils;
use soroban_sdk::{Address, Env, Vec};

pub struct CollectionVoting;

impl CollectionVoting {
    // ── Configuration ────────────────────────────────────────────────────

    /// Enable / disable community voting for a collection and set the approval
    /// threshold (basis points).
    ///
    /// Only the collection owner or an admin may call this.
    pub fn set_collection_voting(
        env: &Env,
        caller: Address,
        collection_id: u64,
        enabled: bool,
        threshold_bps: u32,
    ) -> Result<(), ContractError> {
        caller.require_auth();

        let mut collection = Self::require_collection(env, collection_id)?;
        if collection.owner != caller && !AdminManager::is_admin(env, &caller) {
            // Only the collection owner or an admin may change the voting
            // configuration of a collection.
            return Err(ContractError::Unauthorized);
        }

        if threshold_bps == 0 || threshold_bps > 10_000 {
            return Err(ContractError::InvalidVoteThreshold);
        }

        collection.voting_enabled = enabled;
        collection.vote_threshold_bps = threshold_bps;
        collection.updated_at = env.ledger().timestamp();
        env.storage()
            .persistent()
            .set(&StorageKey::Collection(collection_id), &collection);

        Ok(())
    }

    // ── Voting ───────────────────────────────────────────────────────────

    /// Cast a community vote on the membership of `project_id` in
    /// `collection_id`.
    ///
    /// Returns the updated tally. When the vote crosses the collection's
    /// threshold the membership is updated in the same transaction.
    pub fn vote_on_collection_membership(
        env: &Env,
        voter: Address,
        collection_id: u64,
        project_id: u64,
        direction: VoteDirection,
    ) -> Result<CollectionVoteTally, ContractError> {
        voter.require_auth();

        let collection = Self::require_collection(env, collection_id)?;
        if !collection.voting_enabled {
            return Err(ContractError::CollectionVotingDisabled);
        }

        let project =
            ProjectRegistry::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;

        // ── Anti-gaming: no self-promotion ───────────────────────────────
        if project.owner == voter {
            return Err(ContractError::Unauthorized);
        }

        // ── Anti-gaming: reputation requirement ─────────────────────────
        Self::require_reputation(env, &voter)?;

        // ── Anti-gaming: one vote per address per proposal ───────────────
        let vote_key = CollectionVoteKey::Vote(collection_id, project_id, voter.clone());
        if env.storage().persistent().has(&vote_key) {
            return Err(ContractError::CollectionVoteAlreadyCast);
        }

        // ── Anti-gaming: a resolved proposal is final ────────────────────
        let already_resolved: bool = env
            .storage()
            .persistent()
            .get(&CollectionVoteKey::Resolved(collection_id, project_id))
            .unwrap_or(false);
        if already_resolved {
            return Err(ContractError::CollectionVoteAlreadyResolved);
        }

        let vote = CollectionMembershipVote {
            collection_id,
            project_id,
            voter: voter.clone(),
            direction,
            created_at: env.ledger().timestamp(),
        };
        env.storage().persistent().set(&vote_key, &vote);

        let mut approve_votes: u32 = env
            .storage()
            .persistent()
            .get(&CollectionVoteKey::ApproveCount(collection_id, project_id))
            .unwrap_or(0);
        let mut reject_votes: u32 = env
            .storage()
            .persistent()
            .get(&CollectionVoteKey::RejectCount(collection_id, project_id))
            .unwrap_or(0);

        match direction {
            VoteDirection::Approve => approve_votes = approve_votes.saturating_add(1),
            VoteDirection::Reject => reject_votes = reject_votes.saturating_add(1),
        }

        env.storage().persistent().set(
            &CollectionVoteKey::ApproveCount(collection_id, project_id),
            &approve_votes,
        );
        env.storage().persistent().set(
            &CollectionVoteKey::RejectCount(collection_id, project_id),
            &reject_votes,
        );

        Self::track_voted_project(env, collection_id, project_id);

        publish_collection_membership_vote_event(
            env,
            collection_id,
            project_id,
            voter.clone(),
            direction,
            approve_votes,
            reject_votes,
        );

        let total_votes = approve_votes.saturating_add(reject_votes);
        let approval_bps = Self::ratio_bps(approve_votes, total_votes);
        let rejection_bps = Self::ratio_bps(reject_votes, total_votes);
        let threshold_bps = if collection.vote_threshold_bps == 0 {
            DEFAULT_COLLECTION_VOTE_THRESHOLD_BPS
        } else {
            collection.vote_threshold_bps
        };

        let mut outcome = MembershipVoteOutcome::Pending;
        let mut resolved = false;

        if total_votes >= MIN_COLLECTION_VOTES_FOR_RESOLUTION {
            if approval_bps >= threshold_bps {
                Self::add_project_to_collection(env, collection_id, project_id, &voter);
                outcome = MembershipVoteOutcome::Approved;
                resolved = true;
            } else if rejection_bps >= threshold_bps {
                Self::remove_project_from_collection(env, collection_id, project_id, &voter);
                outcome = MembershipVoteOutcome::Rejected;
                resolved = true;
            }
        }

        if resolved {
            env.storage().persistent().set(
                &CollectionVoteKey::Resolved(collection_id, project_id),
                &true,
            );
            publish_collection_membership_resolved_event(
                env,
                collection_id,
                project_id,
                outcome,
                approve_votes,
                reject_votes,
                approval_bps,
            );
        }

        Ok(CollectionVoteTally {
            collection_id,
            project_id,
            approve_votes,
            reject_votes,
            total_votes,
            approval_bps,
            threshold_bps,
            outcome,
            resolved,
        })
    }

    // ── Reads ────────────────────────────────────────────────────────────

    /// Aggregated vote state for one `(collection_id, project_id)` pair.
    pub fn get_collection_membership_votes(
        env: &Env,
        collection_id: u64,
        project_id: u64,
    ) -> CollectionVoteTally {
        let approve_votes: u32 = env
            .storage()
            .persistent()
            .get(&CollectionVoteKey::ApproveCount(collection_id, project_id))
            .unwrap_or(0);
        let reject_votes: u32 = env
            .storage()
            .persistent()
            .get(&CollectionVoteKey::RejectCount(collection_id, project_id))
            .unwrap_or(0);
        let total_votes = approve_votes.saturating_add(reject_votes);
        let approval_bps = Self::ratio_bps(approve_votes, total_votes);

        let threshold_bps = match Self::require_collection(env, collection_id) {
            Ok(c) => {
                if c.vote_threshold_bps == 0 {
                    DEFAULT_COLLECTION_VOTE_THRESHOLD_BPS
                } else {
                    c.vote_threshold_bps
                }
            }
            Err(_) => DEFAULT_COLLECTION_VOTE_THRESHOLD_BPS,
        };

        let resolved: bool = env
            .storage()
            .persistent()
            .get(&CollectionVoteKey::Resolved(collection_id, project_id))
            .unwrap_or(false);

        let outcome = if resolved {
            let rejection_bps = Self::ratio_bps(reject_votes, total_votes);
            if approval_bps >= threshold_bps {
                MembershipVoteOutcome::Approved
            } else if rejection_bps >= threshold_bps {
                MembershipVoteOutcome::Rejected
            } else {
                MembershipVoteOutcome::Pending
            }
        } else {
            MembershipVoteOutcome::Pending
        };

        CollectionVoteTally {
            collection_id,
            project_id,
            approve_votes,
            reject_votes,
            total_votes,
            approval_bps,
            threshold_bps,
            outcome,
            resolved,
        }
    }

    /// The vote of a single voter, if any.
    pub fn get_collection_membership_vote(
        env: &Env,
        collection_id: u64,
        project_id: u64,
        voter: Address,
    ) -> Option<CollectionMembershipVote> {
        env.storage()
            .persistent()
            .get(&CollectionVoteKey::Vote(collection_id, project_id, voter))
    }

    /// List the projects that received votes in a collection.
    pub fn list_voted_projects(
        env: &Env,
        collection_id: u64,
        start_index: u32,
        limit: u32,
    ) -> Vec<u64> {
        let ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&CollectionVoteKey::VotedProjects(collection_id))
            .unwrap_or_else(|| Vec::new(env));
        paginate(env, &ids, start_index, limit)
    }

    // ── Internal helpers ─────────────────────────────────────────────────

    fn require_collection(env: &Env, collection_id: u64) -> Result<Collection, ContractError> {
        env.storage()
            .persistent()
            .get(&StorageKey::Collection(collection_id))
            .ok_or(ContractError::CollectionNotFound)
    }

    /// Reputation gate: the voter must have contributed to the registry.
    ///
    /// A voter qualifies when they authored at least `MIN_VOTER_REVIEWS`
    /// reviews, accumulated at least `MIN_VOTER_REPUTATION_POINTS` reviewer
    /// points, or have a verified reviewer identity. Admins always qualify.
    fn require_reputation(env: &Env, voter: &Address) -> Result<(), ContractError> {
        if AdminManager::is_admin(env, voter) {
            return Ok(());
        }

        let reviews: Vec<u64> = env
            .storage()
            .persistent()
            .get(&StorageKey::UserReviews(voter.clone()))
            .unwrap_or_else(|| Vec::new(env));
        if reviews.len() >= MIN_VOTER_REVIEWS {
            return Ok(());
        }

        let points = TrustAndSafety::get_reviewer_points(env, voter);
        if points.total_points >= MIN_VOTER_REPUTATION_POINTS {
            return Ok(());
        }

        let identity = TrustAndSafety::get_reviewer_identity(env, voter);
        if identity.is_verified {
            return Ok(());
        }

        Err(ContractError::CollectionVoteInsufficientReputation)
    }

    fn track_voted_project(env: &Env, collection_id: u64, project_id: u64) {
        let mut projects: Vec<u64> = env
            .storage()
            .persistent()
            .get(&CollectionVoteKey::VotedProjects(collection_id))
            .unwrap_or_else(|| Vec::new(env));
        if !projects.iter().any(|p| p == project_id) {
            projects.push_back(project_id);
            env.storage()
                .persistent()
                .set(&CollectionVoteKey::VotedProjects(collection_id), &projects);
        }
    }

    fn add_project_to_collection(env: &Env, collection_id: u64, project_id: u64, actor: &Address) {
        let mut members: Vec<u64> = env
            .storage()
            .persistent()
            .get(&StorageKey::CollectionProjectIds(collection_id))
            .unwrap_or_else(|| Vec::new(env));

        if members.iter().any(|m| m == project_id) {
            return;
        }
        if members.len() >= MAX_PROJECTS_PER_COLLECTION {
            return;
        }
        members.push_back(project_id);
        env.storage()
            .persistent()
            .set(&StorageKey::CollectionProjectIds(collection_id), &members);
        publish_project_added_to_collection_event(env, collection_id, project_id, actor.clone());
    }

    fn remove_project_from_collection(
        env: &Env,
        collection_id: u64,
        project_id: u64,
        actor: &Address,
    ) {
        let members: Vec<u64> = env
            .storage()
            .persistent()
            .get(&StorageKey::CollectionProjectIds(collection_id))
            .unwrap_or_else(|| Vec::new(env));
        if !members.iter().any(|m| m == project_id) {
            return;
        }
        let updated = Utils::remove_item_from_vec(env, &members, &project_id);
        env.storage()
            .persistent()
            .set(&StorageKey::CollectionProjectIds(collection_id), &updated);
        publish_project_removed_from_collection_event(
            env,
            collection_id,
            project_id,
            actor.clone(),
        );
    }

    /// Ratio of `part` to `total`, in basis points.
    fn ratio_bps(part: u32, total: u32) -> u32 {
        if total == 0 {
            return 0;
        }
        (part as u64 * 10_000u64 / total as u64) as u32
    }
}
