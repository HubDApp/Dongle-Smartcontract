//! Verification request assignment to specific admins, expertise routing,
//! accept/decline workflows, assignment history tracking, and SLA escalation.

use crate::admin_action_log::AdminActionLog;
use crate::admin_manager::AdminManager;
use crate::auth::require_admin_auth;
use crate::constants::{
    DEFAULT_VERIFICATION_SLA_SECS, MAX_ASSIGNMENT_REASON_LEN, MAX_EXPERTISE_LEN,
    MAX_EXPERTISE_TAGS_PER_ADMIN, MAX_VERIFICATION_SLA_SECS, MIN_VERIFICATION_SLA_SECS,
};
use crate::errors::ContractError;
use crate::events::{
    publish_admin_expertise_set_event, publish_verification_assigned_event,
    publish_verification_assigned_with_expertise_event,
    publish_verification_assignment_accepted_event, publish_verification_assignment_declined_event,
    publish_verification_assignment_escalated_event, publish_verification_sla_set_event,
};
use crate::storage_keys::{AssignmentKey, StorageKey};
use crate::types::{
    AdminActionType, AdminWorkload, VerificationAssignment, VerificationAssignmentStatus,
    VerificationStatus,
};
use crate::verification_registry::storage::VerificationRegistry;
use soroban_sdk::{Address, Env, String, Vec};

pub struct VerificationAssignmentRegistry;

impl VerificationAssignmentRegistry {
    // ── SLA Management ────────────────────────────────────────────────────────

    /// Get configured review SLA duration in seconds, or default (3 days).
    pub fn get_verification_sla(env: &Env) -> u64 {
        env.storage()
            .persistent()
            .get(&AssignmentKey::VerificationSlaDuration)
            .unwrap_or(DEFAULT_VERIFICATION_SLA_SECS)
    }

    /// Admin: configure the review SLA duration (in seconds).
    pub fn set_verification_sla(
        env: &Env,
        admin: Address,
        sla_seconds: u64,
    ) -> Result<(), ContractError> {
        require_admin_auth(env, &admin)?;

        if sla_seconds < MIN_VERIFICATION_SLA_SECS || sla_seconds > MAX_VERIFICATION_SLA_SECS {
            return Err(ContractError::InvalidInput);
        }

        env.storage()
            .persistent()
            .set(&AssignmentKey::VerificationSlaDuration, &sla_seconds);

        publish_verification_sla_set_event(env, admin.clone(), sla_seconds);

        AdminActionLog::record_action(
            env,
            admin,
            AdminActionType::VerificationSlaSet,
            None,
            None,
            None,
        );

        Ok(())
    }

    // ── Admin Expertise Management ────────────────────────────────────────────

    /// Admin: set or update the specialized expertise domains for an admin.
    pub fn set_admin_expertise(
        env: &Env,
        caller: Address,
        admin: Address,
        expertise: Vec<String>,
    ) -> Result<(), ContractError> {
        require_admin_auth(env, &caller)?;

        if !AdminManager::is_admin(env, &admin) {
            return Err(ContractError::AdminNotFound);
        }

        let count = expertise.len() as usize;
        if count > MAX_EXPERTISE_TAGS_PER_ADMIN as usize {
            return Err(ContractError::InvalidInput);
        }

        // Validate each expertise tag and ensure no duplicates
        for i in 0..expertise.len() {
            let tag = expertise.get(i).unwrap();
            let len = tag.len() as usize;
            if len == 0 || len > MAX_EXPERTISE_LEN {
                return Err(ContractError::InvalidInput);
            }
            for j in (i + 1)..expertise.len() {
                if expertise.get(j).unwrap() == tag {
                    return Err(ContractError::InvalidInput);
                }
            }
        }

        // Remove old tags from inverted index
        let old_expertise = Self::get_admin_expertise(env, admin.clone());
        for i in 0..old_expertise.len() {
            let old_tag = old_expertise.get(i).unwrap();
            Self::remove_admin_from_expertise_index(env, &old_tag, &admin);
        }

        // Add new tags to inverted index
        for i in 0..expertise.len() {
            let new_tag = expertise.get(i).unwrap();
            Self::add_admin_to_expertise_index(env, &new_tag, &admin);
        }

        env.storage()
            .persistent()
            .set(&AssignmentKey::AdminExpertise(admin.clone()), &expertise);

        publish_admin_expertise_set_event(env, caller.clone(), admin.clone(), count as u32);

        AdminActionLog::record_action(
            env,
            caller,
            AdminActionType::AdminExpertiseSet,
            None,
            Some(admin),
            None,
        );

        Ok(())
    }

    /// Get the expertise tags registered for an admin.
    pub fn get_admin_expertise(env: &Env, admin: Address) -> Vec<String> {
        env.storage()
            .persistent()
            .get(&AssignmentKey::AdminExpertise(admin))
            .unwrap_or_else(|| Vec::new(env))
    }

    /// Check if an admin possesses a given expertise domain.
    pub fn admin_has_expertise(env: &Env, admin: &Address, expertise: &String) -> bool {
        let tags = Self::get_admin_expertise(env, admin.clone());
        for i in 0..tags.len() {
            if let Some(tag) = tags.get(i) {
                if tag == *expertise {
                    return true;
                }
            }
        }
        false
    }

    /// Get all current admins that have the specified expertise.
    pub fn get_admins_by_expertise(env: &Env, expertise: String) -> Vec<Address> {
        let candidate_admins: Vec<Address> = env
            .storage()
            .persistent()
            .get(&AssignmentKey::ExpertiseAdmins(expertise))
            .unwrap_or_else(|| Vec::new(env));

        let mut out = Vec::new(env);
        for i in 0..candidate_admins.len() {
            let addr = candidate_admins.get(i).unwrap();
            if AdminManager::is_admin(env, &addr) {
                out.push_back(addr);
            }
        }
        out
    }

    // ── Assignment & Routing ──────────────────────────────────────────────────

    /// Admin: assign a pending verification request to an admin with specific expertise.
    pub fn assign_verification_with_expertise(
        env: &Env,
        project_id: u64,
        admin: Address,
        assignee: Address,
        expertise: Option<String>,
    ) -> Result<u64, ContractError> {
        require_admin_auth(env, &admin)?;

        // Assignee must also be an admin
        if !AdminManager::is_admin(env, &assignee) {
            return Err(ContractError::AdminNotFound);
        }

        // If expertise is specified, validate that assignee possesses it
        if let Some(ref exp) = expertise {
            let exp_len = exp.len() as usize;
            if exp_len == 0 || exp_len > MAX_EXPERTISE_LEN {
                return Err(ContractError::InvalidInput);
            }
            if !Self::admin_has_expertise(env, &assignee, exp) {
                return Err(ContractError::AdminLacksExpertise);
            }
        }

        let mut record = VerificationRegistry::get_verification(env, project_id)
            .ok_or(ContractError::VerificationNotFound)?;

        if record.status != VerificationStatus::Pending {
            return Err(ContractError::InvalidStatus);
        }

        let now = env.ledger().timestamp();
        let sla_duration = Self::get_verification_sla(env);
        let sla_deadline = now.saturating_add(sla_duration);
        let assignment_id = Self::get_next_assignment_id(env);

        let assignment = VerificationAssignment {
            assignment_id,
            project_id,
            request_id: record.request_id,
            assigner: admin.clone(),
            assignee: assignee.clone(),
            expertise: expertise.clone(),
            status: VerificationAssignmentStatus::Assigned,
            assigned_at: now,
            responded_at: None,
            sla_deadline,
            decline_reason: None,
            escalation_reason: None,
        };

        // Persist assignment record
        env.storage()
            .persistent()
            .set(&AssignmentKey::Assignment(assignment_id), &assignment);
        env.storage().persistent().set(
            &AssignmentKey::ActiveProjectAssignment(project_id),
            &assignment_id,
        );

        Self::append_to_project_history(env, project_id, assignment_id);
        Self::append_to_admin_assignments(env, assignee.clone(), assignment_id);

        // Update record
        record.assigned_admin = Some(assignee.clone());
        env.storage()
            .persistent()
            .set(&StorageKey::VerificationRecord(record.request_id), &record);

        // Publish events
        publish_verification_assigned_event(
            env,
            project_id,
            record.request_id,
            assignee.clone(),
            admin.clone(),
        );

        publish_verification_assigned_with_expertise_event(
            env,
            assignment_id,
            project_id,
            record.request_id,
            admin.clone(),
            assignee.clone(),
            expertise,
            sla_deadline,
        );

        AdminActionLog::record_action(
            env,
            admin,
            AdminActionType::VerificationAssigned,
            Some(project_id),
            Some(assignee),
            None,
        );

        Ok(assignment_id)
    }

    /// Admin: route verification request automatically to an available admin with the requested expertise.
    pub fn route_verification_to_expert(
        env: &Env,
        project_id: u64,
        admin: Address,
        expertise: String,
    ) -> Result<Address, ContractError> {
        require_admin_auth(env, &admin)?;

        let candidates = Self::get_admins_by_expertise(env, expertise.clone());
        if candidates.is_empty() {
            return Err(ContractError::AdminLacksExpertise);
        }

        let mut best_admin = candidates.get(0).unwrap();
        let mut min_workload = u32::MAX;

        for i in 0..candidates.len() {
            let candidate = candidates.get(i).unwrap();
            let workload = Self::get_admin_active_assignment_count(env, candidate.clone());
            if workload < min_workload {
                min_workload = workload;
                best_admin = candidate;
            }
        }

        Self::assign_verification_with_expertise(
            env,
            project_id,
            admin,
            best_admin.clone(),
            Some(expertise),
        )?;

        Ok(best_admin)
    }

    // ── Accept / Decline Workflows ────────────────────────────────────────────

    /// Assigned Admin: accept the verification assignment to begin review.
    pub fn accept_verification_assignment(
        env: &Env,
        project_id: u64,
        admin: Address,
    ) -> Result<(), ContractError> {
        require_admin_auth(env, &admin)?;

        if !AdminManager::is_admin(env, &admin) {
            return Err(ContractError::AdminNotFound);
        }

        let record = VerificationRegistry::get_verification(env, project_id)
            .ok_or(ContractError::VerificationNotFound)?;

        if record.status != VerificationStatus::Pending {
            return Err(ContractError::InvalidStatus);
        }

        if record.assigned_admin != Some(admin.clone()) {
            return Err(ContractError::NotAssignedAdmin);
        }

        let now = env.ledger().timestamp();

        if let Some(active_id) = env
            .storage()
            .persistent()
            .get::<_, u64>(&AssignmentKey::ActiveProjectAssignment(project_id))
        {
            let mut assignment: VerificationAssignment = env
                .storage()
                .persistent()
                .get(&AssignmentKey::Assignment(active_id))
                .ok_or(ContractError::AssignmentNotFound)?;

            if assignment.assignee != admin {
                return Err(ContractError::NotAssignedAdmin);
            }

            if assignment.status != VerificationAssignmentStatus::Assigned {
                return Err(ContractError::InvalidAssignmentStatus);
            }

            assignment.status = VerificationAssignmentStatus::Accepted;
            assignment.responded_at = Some(now);
            env.storage()
                .persistent()
                .set(&AssignmentKey::Assignment(active_id), &assignment);

            publish_verification_assignment_accepted_event(
                env,
                active_id,
                project_id,
                record.request_id,
                admin.clone(),
            );

            AdminActionLog::record_action(
                env,
                admin,
                AdminActionType::VerificationAssignmentAccepted,
                Some(project_id),
                None,
                None,
            );
        } else {
            // Synthesize an assignment entry for assignments made via legacy assign_verification
            let id = Self::get_next_assignment_id(env);
            let sla_duration = Self::get_verification_sla(env);
            let sla_deadline = now.saturating_add(sla_duration);

            let assignment = VerificationAssignment {
                assignment_id: id,
                project_id,
                request_id: record.request_id,
                assigner: admin.clone(),
                assignee: admin.clone(),
                expertise: None,
                status: VerificationAssignmentStatus::Accepted,
                assigned_at: now,
                responded_at: Some(now),
                sla_deadline,
                decline_reason: None,
                escalation_reason: None,
            };

            env.storage()
                .persistent()
                .set(&AssignmentKey::Assignment(id), &assignment);
            env.storage()
                .persistent()
                .set(&AssignmentKey::ActiveProjectAssignment(project_id), &id);

            Self::append_to_project_history(env, project_id, id);
            Self::append_to_admin_assignments(env, admin.clone(), id);

            publish_verification_assignment_accepted_event(
                env,
                id,
                project_id,
                record.request_id,
                admin.clone(),
            );

            AdminActionLog::record_action(
                env,
                admin,
                AdminActionType::VerificationAssignmentAccepted,
                Some(project_id),
                None,
                None,
            );
        }

        Ok(())
    }

    /// Assigned Admin: decline the verification assignment with a reason, releasing it for reassignment.
    pub fn decline_verification_assignment(
        env: &Env,
        project_id: u64,
        admin: Address,
        reason: String,
    ) -> Result<(), ContractError> {
        require_admin_auth(env, &admin)?;

        if !AdminManager::is_admin(env, &admin) {
            return Err(ContractError::AdminNotFound);
        }

        let reason_len = reason.len() as usize;
        if reason_len == 0 || reason_len > MAX_ASSIGNMENT_REASON_LEN {
            return Err(ContractError::InvalidInput);
        }

        let mut record = VerificationRegistry::get_verification(env, project_id)
            .ok_or(ContractError::VerificationNotFound)?;

        if record.status != VerificationStatus::Pending {
            return Err(ContractError::InvalidStatus);
        }

        if record.assigned_admin != Some(admin.clone()) {
            return Err(ContractError::NotAssignedAdmin);
        }

        let now = env.ledger().timestamp();

        let assignment_id = if let Some(active_id) = env
            .storage()
            .persistent()
            .get::<_, u64>(&AssignmentKey::ActiveProjectAssignment(project_id))
        {
            let mut assignment: VerificationAssignment = env
                .storage()
                .persistent()
                .get(&AssignmentKey::Assignment(active_id))
                .ok_or(ContractError::AssignmentNotFound)?;

            if assignment.assignee != admin {
                return Err(ContractError::NotAssignedAdmin);
            }

            if assignment.status != VerificationAssignmentStatus::Assigned
                && assignment.status != VerificationAssignmentStatus::Accepted
            {
                return Err(ContractError::InvalidAssignmentStatus);
            }

            assignment.status = VerificationAssignmentStatus::Declined;
            assignment.responded_at = Some(now);
            assignment.decline_reason = Some(reason.clone());
            env.storage()
                .persistent()
                .set(&AssignmentKey::Assignment(active_id), &assignment);

            active_id
        } else {
            let id = Self::get_next_assignment_id(env);
            let sla_duration = Self::get_verification_sla(env);
            let sla_deadline = now.saturating_add(sla_duration);

            let assignment = VerificationAssignment {
                assignment_id: id,
                project_id,
                request_id: record.request_id,
                assigner: admin.clone(),
                assignee: admin.clone(),
                expertise: None,
                status: VerificationAssignmentStatus::Declined,
                assigned_at: now,
                responded_at: Some(now),
                sla_deadline,
                decline_reason: Some(reason.clone()),
                escalation_reason: None,
            };

            env.storage()
                .persistent()
                .set(&AssignmentKey::Assignment(id), &assignment);

            Self::append_to_project_history(env, project_id, id);
            Self::append_to_admin_assignments(env, admin.clone(), id);

            id
        };

        // Remove active assignment pointer so project can be assigned again
        env.storage()
            .persistent()
            .remove(&AssignmentKey::ActiveProjectAssignment(project_id));

        // Unassign in record
        record.assigned_admin = None;
        env.storage()
            .persistent()
            .set(&StorageKey::VerificationRecord(record.request_id), &record);

        publish_verification_assignment_declined_event(
            env,
            assignment_id,
            project_id,
            record.request_id,
            admin.clone(),
            reason.clone(),
        );

        AdminActionLog::record_action(
            env,
            admin,
            AdminActionType::VerificationAssignmentDeclined,
            Some(project_id),
            None,
            Some(reason),
        );

        Ok(())
    }

    // ── SLA Escalation ────────────────────────────────────────────────────────

    /// Check if the active verification assignment for a project has breached SLA.
    pub fn is_assignment_sla_breached(env: &Env, project_id: u64) -> bool {
        let active_id = match env
            .storage()
            .persistent()
            .get::<_, u64>(&AssignmentKey::ActiveProjectAssignment(project_id))
        {
            Some(id) => id,
            None => return false,
        };

        let assignment: VerificationAssignment = match env
            .storage()
            .persistent()
            .get(&AssignmentKey::Assignment(active_id))
        {
            Some(a) => a,
            None => return false,
        };

        if assignment.status != VerificationAssignmentStatus::Assigned
            && assignment.status != VerificationAssignmentStatus::Accepted
        {
            return false;
        }

        let now = env.ledger().timestamp();
        now > assignment.sla_deadline
    }

    /// Get remaining seconds until active assignment SLA breaches (0 if breached, None if inactive).
    pub fn get_assignment_sla_remaining(env: &Env, project_id: u64) -> Option<u64> {
        let active_id = env
            .storage()
            .persistent()
            .get::<_, u64>(&AssignmentKey::ActiveProjectAssignment(project_id))?;

        let assignment: VerificationAssignment = env
            .storage()
            .persistent()
            .get(&AssignmentKey::Assignment(active_id))?;

        if assignment.status != VerificationAssignmentStatus::Assigned
            && assignment.status != VerificationAssignmentStatus::Accepted
        {
            return None;
        }

        let now = env.ledger().timestamp();
        if now >= assignment.sla_deadline {
            Some(0)
        } else {
            Some(assignment.sla_deadline.saturating_sub(now))
        }
    }

    /// Admin: escalate an uncompleted verification assignment that breached SLA.
    pub fn escalate_verification_assignment(
        env: &Env,
        caller: Address,
        project_id: u64,
        reason: String,
    ) -> Result<(), ContractError> {
        require_admin_auth(env, &caller)?;

        if !AdminManager::is_admin(env, &caller) {
            return Err(ContractError::AdminNotFound);
        }

        let reason_len = reason.len() as usize;
        if reason_len == 0 || reason_len > MAX_ASSIGNMENT_REASON_LEN {
            return Err(ContractError::InvalidInput);
        }

        let mut record = VerificationRegistry::get_verification(env, project_id)
            .ok_or(ContractError::VerificationNotFound)?;

        if record.status != VerificationStatus::Pending {
            return Err(ContractError::InvalidStatus);
        }

        let active_id = env
            .storage()
            .persistent()
            .get::<_, u64>(&AssignmentKey::ActiveProjectAssignment(project_id))
            .ok_or(ContractError::NoActiveAssignment)?;

        let mut assignment: VerificationAssignment = env
            .storage()
            .persistent()
            .get(&AssignmentKey::Assignment(active_id))
            .ok_or(ContractError::AssignmentNotFound)?;

        if assignment.status != VerificationAssignmentStatus::Assigned
            && assignment.status != VerificationAssignmentStatus::Accepted
        {
            return Err(ContractError::InvalidAssignmentStatus);
        }

        let now = env.ledger().timestamp();
        // Escalate only if review SLA has been breached
        if now <= assignment.sla_deadline {
            return Err(ContractError::SlaNotBreached);
        }

        assignment.status = VerificationAssignmentStatus::Escalated;
        assignment.escalation_reason = Some(reason.clone());
        env.storage()
            .persistent()
            .set(&AssignmentKey::Assignment(active_id), &assignment);

        // Remove active assignment pointer
        env.storage()
            .persistent()
            .remove(&AssignmentKey::ActiveProjectAssignment(project_id));

        // Unassign in record so it can be re-routed / reassigned
        record.assigned_admin = None;
        env.storage()
            .persistent()
            .set(&StorageKey::VerificationRecord(record.request_id), &record);

        publish_verification_assignment_escalated_event(
            env,
            active_id,
            project_id,
            record.request_id,
            assignment.assignee.clone(),
            caller.clone(),
            reason.clone(),
        );

        AdminActionLog::record_action(
            env,
            caller,
            AdminActionType::VerificationAssignmentEscalated,
            Some(project_id),
            Some(assignment.assignee),
            Some(reason),
        );

        Ok(())
    }

    // ── History & Queries ─────────────────────────────────────────────────────

    /// Get active verification assignment for a project.
    pub fn get_current_verification_assignment(
        env: &Env,
        project_id: u64,
    ) -> Option<VerificationAssignment> {
        if let Some(active_id) = env
            .storage()
            .persistent()
            .get::<_, u64>(&AssignmentKey::ActiveProjectAssignment(project_id))
        {
            let mut assignment: VerificationAssignment = env
                .storage()
                .persistent()
                .get(&AssignmentKey::Assignment(active_id))?;

            if let Some(record) = VerificationRegistry::get_verification(env, project_id) {
                if record.status != VerificationStatus::Pending
                    && (assignment.status == VerificationAssignmentStatus::Assigned
                        || assignment.status == VerificationAssignmentStatus::Accepted)
                {
                    assignment.status = VerificationAssignmentStatus::Completed;
                }
            }
            return Some(assignment);
        }

        // Fallback for requests assigned via legacy assign_verification
        let record = VerificationRegistry::get_verification(env, project_id)?;
        let assigned_admin = record.assigned_admin?;
        let sla_duration = Self::get_verification_sla(env);
        let sla_deadline = record.requested_at.saturating_add(sla_duration);
        let status = if record.status != VerificationStatus::Pending {
            VerificationAssignmentStatus::Completed
        } else {
            VerificationAssignmentStatus::Assigned
        };

        Some(VerificationAssignment {
            assignment_id: 0,
            project_id,
            request_id: record.request_id,
            assigner: assigned_admin.clone(),
            assignee: assigned_admin,
            expertise: None,
            status,
            assigned_at: record.requested_at,
            responded_at: None,
            sla_deadline,
            decline_reason: None,
            escalation_reason: None,
        })
    }

    /// Get a specific verification assignment record by ID.
    pub fn get_verification_assignment(
        env: &Env,
        assignment_id: u64,
    ) -> Option<VerificationAssignment> {
        env.storage()
            .persistent()
            .get(&AssignmentKey::Assignment(assignment_id))
    }

    /// Retrieve complete verification assignment history for a project.
    pub fn get_verification_assignment_history(
        env: &Env,
        project_id: u64,
    ) -> Vec<VerificationAssignment> {
        let history_ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&AssignmentKey::ProjectAssignmentHistory(project_id))
            .unwrap_or_else(|| Vec::new(env));

        let mut out = Vec::new(env);
        for i in 0..history_ids.len() {
            if let Some(id) = history_ids.get(i) {
                if let Some(assignment) = env
                    .storage()
                    .persistent()
                    .get::<_, VerificationAssignment>(&AssignmentKey::Assignment(id))
                {
                    out.push_back(assignment);
                }
            }
        }
        out
    }

    /// Retrieve paginated verification assignment history for a project.
    pub fn get_verification_assignment_history_paginated(
        env: &Env,
        project_id: u64,
        start_index: u32,
        limit: u32,
    ) -> Vec<VerificationAssignment> {
        let history_ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&AssignmentKey::ProjectAssignmentHistory(project_id))
            .unwrap_or_else(|| Vec::new(env));

        let page_ids = crate::pagination::paginate(env, &history_ids, start_index, limit);
        let mut out = Vec::new(env);
        for i in 0..page_ids.len() {
            if let Some(id) = page_ids.get(i) {
                if let Some(assignment) = env
                    .storage()
                    .persistent()
                    .get::<_, VerificationAssignment>(&AssignmentKey::Assignment(id))
                {
                    out.push_back(assignment);
                }
            }
        }
        out
    }

    /// Get all assignments given to a specific admin.
    pub fn get_admin_assignments(env: &Env, admin: Address) -> Vec<VerificationAssignment> {
        let ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&AssignmentKey::AdminAssignments(admin))
            .unwrap_or_else(|| Vec::new(env));

        let mut out = Vec::new(env);
        for i in 0..ids.len() {
            if let Some(id) = ids.get(i) {
                if let Some(assignment) = env
                    .storage()
                    .persistent()
                    .get::<_, VerificationAssignment>(&AssignmentKey::Assignment(id))
                {
                    out.push_back(assignment);
                }
            }
        }
        out
    }

    /// Get summary workload statistics for an admin.
    pub fn get_admin_workload(env: &Env, admin: Address) -> AdminWorkload {
        let assignments = Self::get_admin_assignments(env, admin.clone());
        let total_assigned = assignments.len();
        let mut active_assignments: u32 = 0;
        let mut total_completed: u32 = 0;
        let mut total_declined: u32 = 0;
        let mut total_escalated: u32 = 0;

        for i in 0..assignments.len() {
            if let Some(a) = assignments.get(i) {
                match a.status {
                    VerificationAssignmentStatus::Assigned
                    | VerificationAssignmentStatus::Accepted => {
                        active_assignments = active_assignments.saturating_add(1);
                    }
                    VerificationAssignmentStatus::Completed => {
                        total_completed = total_completed.saturating_add(1);
                    }
                    VerificationAssignmentStatus::Declined => {
                        total_declined = total_declined.saturating_add(1);
                    }
                    VerificationAssignmentStatus::Escalated => {
                        total_escalated = total_escalated.saturating_add(1);
                    }
                }
            }
        }

        AdminWorkload {
            admin,
            active_assignments,
            total_assigned,
            total_completed,
            total_declined,
            total_escalated,
        }
    }

    // ── Internal Helpers ──────────────────────────────────────────────────────

    fn get_admin_active_assignment_count(env: &Env, admin: Address) -> u32 {
        let workload = Self::get_admin_workload(env, admin);
        workload.active_assignments
    }

    fn get_next_assignment_id(env: &Env) -> u64 {
        let id: u64 = env
            .storage()
            .persistent()
            .get(&AssignmentKey::NextAssignmentId)
            .unwrap_or(1);
        env.storage()
            .persistent()
            .set(&AssignmentKey::NextAssignmentId, &(id + 1));
        id
    }

    fn append_to_project_history(env: &Env, project_id: u64, assignment_id: u64) {
        let mut history: Vec<u64> = env
            .storage()
            .persistent()
            .get(&AssignmentKey::ProjectAssignmentHistory(project_id))
            .unwrap_or_else(|| Vec::new(env));
        history.push_back(assignment_id);
        env.storage().persistent().set(
            &AssignmentKey::ProjectAssignmentHistory(project_id),
            &history,
        );
    }

    fn append_to_admin_assignments(env: &Env, admin: Address, assignment_id: u64) {
        let mut list: Vec<u64> = env
            .storage()
            .persistent()
            .get(&AssignmentKey::AdminAssignments(admin.clone()))
            .unwrap_or_else(|| Vec::new(env));
        list.push_back(assignment_id);
        env.storage()
            .persistent()
            .set(&AssignmentKey::AdminAssignments(admin), &list);
    }

    fn remove_admin_from_expertise_index(env: &Env, tag: &String, admin: &Address) {
        let mut admins: Vec<Address> = env
            .storage()
            .persistent()
            .get(&AssignmentKey::ExpertiseAdmins(tag.clone()))
            .unwrap_or_else(|| Vec::new(env));
        let mut found_idx: Option<u32> = None;
        for i in 0..admins.len() {
            if admins.get(i).unwrap() == *admin {
                found_idx = Some(i);
                break;
            }
        }
        if let Some(idx) = found_idx {
            admins.remove(idx);
            env.storage()
                .persistent()
                .set(&AssignmentKey::ExpertiseAdmins(tag.clone()), &admins);
        }
    }

    fn add_admin_to_expertise_index(env: &Env, tag: &String, admin: &Address) {
        let mut admins: Vec<Address> = env
            .storage()
            .persistent()
            .get(&AssignmentKey::ExpertiseAdmins(tag.clone()))
            .unwrap_or_else(|| Vec::new(env));
        let mut found = false;
        for i in 0..admins.len() {
            if admins.get(i).unwrap() == *admin {
                found = true;
                break;
            }
        }
        if !found {
            admins.push_back(admin.clone());
            env.storage()
                .persistent()
                .set(&AssignmentKey::ExpertiseAdmins(tag.clone()), &admins);
        }
    }
}
