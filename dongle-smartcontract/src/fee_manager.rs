//! Fee configuration and payment with validation and events.

use crate::admin_action_log::AdminActionLog;
use crate::auth::{require_admin_auth, require_self_auth};
use crate::constants::FEE_PAYMENT_EXPIRY_SECONDS;
use crate::errors::ContractError;
use crate::events::{
    publish_fee_consumed_event, publish_fee_paid_event, publish_fee_set_event, FeeOperation,
};
use crate::constants::FEE_PAYMENT_EXPIRY_SECONDS;
use crate::project_registry::ProjectRegistry;
use crate::storage_keys::{ExtensionKey, FeeHistoryKey, StorageKey};
use crate::types::{
    AdminActionType, FeeConfig, FeeConfigHistoryEntry, FeePaymentRecord, FeeRefundRecord,
};
use soroban_sdk::{Address, Env};

pub struct FeeManager;

impl FeeManager {
    /// Configure fees for the contract (admin only)
    pub fn set_fee(
        env: &Env,
        admin: Address,
        token: Option<Address>,
        verification_fee: u128,
        registration_fee: u128,
        treasury: Address,
    ) -> Result<(), ContractError> {
        require_admin_auth(env, &admin)?;

        if crate::admin_manager::AdminManager::get_admin_approval_threshold(env) > 1 {
            return Err(ContractError::Unauthorized);
        }

        let old_config = env
            .storage()
            .persistent()
            .get::<_, FeeConfig>(&StorageKey::FeeConfig);
        let old_treasury = env
            .storage()
            .persistent()
            .get::<_, Address>(&StorageKey::Treasury);

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

        let history_entry = FeeConfigHistoryEntry {
            admin: admin.clone(),
            old_token: old_config.as_ref().and_then(|config| config.token.clone()),
            old_verification_fee: old_config.as_ref().map(|config| config.verification_fee),
            old_registration_fee: old_config.as_ref().map(|config| config.registration_fee),
            old_treasury,
            token: config.token.clone(),
            verification_fee,
            registration_fee,
            treasury: treasury.clone(),
            timestamp: env.ledger().timestamp(),
        };
        let mut history: Vec<FeeConfigHistoryEntry> = env
            .storage()
            .persistent()
            .get(&ExtensionKey::FeeConfigHistory)
            .unwrap_or_else(|| Vec::new(env));
        history.push_back(history_entry);
        env.storage()
            .persistent()
            .set(&ExtensionKey::FeeConfigHistory, &history);

        publish_fee_set_event(
            env,
            admin.clone(),
            config.token.clone(),
            verification_fee,
            registration_fee,
            treasury,
        );

        AdminActionLog::record_action(env, admin, AdminActionType::FeeChanged, None, None, None);

        Ok(())
    }

    /// Shared payment path for verification and registration fees.
    ///
    /// Validates fee config/treasury, transfers tokens (when amount > 0), sets the
    /// paid flag, stores a [`FeePaymentRecord`], and emits a fee-paid event.
    #[allow(clippy::too_many_arguments)]
    fn execute_fee_payment(
        env: &Env,
        payer: Address,
        amount: u128,
        token: Option<Address>,
        paid_flag_key: StorageKey,
        details_key: ExtensionKey,
        event_project_id: u64,
        operation: FeeOperation,
    ) -> Result<(), ContractError> {
        let config = Self::get_fee_config(env)?;
        let treasury: Address = env
            .storage()
            .persistent()
            .get(&StorageKey::Treasury)
            .ok_or(ContractError::TreasuryNotSet)?;

        if config.token != token {
            return Err(ContractError::InvalidProjectData);
        }

        if amount > 0 {
            // Safety: fee amounts are stored as u128 but the token interface requires i128.
            // Reject any value that exceeds i128::MAX to prevent a silent truncating cast.
            if amount > i128::MAX as u128 {
                return Err(ContractError::InvalidProjectData);
            }
            // set_fee enforces that token is Some when fees are non-zero, so this
            // ok_or branch is a defensive guard against corrupted storage state.
            let token_address = config.token.ok_or(ContractError::FeeConfigNotSet)?;
            let client = soroban_sdk::token::Client::new(env, &token_address);
            // Transfer must succeed before we set the payment flag.
            // If transfer fails, this function returns early without setting the flag.
            client.transfer(&payer, &treasury, &(amount as i128));
        }

        // Only set payment flag after successful token transfer
        env.storage().persistent().set(&paid_flag_key, &true);

        // Store full payment details for getter
        let payment_record = FeePaymentRecord {
            paid_at: env.ledger().timestamp(),
            payer: payer.clone(),
            amount,
            token: token.clone(),
        };
        env.storage()
            .persistent()
            .set(&details_key, &payment_record);

        // Only emit event after successful payment
        publish_fee_paid_event(env, event_project_id, payer, token, operation, amount);
        Ok(())
    }

    /// Pay the verification fee for a project.
    /// Only the project owner may pay; third-party payments are rejected.
    ///
    /// # Behavior on Token Transfer Failure
    /// - If the token transfer fails (e.g., insufficient balance), the payment flag is NOT set
    /// - The fee paid event is NOT emitted
    /// - The caller receives an error and can retry after acquiring sufficient tokens
    ///
    /// # Note: Code Duplication
    /// This function has similar logic to `pay_registration_fee()`. Consider consolidating
    /// these functions in a future refactor to accept an operation type parameter.
    pub fn pay_fee(
        env: &Env,
        payer: Address,
        project_id: u64,
        token: Option<Address>,
    ) -> Result<(), ContractError> {
        require_self_auth(&payer);

        // Enforce owner-only payment
        let project =
            ProjectRegistry::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;
        if project.owner != payer {
            return Err(ContractError::Unauthorized);
        }

        let amount = Self::get_fee_config(env)?.verification_fee;
        Self::execute_fee_payment(
            env,
            payer,
            amount,
            token,
            StorageKey::FeePaidForProject(project_id),
            ExtensionKey::FeePaymentDetails(project_id),
            project_id,
            FeeOperation::Verification,
        )
    }

    /// Check if the fee has been paid for a project
    pub fn is_fee_paid(env: &Env, project_id: u64) -> bool {
        env.storage()
            .persistent()
            .get(&StorageKey::FeePaidForProject(project_id))
            .unwrap_or(false)
    }

    /// Shared consume helper that removes a paid flag and emits the consumed event.
    /// Used by both `consume_fee_payment` and `consume_registration_fee_payment`.
    fn execute_consume_fee_payment(
        env: &Env,
        paid_key: StorageKey,
        event_project_id: u64,
        caller: Address,
        operation: FeeOperation,
        amount: u128,
    ) -> Result<(), ContractError> {
        env.storage().persistent().remove(&paid_key);
        publish_fee_consumed_event(env, event_project_id, caller, operation, amount);
        Ok(())
    }

    /// Consume the fee payment (used during verification request)
    pub fn consume_fee_payment(
        env: &Env,
        project_id: u64,
        caller: Address,
        amount: u128,
    ) -> Result<(), ContractError> {
        if !Self::is_fee_paid(env, project_id) {
            return Err(ContractError::InsufficientFee);
        }
        let record = Self::get_fee_payment_details(env, project_id)
            .ok_or(ContractError::InsufficientFee)?;
        let now = env.ledger().timestamp();
        if now >= record.paid_at + FEE_PAYMENT_EXPIRY_SECONDS {
            return Err(ContractError::FeePaymentExpired);
        }
        Self::execute_consume_fee_payment(
            env,
            StorageKey::FeePaidForProject(project_id),
            project_id,
            caller,
            FeeOperation::Verification,
            amount,
        )
    }

    /// Get current fee configuration
    pub fn get_fee_config(env: &Env) -> Result<FeeConfig, ContractError> {
        env.storage()
            .persistent()
            .get(&StorageKey::FeeConfig)
            .ok_or(ContractError::FeeConfigNotSet)
    }

    /// Get all fee configuration changes in chronological order (oldest first).
    pub fn get_fee_config_history(env: &Env) -> Vec<FeeConfigHistoryEntry> {
        env.storage()
            .persistent()
            .get(&ExtensionKey::FeeConfigHistory)
            .unwrap_or_else(|| Vec::new(env))
    }

    /// Pay the registration fee for a project.
    /// Only the project owner may pay; third-party payments are rejected.
    ///
    /// # Behavior on Token Transfer Failure
    /// - If the token transfer fails (e.g., insufficient balance), the payment flag is NOT set
    /// - The fee paid event is NOT emitted
    /// - The caller receives an error and can retry after acquiring sufficient tokens
    ///
    /// # Note: Code Duplication
    /// This function has similar logic to `pay_fee()`. Consider consolidating
    /// these functions in a future refactor to accept an operation type parameter.
    pub fn pay_registration_fee(
        env: &Env,
        payer: Address,
        token: Option<Address>,
    ) -> Result<(), ContractError> {
        require_self_auth(&payer);

        let amount = Self::get_fee_config(env)?.registration_fee;
        Self::execute_fee_payment(
            env,
            payer.clone(),
            amount,
            token,
            StorageKey::RegistrationFeePaidForAddress(payer.clone()),
            ExtensionKey::RegistrationFeePaymentDetails(payer),
            0,
            FeeOperation::Registration,
        )
    }

    /// Get fee payment details for a project (payer, amount, token, timestamp)
    pub fn get_fee_payment_details(env: &Env, project_id: u64) -> Option<FeePaymentRecord> {
        env.storage()
            .persistent()
            .get(&ExtensionKey::FeePaymentDetails(project_id))
    }

    /// Get registration fee payment details for an address
    pub fn get_registration_fee_payment_details(
        env: &Env,
        address: &Address,
    ) -> Option<FeePaymentRecord> {
        env.storage()
            .persistent()
            .get(&ExtensionKey::RegistrationFeePaymentDetails(
                address.clone(),
            ))
    }

    /// Check if the registration fee has been paid for an address
    pub fn is_registration_fee_paid(env: &Env, address: &Address) -> bool {
        env.storage()
            .persistent()
            .get(&StorageKey::RegistrationFeePaidForAddress(address.clone()))
            .unwrap_or(false)
    }

    /// Consume the registration fee payment (used during project registration)
    pub fn consume_registration_fee_payment(
        env: &Env,
        address: &Address,
        amount: u128,
    ) -> Result<(), ContractError> {
        if !Self::is_registration_fee_paid(env, address) {
            return Err(ContractError::InsufficientFee);
        }
        let record = Self::get_registration_fee_payment_details(env, address)
            .ok_or(ContractError::InsufficientFee)?;
        let now = env.ledger().timestamp();
        if now >= record.paid_at + FEE_PAYMENT_EXPIRY_SECONDS {
            return Err(ContractError::FeePaymentExpired);
        }
        Self::execute_consume_fee_payment(
            env,
            StorageKey::RegistrationFeePaidForAddress(address.clone()),
            0,
            address.clone(),
            FeeOperation::Registration,
            amount,
        )
    }

    /// Cancel a pending verification fee payment and refund the payer if applicable.
    /// Only the payer (project owner) or a contract administrator can cancel.
    pub fn cancel_fee_payment(
        env: &Env,
        caller: Address,
        project_id: u64,
    ) -> Result<(), ContractError> {
        // Enforce eligibility: Fee must have been paid
        if !Self::is_fee_paid(env, project_id) {
            return Err(ContractError::InsufficientFee);
        }

        let record =
            Self::get_fee_payment_details(env, project_id).ok_or(ContractError::InsufficientFee)?;

        // Authorization: Payer or Admin only
        let is_admin = crate::admin_manager::AdminManager::is_admin(env, &caller);
        if caller != record.payer && !is_admin {
            return Err(ContractError::Unauthorized);
        }

        // Cannot cancel if verification is already Pending or Verified
        if let Some(project) = ProjectRegistry::get_project(env, project_id) {
            if project.verification_status == crate::types::VerificationStatus::Pending
                || project.verification_status == crate::types::VerificationStatus::Verified
            {
                return Err(ContractError::InvalidStatus);
            }
        }

        // Process refund if fee amount > 0 and token is configured
        if record.amount > 0 {
            let token_address = record.token.clone().ok_or(ContractError::FeeConfigNotSet)?;
            let treasury: Address = env
                .storage()
                .persistent()
                .get(&StorageKey::Treasury)
                .ok_or(ContractError::TreasuryNotSet)?;

            // Remove payment records from storage BEFORE executing the token transfer.
            // This follows the checks-effects-interactions pattern: the state
            // transition (Pending → Cancelled) is written atomically before the
            // outbound transfer, so that even if re-entrant logic were possible
            // in a future Soroban version the payment flag could never be
            // consumed a second time.  In the current Soroban WASM sandbox,
            // re-entrancy is not possible, but the ordering is preserved here
            // for correctness and consistency with `claim_fee_refund`.
            env.storage()
                .persistent()
                .remove(&StorageKey::FeePaidForProject(project_id));
            env.storage()
                .persistent()
                .remove(&ExtensionKey::FeePaymentDetails(project_id));

            // Treasury authorization is required to transfer tokens out of the treasury
            treasury.require_auth();
            let token_client = soroban_sdk::token::Client::new(env, &token_address);
            token_client.transfer(&treasury, &record.payer, &(record.amount as i128));
        } else {
            // Zero-fee cancellation: just remove the storage flags.
            env.storage()
                .persistent()
                .remove(&StorageKey::FeePaidForProject(project_id));
            env.storage()
                .persistent()
                .remove(&ExtensionKey::FeePaymentDetails(project_id));
        }

        // Publish event
        crate::events::publish_fee_cancelled_event(
            env,
            project_id,
            caller.clone(),
            record.payer.clone(),
            crate::events::FeeOperation::Verification,
            record.amount,
        );

        // Record admin action if cancelled by an admin
        if is_admin {
            AdminActionLog::record_action(
                env,
                caller,
                AdminActionType::FeeRefunded,
                Some(project_id),
                None,
                None,
            );
        }

        Ok(())
    }

    // ── Verification fee refunds (issue #472) ────────────────────────────────

    /// Record a claimable refund for a rejected verification request.
    ///
    /// Called by `reject_verification`. The payout is *recorded*, not executed:
    /// moving tokens out of the treasury needs `treasury.require_auth()`, and
    /// the rejecting admin is generally not the treasury signer. Forcing an
    /// immediate transfer would make every rejection require a second
    /// signature, so a rejection that could not pay out would either fail
    /// outright or leave the project rejected with the fee silently kept.
    ///
    /// `amount` comes from `VerificationRecord::fee_amount` rather than the
    /// live payment record: `request_verification` already *consumed* the
    /// payment (it removes `FeePaidForProject`), so by the time a request is
    /// rejected there is no outstanding payment left to inspect. The
    /// verification record is the durable statement of what this request cost.
    ///
    /// Returns `Ok(None)` when there is nothing to refund — a zero fee, or no
    /// payment details on record — so rejection still succeeds on a fee-free
    /// deployment.
    pub fn record_verification_refund(
        env: &Env,
        project_id: u64,
        request_id: u64,
        payer: Address,
        amount: u128,
    ) -> Result<Option<FeeRefundRecord>, ContractError> {
        if amount == 0 {
            return Ok(None);
        }

        // Prefer the token actually paid in; fall back to the configured token
        // so a refund is still expressible if the payment details were pruned.
        let token = match Self::get_fee_payment_details(env, project_id) {
            Some(record) => record.token,
            None => Self::get_fee_config(env)
                .ok()
                .and_then(|config| config.token),
        };

        // A project may be rejected more than once (Rejected -> Pending is a
        // legal transition, so the owner can pay and re-request). An unclaimed
        // refund from an earlier rejection must not be overwritten — that would
        // silently erase a real debt — and it must not block the admin from
        // rejecting either. Accumulate instead.
        let amount = match Self::get_fee_refund(env, project_id) {
            Some(existing) if existing.claimed_at.is_none() => existing
                .amount
                .checked_add(amount)
                .ok_or(ContractError::ArithmeticOverflow)?,
            _ => amount,
        };

        let refund = FeeRefundRecord {
            project_id,
            request_id,
            payer,
            amount,
            token,
            created_at: env.ledger().timestamp(),
            claimed_at: None,
        };

        env.storage()
            .persistent()
            .set(&ExtensionKey::FeeRefund(project_id), &refund);

        Ok(Some(refund))
    }

    /// Read the refund recorded for `project_id`, claimed or not.
    pub fn get_fee_refund(env: &Env, project_id: u64) -> Option<FeeRefundRecord> {
        env.storage()
            .persistent()
            .get(&ExtensionKey::FeeRefund(project_id))
    }

    /// Pay out a recorded refund.
    ///
    /// Callable by the payer or any admin; the tokens always go to the
    /// recorded payer regardless of who calls, so an admin settling on
    /// someone's behalf cannot redirect the funds.
    pub fn claim_fee_refund(
        env: &Env,
        caller: Address,
        project_id: u64,
    ) -> Result<(), ContractError> {
        caller.require_auth();

        let mut refund =
            Self::get_fee_refund(env, project_id).ok_or(ContractError::NoRefundAvailable)?;

        if refund.claimed_at.is_some() {
            return Err(ContractError::RefundAlreadyClaimed);
        }

        let is_admin = crate::admin_manager::AdminManager::is_admin(env, &caller);
        if caller != refund.payer && !is_admin {
            return Err(ContractError::Unauthorized);
        }

        let token_address = refund.token.clone().ok_or(ContractError::FeeConfigNotSet)?;
        let treasury: Address = env
            .storage()
            .persistent()
            .get(&StorageKey::Treasury)
            .ok_or(ContractError::TreasuryNotSet)?;

        // Mark claimed before transferring. If the transfer panics the whole
        // invocation reverts, so this cannot leave a claimed-but-unpaid
        // record — but it does close the door on re-entrant double claims.
        refund.claimed_at = Some(env.ledger().timestamp());
        env.storage()
            .persistent()
            .set(&ExtensionKey::FeeRefund(project_id), &refund);

        treasury.require_auth();
        let token_client = soroban_sdk::token::Client::new(env, &token_address);
        token_client.transfer(&treasury, &refund.payer, &(refund.amount as i128));

        crate::events::publish_fee_refunded_event(
            env,
            project_id,
            refund.request_id,
            refund.payer.clone(),
            refund.amount,
        );

        if is_admin {
            AdminActionLog::record_action(
                env,
                caller,
                AdminActionType::FeeRefunded,
                Some(project_id),
                None,
                None,
            );
        }

        Ok(())
    }
}
