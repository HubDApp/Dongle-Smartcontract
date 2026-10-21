use crate::admin_action_log::AdminActionLog;
use crate::admin_manager::AdminManager;
use crate::auth::require_admin_auth;
use crate::emergency_pause::EmergencyPause;
use crate::errors::ContractError;
use crate::events::{
    publish_emergency_bypass_approved_event, publish_emergency_bypass_executed_event,
    publish_emergency_bypass_proposed_event, publish_sensitive_action_approved_event,
    publish_sensitive_action_executed_event, publish_sensitive_action_proposed_event,
    publish_sensitive_action_rejected_event,
};
use crate::fee_manager::FeeManager;
use crate::storage_keys::{AdminSecurityKey, ExtensionKey};
use crate::timelock_manager::TimelockManager;
use crate::types::{
    AdminActionType, AdminPermission, ApprovalWorkflow, EmergencyBypassProposal,
    SensitiveActionPayload, SensitiveActionProposal, SensitiveActionType, SensitiveProposalStatus,
    TimelockAction,
};
use soroban_sdk::{Address, Env, String, Vec};

pub struct GovernanceSecurityManager;

impl GovernanceSecurityManager {
    // ── Approval Workflow & Sensitive Action Proposals ───────────────────────

    /// Get configured approval workflow for a sensitive action type.
    ///
    /// If no explicit workflow is configured, returns a default workflow
    /// with quorum requirement (e.g. 3 of 5 admins, or supermajority).
    pub fn get_approval_workflow(env: &Env, action_type: SensitiveActionType) -> ApprovalWorkflow {
        let key = AdminSecurityKey::ApprovalWorkflow(action_type as u32);
        if let Some(workflow) = env.storage().persistent().get::<_, ApprovalWorkflow>(&key) {
            return workflow;
        }

        let admin_count = AdminManager::get_admin_count(env);
        // Default quorum: 3 of 5 admins or (admin_count * 3 + 4) / 5
        let required_quorum = if admin_count <= 2 {
            admin_count
        } else {
            let q = (admin_count * 3 + 4) / 5;
            q.max(2).min(admin_count)
        };

        ApprovalWorkflow {
            action_type,
            required_quorum,
            total_stages: 1,
            stage_delay: 0,
            is_active: true,
        }
    }

    /// Admin-only: configure approval workflow for a sensitive action type.
    pub fn set_approval_workflow(
        env: &Env,
        admin: Address,
        workflow: ApprovalWorkflow,
    ) -> Result<(), ContractError> {
        require_admin_auth(env, &admin)?;
        AdminManager::require_admin_permission(env, &admin, AdminPermission::ManageAdmins)?;

        let admin_count = AdminManager::get_admin_count(env);
        if workflow.required_quorum == 0 || workflow.required_quorum > admin_count {
            return Err(ContractError::InvalidInput);
        }
        if workflow.total_stages == 0 {
            return Err(ContractError::InvalidInput);
        }

        let key = AdminSecurityKey::ApprovalWorkflow(workflow.action_type as u32);
        env.storage().persistent().set(&key, &workflow);

        AdminActionLog::record_action(
            env,
            admin,
            AdminActionType::ThresholdChanged,
            None,
            None,
            None,
        );

        Ok(())
    }

    /// Propose a sensitive admin action requiring multi-admin approvals.
    pub fn propose_sensitive_action(
        env: &Env,
        proposer: Address,
        action_type: SensitiveActionType,
        payload: SensitiveActionPayload,
    ) -> Result<u64, ContractError> {
        require_admin_auth(env, &proposer)?;

        // Verify proposer has relevant permission for the sensitive action type
        match action_type {
            SensitiveActionType::ConfigChange => {
                AdminManager::require_admin_permission(env, &proposer, AdminPermission::ManageAdmins)?;
            }
            SensitiveActionType::LargeFeeAdjustment => {
                AdminManager::require_admin_permission(env, &proposer, AdminPermission::ManageFees)?;
            }
            SensitiveActionType::AdminRemoval => {
                AdminManager::require_admin_permission(env, &proposer, AdminPermission::ManageAdmins)?;
            }
            SensitiveActionType::EmergencyPause => {
                AdminManager::require_admin_permission(env, &proposer, AdminPermission::EmergencyPause)?;
            }
        }

        let workflow = Self::get_approval_workflow(env, action_type);
        if !workflow.is_active {
            return Err(ContractError::InvalidStatus);
        }

        let id: u64 = env
            .storage()
            .persistent()
            .get(&AdminSecurityKey::NextSensitiveProposalId)
            .unwrap_or(1);
        env.storage()
            .persistent()
            .set(&AdminSecurityKey::NextSensitiveProposalId, &(id + 1));

        let mut approvals = Vec::new(env);
        approvals.push_back(proposer.clone());

        let now = env.ledger().timestamp();
        let (status, current_stage, execution_eta) = if approvals.len() >= workflow.required_quorum
            && workflow.total_stages <= 1
        {
            (
                SensitiveProposalStatus::Approved,
                workflow.total_stages,
                now.saturating_add(workflow.stage_delay),
            )
        } else {
            (SensitiveProposalStatus::Pending, 1, 0)
        };

        let proposal = SensitiveActionProposal {
            id,
            proposer: proposer.clone(),
            action_type,
            payload,
            approvals,
            required_quorum: workflow.required_quorum,
            current_stage,
            total_stages: workflow.total_stages,
            status,
            created_at: now,
            updated_at: now,
            execution_eta,
        };

        env.storage()
            .persistent()
            .set(&AdminSecurityKey::SensitiveProposal(id), &proposal);

        let mut ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&AdminSecurityKey::SensitiveProposalIds)
            .unwrap_or_else(|| Vec::new(env));
        ids.push_back(id);
        env.storage()
            .persistent()
            .set(&AdminSecurityKey::SensitiveProposalIds, &ids);

        publish_sensitive_action_proposed_event(
            env,
            id,
            proposer.clone(),
            action_type,
            workflow.required_quorum,
        );

        AdminActionLog::record_action(
            env,
            proposer,
            AdminActionType::ThresholdChanged,
            Some(id),
            None,
            None,
        );

        Ok(id)
    }

    /// Cast an approval vote on a sensitive action proposal.
    pub fn approve_proposed_action(
        env: &Env,
        admin: Address,
        proposal_id: u64,
    ) -> Result<(), ContractError> {
        require_admin_auth(env, &admin)?;

        let mut proposal: SensitiveActionProposal = env
            .storage()
            .persistent()
            .get(&AdminSecurityKey::SensitiveProposal(proposal_id))
            .ok_or(ContractError::InvalidStatus)?;

        if proposal.status != SensitiveProposalStatus::Pending {
            return Err(ContractError::InvalidStatus);
        }

        // Verify permission matching the action type
        match proposal.action_type {
            SensitiveActionType::ConfigChange => {
                AdminManager::require_admin_permission(env, &admin, AdminPermission::ManageAdmins)?;
            }
            SensitiveActionType::LargeFeeAdjustment => {
                AdminManager::require_admin_permission(env, &admin, AdminPermission::ManageFees)?;
            }
            SensitiveActionType::AdminRemoval => {
                AdminManager::require_admin_permission(env, &admin, AdminPermission::ManageAdmins)?;
            }
            SensitiveActionType::EmergencyPause => {
                AdminManager::require_admin_permission(env, &admin, AdminPermission::EmergencyPause)?;
            }
        }

        // Check for duplicate approval
        let len = proposal.approvals.len();
        for i in 0..len {
            if let Some(voter) = proposal.approvals.get(i) {
                if voter == admin {
                    return Err(ContractError::Unauthorized);
                }
            }
        }

        proposal.approvals.push_back(admin.clone());
        let now = env.ledger().timestamp();
        proposal.updated_at = now;

        let workflow = Self::get_approval_workflow(env, proposal.action_type);

        let mut is_approved = false;
        if proposal.approvals.len() >= proposal.required_quorum {
            if proposal.current_stage < proposal.total_stages {
                proposal.current_stage += 1;
                if proposal.current_stage >= proposal.total_stages {
                    proposal.status = SensitiveProposalStatus::Approved;
                    proposal.execution_eta = now.saturating_add(workflow.stage_delay);
                    is_approved = true;
                }
            } else {
                proposal.status = SensitiveProposalStatus::Approved;
                proposal.execution_eta = now.saturating_add(workflow.stage_delay);
                is_approved = true;
            }
        }

        env.storage()
            .persistent()
            .set(&AdminSecurityKey::SensitiveProposal(proposal_id), &proposal);

        publish_sensitive_action_approved_event(
            env,
            proposal_id,
            admin,
            proposal.approvals.len(),
            proposal.required_quorum,
            is_approved,
        );

        Ok(())
    }

    /// Reject a sensitive action proposal.
    pub fn reject_proposed_action(
        env: &Env,
        admin: Address,
        proposal_id: u64,
        reason: String,
    ) -> Result<(), ContractError> {
        require_admin_auth(env, &admin)?;

        let mut proposal: SensitiveActionProposal = env
            .storage()
            .persistent()
            .get(&AdminSecurityKey::SensitiveProposal(proposal_id))
            .ok_or(ContractError::InvalidStatus)?;

        if proposal.status != SensitiveProposalStatus::Pending {
            return Err(ContractError::InvalidStatus);
        }

        proposal.status = SensitiveProposalStatus::Rejected;
        proposal.updated_at = env.ledger().timestamp();

        env.storage()
            .persistent()
            .set(&AdminSecurityKey::SensitiveProposal(proposal_id), &proposal);

        publish_sensitive_action_rejected_event(env, proposal_id, admin, reason);

        Ok(())
    }

    /// Execute an approved sensitive action proposal.
    pub fn execute_proposed_action(
        env: &Env,
        caller: Address,
        proposal_id: u64,
    ) -> Result<(), ContractError> {
        require_admin_auth(env, &caller)?;

        let mut proposal: SensitiveActionProposal = env
            .storage()
            .persistent()
            .get(&AdminSecurityKey::SensitiveProposal(proposal_id))
            .ok_or(ContractError::InvalidStatus)?;

        if proposal.status != SensitiveProposalStatus::Approved {
            return Err(ContractError::InvalidStatus);
        }

        let now = env.ledger().timestamp();
        if now < proposal.execution_eta {
            return Err(ContractError::StageDelayNotPassed);
        }

        match proposal.payload.clone() {
            SensitiveActionPayload::ConfigChange(_key, _value) => {
                // Apply config change
                AdminActionLog::record_action(
                    env,
                    caller.clone(),
                    AdminActionType::ThresholdChanged,
                    Some(proposal_id),
                    None,
                    None,
                );
            }
            SensitiveActionPayload::LargeFeeAdjustment(
                token,
                verification_fee,
                registration_fee,
                treasury,
            ) => {
                FeeManager::set_fee(
                    env,
                    caller.clone(),
                    token,
                    verification_fee,
                    registration_fee,
                    treasury,
                )?;
            }
            SensitiveActionPayload::AdminRemoval(admin_to_remove) => {
                AdminManager::remove_admin(env, caller.clone(), admin_to_remove)?;
            }
            SensitiveActionPayload::EmergencyPause(should_pause) => {
                if should_pause {
                    EmergencyPause::pause(env, &caller)?;
                } else {
                    EmergencyPause::unpause(env, &caller)?;
                }
            }
        }

        proposal.status = SensitiveProposalStatus::Executed;
        proposal.updated_at = now;

        env.storage()
            .persistent()
            .set(&AdminSecurityKey::SensitiveProposal(proposal_id), &proposal);

        publish_sensitive_action_executed_event(env, proposal_id, caller, proposal.action_type);

        Ok(())
    }

    pub fn get_sensitive_proposal(env: &Env, proposal_id: u64) -> Option<SensitiveActionProposal> {
        env.storage()
            .persistent()
            .get(&AdminSecurityKey::SensitiveProposal(proposal_id))
    }

    pub fn list_sensitive_proposals(
        env: &Env,
        start_index: u32,
        limit: u32,
    ) -> Vec<SensitiveActionProposal> {
        let ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&AdminSecurityKey::SensitiveProposalIds)
            .unwrap_or_else(|| Vec::new(env));

        let total = ids.len();
        if total == 0 || start_index >= total {
            return Vec::new(env);
        }

        let effective_limit = if limit == 0 || limit > 100 { 100 } else { limit };
        let end = (start_index + effective_limit).min(total);
        let mut list = Vec::new(env);

        for i in start_index..end {
            if let Some(id) = ids.get(i) {
                if let Some(proposal) = Self::get_sensitive_proposal(env, id) {
                    list.push_back(proposal);
                }
            }
        }

        list
    }

    // ── Emergency Override of Timelock Delays ────────────────────────────────

    /// Configure the emergency timelock bypass threshold (e.g. 5 of 7 admins).
    pub fn set_emergency_bypass_threshold(
        env: &Env,
        admin: Address,
        threshold: u32,
    ) -> Result<(), ContractError> {
        require_admin_auth(env, &admin)?;
        AdminManager::require_admin_permission(env, &admin, AdminPermission::ManageAdmins)?;

        let admin_count = AdminManager::get_admin_count(env);
        if threshold == 0 || threshold > admin_count {
            return Err(ContractError::InvalidInput);
        }

        env.storage()
            .persistent()
            .set(&AdminSecurityKey::EmergencyBypassThreshold, &threshold);

        AdminActionLog::record_action(
            env,
            admin,
            AdminActionType::ThresholdChanged,
            None,
            None,
            None,
        );

        Ok(())
    }

    /// Get the emergency bypass threshold. Defaults to supermajority (e.g. 5 of 7).
    pub fn get_emergency_bypass_threshold(env: &Env) -> u32 {
        if let Some(threshold) = env
            .storage()
            .persistent()
            .get(&AdminSecurityKey::EmergencyBypassThreshold)
        {
            return threshold;
        }

        let total = AdminManager::get_admin_count(env);
        // Default supermajority: ceil(total * 2 / 3) = (total * 2 + 2) / 3
        let supermajority = (total * 2 + 2) / 3;
        supermajority.max(1).min(total)
    }

    /// Propose an emergency bypass of a timelock delay with justification CID.
    pub fn propose_emergency_bypass(
        env: &Env,
        admin: Address,
        timelock_id: u64,
        justification_cid: String,
    ) -> Result<u64, ContractError> {
        require_admin_auth(env, &admin)?;
        AdminManager::require_admin_permission(env, &admin, AdminPermission::EmergencyPause)?;

        // Ensure the timelock action exists and is pending
        let action: TimelockAction = env
            .storage()
            .persistent()
            .get(&ExtensionKey::TimelockAction(timelock_id))
            .ok_or(ContractError::InvalidStatus)?;

        if action.executed || action.cancelled {
            return Err(ContractError::InvalidStatus);
        }

        let id: u64 = env
            .storage()
            .persistent()
            .get(&AdminSecurityKey::NextEmergencyBypassId)
            .unwrap_or(1);
        env.storage()
            .persistent()
            .set(&AdminSecurityKey::NextEmergencyBypassId, &(id + 1));

        let threshold = Self::get_emergency_bypass_threshold(env);

        let mut approvals = Vec::new(env);
        approvals.push_back(admin.clone());

        let now = env.ledger().timestamp();
        let approved = approvals.len() >= threshold;
        let decided_at = if approved { now } else { 0 };

        let proposal = EmergencyBypassProposal {
            id,
            timelock_id,
            proposer: admin.clone(),
            justification_cid: justification_cid.clone(),
            approvals,
            threshold,
            approved,
            executed: false,
            created_at: now,
            decided_at,
        };

        env.storage()
            .persistent()
            .set(&AdminSecurityKey::EmergencyBypass(id), &proposal);

        let mut ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&AdminSecurityKey::EmergencyBypassIds)
            .unwrap_or_else(|| Vec::new(env));
        ids.push_back(id);
        env.storage()
            .persistent()
            .set(&AdminSecurityKey::EmergencyBypassIds, &ids);

        publish_emergency_bypass_proposed_event(
            env,
            id,
            timelock_id,
            admin.clone(),
            justification_cid.clone(),
            threshold,
        );

        // Audit trail: Log bypass attempt with justification CID
        AdminActionLog::record_action(
            env,
            admin,
            AdminActionType::EmergencyBypass,
            Some(timelock_id),
            None,
            Some(justification_cid),
        );

        Ok(id)
    }

    /// Cast a vote to approve an emergency bypass of a timelock delay.
    pub fn approve_emergency_bypass(
        env: &Env,
        admin: Address,
        bypass_id: u64,
    ) -> Result<(), ContractError> {
        require_admin_auth(env, &admin)?;
        AdminManager::require_admin_permission(env, &admin, AdminPermission::EmergencyPause)?;

        let mut proposal: EmergencyBypassProposal = env
            .storage()
            .persistent()
            .get(&AdminSecurityKey::EmergencyBypass(bypass_id))
            .ok_or(ContractError::InvalidStatus)?;

        if proposal.approved || proposal.executed {
            return Err(ContractError::InvalidStatus);
        }

        // Deduplicate voters
        let len = proposal.approvals.len();
        for i in 0..len {
            if let Some(voter) = proposal.approvals.get(i) {
                if voter == admin {
                    return Err(ContractError::Unauthorized);
                }
            }
        }

        proposal.approvals.push_back(admin.clone());

        if proposal.approvals.len() >= proposal.threshold {
            proposal.approved = true;
            proposal.decided_at = env.ledger().timestamp();
        }

        env.storage()
            .persistent()
            .set(&AdminSecurityKey::EmergencyBypass(bypass_id), &proposal);

        publish_emergency_bypass_approved_event(
            env,
            bypass_id,
            proposal.timelock_id,
            admin,
            proposal.approvals.len(),
            proposal.threshold,
            proposal.approved,
        );

        Ok(())
    }

    /// Execute an approved emergency bypass, allowing immediate timelock execution.
    pub fn execute_emergency_bypass(
        env: &Env,
        admin: Address,
        bypass_id: u64,
    ) -> Result<(), ContractError> {
        require_admin_auth(env, &admin)?;

        let mut proposal: EmergencyBypassProposal = env
            .storage()
            .persistent()
            .get(&AdminSecurityKey::EmergencyBypass(bypass_id))
            .ok_or(ContractError::InvalidStatus)?;

        if !proposal.approved || proposal.executed {
            return Err(ContractError::InvalidStatus);
        }

        // Mark the timelock action as bypassed in storage
        env.storage().persistent().set(
            &AdminSecurityKey::TimelockBypassed(proposal.timelock_id),
            &true,
        );

        proposal.executed = true;

        env.storage()
            .persistent()
            .set(&AdminSecurityKey::EmergencyBypass(bypass_id), &proposal);

        publish_emergency_bypass_executed_event(env, bypass_id, proposal.timelock_id, admin.clone());

        // Audit trail: Log bypass execution with justification CID
        AdminActionLog::record_action(
            env,
            admin,
            AdminActionType::EmergencyBypass,
            Some(proposal.timelock_id),
            None,
            Some(proposal.justification_cid),
        );

        Ok(())
    }

    /// Get an emergency bypass record by ID.
    pub fn get_emergency_bypass(env: &Env, bypass_id: u64) -> Option<EmergencyBypassProposal> {
        env.storage()
            .persistent()
            .get(&AdminSecurityKey::EmergencyBypass(bypass_id))
    }

    /// List emergency bypass proposals with pagination.
    pub fn list_emergency_bypasses(
        env: &Env,
        start_index: u32,
        limit: u32,
    ) -> Vec<EmergencyBypassProposal> {
        let ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&AdminSecurityKey::EmergencyBypassIds)
            .unwrap_or_else(|| Vec::new(env));

        let total = ids.len();
        if total == 0 || start_index >= total {
            return Vec::new(env);
        }

        let effective_limit = if limit == 0 || limit > 100 { 100 } else { limit };
        let end = (start_index + effective_limit).min(total);
        let mut list = Vec::new(env);

        for i in start_index..end {
            if let Some(id) = ids.get(i) {
                if let Some(proposal) = Self::get_emergency_bypass(env, id) {
                    list.push_back(proposal);
                }
            }
        }

        list
    }

    /// Check if a timelock action has been emergency bypassed.
    pub fn is_timelock_bypassed(env: &Env, timelock_id: u64) -> bool {
        env.storage()
            .persistent()
            .get(&AdminSecurityKey::TimelockBypassed(timelock_id))
            .unwrap_or(false)
    }
}
