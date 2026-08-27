//! Admin role management and access control
//!
//! This module provides functionality for managing admin roles and enforcing
//! access control across privileged contract operations.

use crate::admin_action_log::AdminActionLog;
use crate::auth::require_admin_auth;
use crate::constants::DEFAULT_VERIFICATION_DURATION_SECS;
use crate::errors::ContractError;
use crate::events::{publish_admin_added_event, publish_admin_removed_event};
use crate::storage_keys::StorageKey;
use crate::storage_manager::StorageManager;
use crate::types::{
    AdminActionType, AdminProposal, FeeConfig, ProposalPayload, ProposalStatus, VerificationStatus,
};
use crate::utils::Utils;
use soroban_sdk::{xdr::ToXdr, Address, Env, Map, Vec};

pub struct AdminManager;
impl AdminManager {
    /// Initialize the contract with the first admin
    pub fn initialize(env: &Env, admin: Address) -> Result<(), ContractError> {
        // Check if already initialized
        if env.storage().persistent().has(&StorageKey::AdminList) {
            return Err(ContractError::AlreadyInitialized);
        }

        // Don't require auth during initialization - this is typically called once during contract deployment

        // Set the admin in storage
        env.storage()
            .persistent()
            .set(&StorageKey::Admin(admin.clone()), &true);

        // Initialize admin list
        let mut admins = Vec::new(env);
        admins.push_back(admin.clone());
        env.storage()
            .persistent()
            .set(&StorageKey::AdminList, &admins);

        // Extend TTL for admin data
        StorageManager::extend_all_admin_ttl(env, &admin);

        publish_admin_added_event(env, admin);

        Ok(())
    }

    /// Add a new admin (only callable by existing admins)
    /// 
    /// # Errors
    /// Returns `MultiSigRequired` when admin approval threshold > 1.
    /// Use the proposal system (`create_proposal`) instead for multi-signature environments.
    pub fn add_admin(env: &Env, caller: Address, new_admin: Address) -> Result<(), ContractError> {
        require_admin_auth(env, &caller)?;

        if Self::get_admin_approval_threshold(env) > 1 {
            return Err(ContractError::MultiSigRequired);
        }

        // Check if already an admin
        if Self::is_admin(env, &new_admin) {
            return Ok(()); // Already an admin, no-op
        }

        // Add to admin mapping
        env.storage()
            .persistent()
            .set(&StorageKey::Admin(new_admin.clone()), &true);

        // Add to admin list
        let mut admins = Self::get_admin_list(env);
        admins.push_back(new_admin.clone());
        env.storage()
            .persistent()
            .set(&StorageKey::AdminList, &admins);

        // Extend TTL for admin data
        StorageManager::extend_all_admin_ttl(env, &new_admin);

        publish_admin_added_event(env, new_admin.clone());

        AdminActionLog::record_action(
            env,
            caller.clone(),
            AdminActionType::AdminAdded,
            None,
            Some(new_admin.clone()),
            None,
        );

        Ok(())
    }

    /// Remove an admin (only callable by existing admins)
    /// 
    /// # Errors
    /// Returns `MultiSigRequired` when admin approval threshold > 1.
    /// Use the proposal system (`create_proposal`) instead for multi-signature environments.
    pub fn remove_admin(
        env: &Env,
        caller: Address,
        admin_to_remove: Address,
    ) -> Result<(), ContractError> {
        require_admin_auth(env, &caller)?;

        if Self::get_admin_approval_threshold(env) > 1 {
            return Err(ContractError::MultiSigRequired);
        }

        // Check if the address is actually an admin first
        if !Self::is_admin(env, &admin_to_remove) {
            return Err(ContractError::AdminNotFound);
        }

        // Prevent removing the last admin
        let admins = Self::get_admin_list(env);
        if admins.len() <= 1 {
            return Err(ContractError::CannotRemoveLastAdmin);
        }

        // Remove from admin mapping
        env.storage()
            .persistent()
            .remove(&StorageKey::Admin(admin_to_remove.clone()));

        // Remove from admin list
        let new_admins = Utils::remove_item_from_vec(env, &admins, &admin_to_remove);
        env.storage()
            .persistent()
            .set(&StorageKey::AdminList, &new_admins);

        publish_admin_removed_event(env, admin_to_remove.clone());

        AdminActionLog::record_action(
            env,
            caller.clone(),
            AdminActionType::AdminRemoved,
            None,
            Some(admin_to_remove.clone()),
            None,
        );

        Ok(())
    }

    /// Check if an address is an admin
    pub fn is_admin(env: &Env, address: &Address) -> bool {
        let is_admin = env
            .storage()
            .persistent()
            .get(&StorageKey::Admin(address.clone()))
            .unwrap_or(false);

        // Bump TTL on read if admin exists
        if is_admin {
            StorageManager::extend_admin_ttl(env, address);
        }

        is_admin
    }

    /// Require that the caller is an admin, otherwise return an error
    pub fn require_admin(env: &Env, address: &Address) -> Result<(), ContractError> {
        if Self::is_admin(env, address) {
            Ok(())
        } else {
            Err(ContractError::AdminOnly)
        }
    }

    /// Get the list of all admins
    pub fn get_admin_list(env: &Env) -> Vec<Address> {
        env.storage()
            .persistent()
            .get(&StorageKey::AdminList)
            .unwrap_or(Vec::new(env))
    }

    /// Get the count of admins
    pub fn get_admin_count(env: &Env) -> u32 {
        Self::get_admin_list(env).len()
    }

    /// Set the verification duration (admin only).
    ///
    /// `duration_secs` is the number of seconds a Verified status will remain
    /// active after approval. Pass `0` to revert to the contract default.
    pub fn set_verification_duration(
        env: &Env,
        caller: Address,
        duration_secs: u64,
    ) -> Result<(), ContractError> {
        require_admin_auth(env, &caller)?;

        env.storage()
            .persistent()
            .set(&StorageKey::VerificationDuration, &duration_secs);

        // Keep this config entry alive as long as critical data.
        StorageManager::extend_critical_config_ttl(env);

        Ok(())
    }

    pub fn get_admin_approval_threshold(env: &Env) -> u32 {
        env.storage()
            .persistent()
            .get(&crate::storage_keys::ExtensionKey::AdminApprovalThreshold)
            .unwrap_or(1)
    }

    /// Directly set the admin approval threshold (single-admin fast-path).
    ///
    /// # Purpose
    /// This function is the *bootstrap* path for enabling multi-sig governance.
    /// It lets a single admin raise the threshold from the default of 1 to any
    /// value up to the current admin count.
    ///
    /// # Why it is intentionally locked once multi-sig is active
    /// Once the threshold is above 1 (multi-sig mode), this direct setter
    /// returns `Unauthorized` for every caller. This is deliberate: changing the
    /// governance quorum must itself pass through the same quorum.  Any future
    /// threshold change — including *lowering* it — must be submitted as a
    /// `ProposalPayload::SetThreshold` proposal and approved by the required
    /// number of admins before it can execute.
    ///
    /// # Threshold-downgrade protection in the proposal path
    /// A `SetThreshold` proposal that would lower the current threshold is
    /// subject to a supermajority rule enforced inside `execute_proposal`:
    /// the number of approvals on the proposal must be **strictly greater than**
    /// the proposed new threshold.  This prevents exactly `new_threshold`
    /// colluding admins from using the proposal system to silently dismantle the
    /// multi-sig quorum that was designed to stop them.
    ///
    /// Example: threshold is currently 3 and a proposal wants to reduce it to 2.
    /// The proposal must collect at least 3 approvals (> 2) before it can
    /// execute. A threshold increase has no additional requirement beyond the
    /// live threshold.
    ///
    /// # Errors
    /// - `InvalidProjectData` – `threshold` is 0 or exceeds the current admin count.
    /// - `Unauthorized`       – the current threshold is already above 1; use the
    ///                          proposal system instead.
    pub fn set_admin_approval_threshold(
        env: &Env,
        caller: Address,
        threshold: u32,
    ) -> Result<(), ContractError> {
        caller.require_auth();
        Self::require_admin(env, &caller)?;

        if threshold == 0 || threshold > Self::get_admin_count(env) {
            return Err(ContractError::InvalidProjectData);
        }

        let current_threshold = Self::get_admin_approval_threshold(env);
        if current_threshold > 1 {
            return Err(ContractError::Unauthorized);
        }

        env.storage().persistent().set(
            &crate::storage_keys::ExtensionKey::AdminApprovalThreshold,
            &threshold,
        );

        Ok(())
    }

    pub fn compute_payload_hash(env: &Env, payload: &ProposalPayload) -> soroban_sdk::BytesN<32> {
        let payload_bytes = payload.clone().to_xdr(env);
        env.crypto().sha256(&payload_bytes).into()
    }

    pub fn create_proposal(
        env: &Env,
        proposer: Address,
        payload: ProposalPayload,
        expires_at: u64,
    ) -> Result<u64, ContractError> {
        proposer.require_auth();
        Self::require_admin(env, &proposer)?;

        let id: u64 = env
            .storage()
            .persistent()
            .get(&crate::storage_keys::ExtensionKey::NextAdminProposalId)
            .unwrap_or(0);

        let action_type = match &payload {
            ProposalPayload::AddAdmin(_) => AdminActionType::AdminAdded,
            ProposalPayload::RemoveAdmin(_) => AdminActionType::AdminRemoved,
            ProposalPayload::SetFee(_, _, _, _) => AdminActionType::FeeChanged,
            ProposalPayload::SetThreshold(_) => AdminActionType::ThresholdChanged,
            ProposalPayload::ApproveVerification(_) => AdminActionType::VerificationApproved,
            ProposalPayload::RejectVerification(_) => AdminActionType::VerificationRejected,
            ProposalPayload::RevokeVerification(_, _) => AdminActionType::VerificationRevoked,
        };

        let payload_hash = Self::compute_payload_hash(env, &payload);

        let mut approvals = Map::new(env);
        approvals.set(proposer.clone(), true);

        let threshold = Self::get_admin_approval_threshold(env);
        let status = if approvals.len() >= threshold {
            ProposalStatus::Approved
        } else {
            ProposalStatus::Pending
        };

        let proposal = AdminProposal {
            id,
            proposer,
            action_type,
            payload_hash,
            payload,
            approvals,
            status,
            created_at: env.ledger().timestamp(),
            expires_at,
        };

        env.storage().persistent().set(
            &crate::storage_keys::ExtensionKey::AdminProposal(id),
            &proposal,
        );

        let mut ids = env
            .storage()
            .persistent()
            .get::<_, Vec<u64>>(&crate::storage_keys::ExtensionKey::AdminProposalIds)
            .unwrap_or_else(|| Vec::new(env));
        ids.push_back(id);
        env.storage()
            .persistent()
            .set(&crate::storage_keys::ExtensionKey::AdminProposalIds, &ids);

        env.storage().persistent().set(
            &crate::storage_keys::ExtensionKey::NextAdminProposalId,
            &(id + 1),
        );

        Ok(id)
    }

    pub fn approve_proposal(
        env: &Env,
        admin: Address,
        proposal_id: u64,
    ) -> Result<(), ContractError> {
        admin.require_auth();
        Self::require_admin(env, &admin)?;

        let mut proposal = env
            .storage()
            .persistent()
            .get::<_, AdminProposal>(&crate::storage_keys::ExtensionKey::AdminProposal(
                proposal_id,
            ))
            .ok_or(ContractError::InvalidStatus)?;

        if proposal.status != ProposalStatus::Pending {
            return Err(ContractError::InvalidStatus);
        }

        if proposal.approvals.contains_key(&admin) {
            return Err(ContractError::Unauthorized);
        }

        proposal.approvals.set(admin, true);

        let threshold = Self::get_admin_approval_threshold(env);
        if proposal.approvals.len() >= threshold {
            proposal.status = ProposalStatus::Approved;
        }

        env.storage().persistent().set(
            &crate::storage_keys::ExtensionKey::AdminProposal(proposal_id),
            &proposal,
        );

        Ok(())
    }

    pub fn reject_proposal(
        env: &Env,
        admin: Address,
        proposal_id: u64,
    ) -> Result<(), ContractError> {
        admin.require_auth();
        Self::require_admin(env, &admin)?;

        let mut proposal = env
            .storage()
            .persistent()
            .get::<_, AdminProposal>(&crate::storage_keys::ExtensionKey::AdminProposal(
                proposal_id,
            ))
            .ok_or(ContractError::InvalidStatus)?;

        if proposal.status != ProposalStatus::Pending {
            return Err(ContractError::InvalidStatus);
        }

        proposal.status = ProposalStatus::Rejected;
        env.storage().persistent().set(
            &crate::storage_keys::ExtensionKey::AdminProposal(proposal_id),
            &proposal,
        );

        Ok(())
    }

    pub fn execute_proposal(
        env: &Env,
        caller: Address,
        proposal_id: u64,
    ) -> Result<(), ContractError> {
        caller.require_auth();
        Self::require_admin(env, &caller)?;

        let mut proposal = env
            .storage()
            .persistent()
            .get::<_, AdminProposal>(&crate::storage_keys::ExtensionKey::AdminProposal(
                proposal_id,
            ))
            .ok_or(ContractError::InvalidStatus)?;

        // Re-compute the payload hash and verify it matches the hash stored at
        // proposal creation time. This prevents a proposal whose stored payload
        // has been corrupted (e.g. storage corruption) from being silently
        // executed with unintended effects.
        let computed_hash = Self::compute_payload_hash(env, &proposal.payload);
        if computed_hash != proposal.payload_hash {
            return Err(ContractError::PayloadHashMismatch);
        }

        // Reject stale proposals: if expires_at is non-zero and the current
        // ledger time has reached or passed it, the proposal can no longer be
        // executed regardless of its approval status.
        if proposal.expires_at != 0 && env.ledger().timestamp() >= proposal.expires_at {
            return Err(ContractError::ProposalExpired);
        }

        if proposal.status != ProposalStatus::Approved {
            return Err(ContractError::InvalidStatus);
        }

        let threshold = Self::get_admin_approval_threshold(env);
        if proposal.approvals.len() < threshold {
            return Err(ContractError::Unauthorized);
        }

        match proposal.payload.clone() {
            ProposalPayload::AddAdmin(new_admin) => {
                if !Self::is_admin(env, &new_admin) {
                    env.storage()
                        .persistent()
                        .set(&StorageKey::Admin(new_admin.clone()), &true);
                    let mut admins = Self::get_admin_list(env);
                    admins.push_back(new_admin.clone());
                    env.storage()
                        .persistent()
                        .set(&StorageKey::AdminList, &admins);
                    StorageManager::extend_all_admin_ttl(env, &new_admin);
                    publish_admin_added_event(env, new_admin.clone());
                }
            }
            ProposalPayload::RemoveAdmin(admin_to_remove) => {
                if !Self::is_admin(env, &admin_to_remove) {
                    return Err(ContractError::AdminNotFound);
                }
                let admins = Self::get_admin_list(env);
                if admins.len() <= 1 {
                    return Err(ContractError::CannotRemoveLastAdmin);
                }
                env.storage()
                    .persistent()
                    .remove(&StorageKey::Admin(admin_to_remove.clone()));
                let new_admins = Utils::remove_item_from_vec(env, &admins, &admin_to_remove);
                env.storage()
                    .persistent()
                    .set(&StorageKey::AdminList, &new_admins);
                publish_admin_removed_event(env, admin_to_remove.clone());
            }
            ProposalPayload::SetFee(token, verification_fee, registration_fee, treasury) => {
                let config = FeeConfig {
                    token,
                    verification_fee,
                    registration_fee,
                };
                env.storage()
                    .persistent()
                    .set(&StorageKey::FeeConfig, &config);
                env.storage()
                    .persistent()
                    .set(&StorageKey::Treasury, &treasury);
                crate::events::publish_fee_set_event(
                    env,
                    caller.clone(),
                    config.token.clone(),
                    verification_fee,
                    registration_fee,
                    treasury,
                );
            }
            ProposalPayload::SetThreshold(new_threshold) => {
                if new_threshold == 0 || new_threshold > Self::get_admin_count(env) {
                    return Err(ContractError::InvalidProjectData);
                }

                // Supermajority rule for threshold downgrades:
                //
                // If this proposal would *lower* the current threshold, the number
                // of approvals must be strictly greater than the *current* threshold
                // — not merely greater than the proposed new threshold.
                //
                // Rationale: the quorum that is being dismantled must itself be
                // exceeded, not just the smaller quorum being installed. With a
                // guard of `> new_threshold` only, exactly `current_threshold`
                // colluding admins could create a proposal that passes the live
                // threshold check and yet immediately reduces future quorum.
                // Requiring `> current_threshold` means at least one admin beyond
                // the current quorum must sign off on any reduction.
                //
                // For threshold *increases* or no-ops the normal threshold check
                // (approvals.len() >= current_threshold) already performed above
                // is sufficient; no additional requirement is added.
                let current_threshold = Self::get_admin_approval_threshold(env);
                if new_threshold < current_threshold && proposal.approvals.len() <= current_threshold {
                    return Err(ContractError::ThresholdDowngradeRequiresSupermajority);
                }

                env.storage().persistent().set(
                    &crate::storage_keys::ExtensionKey::AdminApprovalThreshold,
                    &new_threshold,
                );
            }
            ProposalPayload::ApproveVerification(project_id) => {
                let mut project =
                    crate::project_registry::ProjectRegistry::get_project(env, project_id)
                        .ok_or(ContractError::ProjectNotFound)?;
                let mut record =
                    crate::verification_registry::VerificationRegistry::get_verification(
                        env, project_id,
                    )
                    .ok_or(ContractError::VerificationNotFound)?;
                crate::verification_registry::VerificationStateMachine::validate_transition(
                    project.verification_status,
                    VerificationStatus::Verified,
                )?;
                let now = env.ledger().timestamp();
                record.status = VerificationStatus::Verified;
                record.decided_at = now;
                record.expires_at = now.saturating_add(
                    crate::verification_registry::VerificationRegistry::get_verification_duration(
                        env,
                    ),
                );
                env.storage()
                    .persistent()
                    .set(&StorageKey::Verification(project_id), &record.request_id);
                env.storage()
                    .persistent()
                    .set(&StorageKey::VerificationRecord(record.request_id), &record);
                project.verification_status = VerificationStatus::Verified;
                project.current_verification_id = Some(record.request_id);
                project.updated_at = now;
                env.storage()
                    .persistent()
                    .set(&StorageKey::Project(project_id), &project);
                crate::events::publish_verification_approved_event(
                    env,
                    project_id,
                    caller.clone(),
                    now,
                );
            }
            ProposalPayload::RejectVerification(project_id) => {
                let mut project =
                    crate::project_registry::ProjectRegistry::get_project(env, project_id)
                        .ok_or(ContractError::ProjectNotFound)?;
                let mut record =
                    crate::verification_registry::VerificationRegistry::get_verification(
                        env, project_id,
                    )
                    .ok_or(ContractError::VerificationNotFound)?;
                crate::verification_registry::VerificationStateMachine::validate_transition(
                    project.verification_status,
                    VerificationStatus::Rejected,
                )?;
                let now = env.ledger().timestamp();
                record.status = VerificationStatus::Rejected;
                record.decided_at = now;
                env.storage()
                    .persistent()
                    .set(&StorageKey::Verification(project_id), &record.request_id);
                env.storage()
                    .persistent()
                    .set(&StorageKey::VerificationRecord(record.request_id), &record);
                project.verification_status = VerificationStatus::Rejected;
                project.current_verification_id = Some(record.request_id);
                project.updated_at = now;
                env.storage()
                    .persistent()
                    .set(&StorageKey::Project(project_id), &project);
                crate::events::publish_verification_rejected_event(
                    env,
                    project_id,
                    caller.clone(),
                    now,
                );
            }
            ProposalPayload::RevokeVerification(project_id, reason) => {
                let mut project =
                    crate::project_registry::ProjectRegistry::get_project(env, project_id)
                        .ok_or(ContractError::ProjectNotFound)?;
                if project.verification_status != VerificationStatus::Verified {
                    return Err(ContractError::InvalidStatus);
                }
                let mut record =
                    crate::verification_registry::VerificationRegistry::get_verification(
                        env, project_id,
                    )
                    .ok_or(ContractError::VerificationNotFound)?;
                let now = env.ledger().timestamp();
                record.status = VerificationStatus::Unverified;
                record.revoke_reason = Some(reason.clone());
                env.storage()
                    .persistent()
                    .set(&StorageKey::Verification(project_id), &record.request_id);
                env.storage()
                    .persistent()
                    .set(&StorageKey::VerificationRecord(record.request_id), &record);
                project.verification_status = VerificationStatus::Unverified;
                project.current_verification_id = Some(record.request_id);
                project.updated_at = now;
                env.storage()
                    .persistent()
                    .set(&StorageKey::Project(project_id), &project);
                crate::events::publish_verification_revoked_event(
                    env,
                    project_id,
                    caller.clone(),
                    reason,
                );
            }
        }

        proposal.status = ProposalStatus::Executed;
        env.storage().persistent().set(
            &crate::storage_keys::ExtensionKey::AdminProposal(proposal_id),
            &proposal,
        );

        Ok(())
    }

    /// Get the configured verification duration in seconds.
    /// Returns the admin-configured value if set, otherwise the contract default.
    pub fn get_verification_duration(env: &Env) -> u64 {
        env.storage()
            .persistent()
            .get(&StorageKey::VerificationDuration)
            .unwrap_or(DEFAULT_VERIFICATION_DURATION_SECS)
    }

    pub fn get_proposal(env: &Env, proposal_id: u64) -> Option<AdminProposal> {
        env.storage()
            .persistent()
            .get(&crate::storage_keys::ExtensionKey::AdminProposal(
                proposal_id,
            ))
    }

    /// List admin proposals with pagination.
    ///
    /// `start` is a zero-based offset into the proposal ID list and `limit`
    /// caps how many proposals are returned (clamped to `MAX_PAGE_LIMIT`).
    /// Returns the corresponding `AdminProposal` structs for the paginated
    /// slice of IDs, skipping any that are missing from storage.
    pub fn list_proposals(env: &Env, start: u32, limit: u32) -> Vec<AdminProposal> {
        let ids: Vec<u64> = env
            .storage()
            .persistent()
            .get::<_, Vec<u64>>(&crate::storage_keys::ExtensionKey::AdminProposalIds)
            .unwrap_or_else(|| Vec::new(env));
        let page_ids = crate::pagination::paginate(env, &ids, start, limit);
        let mut result = Vec::new(env);
        for proposal_id in page_ids.iter() {
            if let Some(proposal) = env.storage().persistent().get::<_, AdminProposal>(
                &crate::storage_keys::ExtensionKey::AdminProposal(proposal_id),
            ) {
                result.push_back(proposal);
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use crate::errors::ContractError;
    use crate::DongleContract;
    use crate::DongleContractClient;
    use soroban_sdk::{testutils::Address as _, Address, Env};

    #[test]
    fn test_initialize_admin() {
        let env = Env::default();
        let contract_id = env.register(DongleContract, ());
        let client = DongleContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);

        client.mock_all_auths().initialize(&admin);

        assert!(client.is_admin(&admin));
        assert_eq!(client.get_admin_count(), 1);
    }

    #[test]
    #[should_panic]
    fn test_initialize_only_once() {
        let env = Env::default();
        let contract_id = env.register(DongleContract, ());
        let client = DongleContractClient::new(&env, &contract_id);
        let admin1 = Address::generate(&env);
        let admin2 = Address::generate(&env);

        client.mock_all_auths().initialize(&admin1);
        // This should panic
        client.mock_all_auths().initialize(&admin2);
    }

    #[test]
    fn test_add_admin_duplicate() {
        let env = Env::default();
        let contract_id = env.register(DongleContract, ());
        let client = DongleContractClient::new(&env, &contract_id);
        let admin1 = Address::generate(&env);
        let admin2 = Address::generate(&env);

        client.mock_all_auths().initialize(&admin1);
        client.mock_all_auths().add_admin(&admin1, &admin2);
        // Adding the same admin again should be a no-op
        client.mock_all_auths().add_admin(&admin1, &admin2);

        assert!(client.is_admin(&admin2));
        assert_eq!(client.get_admin_count(), 2);
    }

    #[test]
    fn test_add_admin_unauthorized() {
        let env = Env::default();
        let contract_id = env.register(DongleContract, ());
        let client = DongleContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let non_admin = Address::generate(&env);
        let new_admin = Address::generate(&env);

        client.mock_all_auths().initialize(&admin);
        let result = client
            .mock_all_auths()
            .try_add_admin(&non_admin, &new_admin);

        assert_eq!(result, Err(Ok(ContractError::AdminOnly)));
        assert!(!client.is_admin(&new_admin));
    }

    #[test]
    fn test_remove_admin() {
        let env = Env::default();
        let contract_id = env.register(DongleContract, ());
        let client = DongleContractClient::new(&env, &contract_id);
        let admin1 = Address::generate(&env);
        let admin2 = Address::generate(&env);

        client.mock_all_auths().initialize(&admin1);
        client.mock_all_auths().add_admin(&admin1, &admin2);
        client.mock_all_auths().remove_admin(&admin1, &admin2);

        assert!(!client.is_admin(&admin2));
        assert_eq!(client.get_admin_count(), 1);
    }

    #[test]
    fn test_cannot_remove_last_admin() {
        let env = Env::default();
        let contract_id = env.register(DongleContract, ());
        let client = DongleContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);

        client.mock_all_auths().initialize(&admin);
        let result = client.mock_all_auths().try_remove_admin(&admin, &admin);

        assert_eq!(result, Err(Ok(ContractError::CannotRemoveLastAdmin)));
        assert!(client.is_admin(&admin));
    }

    #[test]
    fn test_remove_non_existent_admin() {
        let env = Env::default();
        let contract_id = env.register(DongleContract, ());
        let client = DongleContractClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let non_admin = Address::generate(&env);
        let another_admin = Address::generate(&env);

        client.mock_all_auths().initialize(&admin);
        client.mock_all_auths().add_admin(&admin, &another_admin);
        let result = client.mock_all_auths().try_remove_admin(&admin, &non_admin);

        assert_eq!(result, Err(Ok(ContractError::AdminNotFound)));
        assert!(client.is_admin(&another_admin));
    }

    #[test]
    fn test_remove_admin_twice() {
        let env = Env::default();
        let contract_id = env.register(DongleContract, ());
        let client = DongleContractClient::new(&env, &contract_id);
        let admin1 = Address::generate(&env);
        let admin2 = Address::generate(&env);

        client.mock_all_auths().initialize(&admin1);
        client.mock_all_auths().add_admin(&admin1, &admin2);
        client.mock_all_auths().remove_admin(&admin1, &admin2);
        // Trying to remove the same admin again should fail
        let result = client.mock_all_auths().try_remove_admin(&admin1, &admin2);

        assert_eq!(result, Err(Ok(ContractError::AdminNotFound)));
        assert_eq!(client.get_admin_count(), 1);
    }

    #[test]
    fn test_admin_can_remove_themselves() {
        let env = Env::default();
        let contract_id = env.register(DongleContract, ());
        let client = DongleContractClient::new(&env, &contract_id);
        let admin1 = Address::generate(&env);
        let admin2 = Address::generate(&env);
        let admin3 = Address::generate(&env);

        client.mock_all_auths().initialize(&admin1);
        client.mock_all_auths().add_admin(&admin1, &admin2);
        client.mock_all_auths().add_admin(&admin1, &admin3);

        // Admin2 can remove themselves
        client.mock_all_auths().remove_admin(&admin2, &admin2);

        assert!(client.is_admin(&admin1));
        assert!(!client.is_admin(&admin2));
        assert!(client.is_admin(&admin3));
        assert_eq!(client.get_admin_count(), 2);
    }
}
