//! Fee extensions: waivers, auto-refunds, receipts, and escrow.
//!
//! This module implements four features that extend the core `FeeManager`:
//!
//! 1. **Fee Waivers** — FOSS adoption: OpenSource / Academic / Nonprofit
//!    project owners apply for a waiver; admins (or auto-grant logic) approve.
//! 2. **Auto-refunds** — automatic or owner-triggered refunds tied to project
//!    lifecycle events (deletion, verification rejection, partial refund request).
//! 3. **Fee Receipts** — verifiable proof-of-payment structs with IPFS CID,
//!    admin signature, and batch export support.
//! 4. **Fee Escrow** — owners may dispute a fee to lock it in escrow until an
//!    admin resolves the dispute (full/partial refund or fee maintained).

use crate::admin_manager::AdminManager;
use crate::auth::{require_admin_auth, require_owner_auth};
use crate::errors::ContractError;
use crate::events::{
    FeeOperation,
    publish_auto_refund_claimed_event, publish_auto_refund_created_event,
    publish_fee_escrow_created_event, publish_fee_escrow_released_event,
    publish_fee_receipt_issued_event, publish_fee_receipt_signed_event,
    publish_fee_waiver_applied_event, publish_fee_waiver_decided_event,
};
use crate::fee_manager::FeeManager;
use crate::project_registry::ProjectRegistry;
use crate::storage_keys::{ExtensionKey, FeeExtensionKey, StorageKey};
use crate::types::{
    AutoRefundRecord, EscrowedFee, FeeDisputeResolution, FeeReceipt, FeeWaiverRecord,
    FeeWaiverType, RefundTrigger, WaiverStatus,
};
use crate::utils::Utils;
use soroban_sdk::{Address, Env, String, Vec};

// ── Escrow lock duration (7 days) ─────────────────────────────────────────────
const ESCROW_LOCK_SECONDS: u64 = 7 * 24 * 60 * 60;

pub struct FeeExtensions;

impl FeeExtensions {
    // ── Fee Waiver ─────────────────────────────────────────────────────────────

    /// Apply for a fee waiver as a project owner.
    ///
    /// Submits a `FeeWaiverRecord` in `Pending` state. If the submitted
    /// `proof_cid` matches a pre-approved proof registered by an admin via
    /// `pre_approve_waiver_proof`, the waiver is auto-granted immediately.
    ///
    /// Only one application per project is allowed at a time. Reapplying
    /// after a rejection clears the old record.
    pub fn apply_for_fee_waiver(
        env: &Env,
        project_id: u64,
        proof_cid: String,
        waiver_type: FeeWaiverType,
    ) -> Result<FeeWaiverRecord, ContractError> {
        let project = ProjectRegistry::get_project(env, project_id)
            .ok_or(ContractError::ProjectNotFound)?;

        // Only the project owner may apply.
        project.owner.require_auth();

        // Reject duplicate pending applications.
        if let Some(existing) = Self::get_fee_waiver(env, project_id) {
            if existing.status == WaiverStatus::Pending {
                return Err(ContractError::WaiverAlreadyApplied);
            }
        }

        let now = env.ledger().timestamp();

        // Auto-grant if the proof_cid is pre-approved.
        let status = if Self::is_pre_approved_proof(env, &proof_cid) {
            WaiverStatus::Approved
        } else {
            WaiverStatus::Pending
        };

        let record = FeeWaiverRecord {
            project_id,
            waiver_type,
            proof_cid: proof_cid.clone(),
            applicant: project.owner.clone(),
            status,
            applied_at: now,
            decided_at: if status == WaiverStatus::Approved { now } else { 0 },
            decided_by: None,
        };

        env.storage()
            .persistent()
            .set(&FeeExtensionKey::ProjectFeeWaiver(project_id), &record);

        // Maintain the admin dashboard index.
        let mut ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&FeeExtensionKey::FeeWaiverProjectIds)
            .unwrap_or_else(|| Vec::new(env));
        if !ids.iter().any(|id| id == project_id) {
            ids.push_back(project_id);
            env.storage()
                .persistent()
                .set(&FeeExtensionKey::FeeWaiverProjectIds, &ids);
        }

        publish_fee_waiver_applied_event(
            env,
            project_id,
            project.owner,
            waiver_type,
            proof_cid,
        );

        Ok(record)
    }

    /// Admin: approve or reject a pending fee waiver application.
    pub fn decide_fee_waiver(
        env: &Env,
        admin: Address,
        project_id: u64,
        approve: bool,
    ) -> Result<FeeWaiverRecord, ContractError> {
        require_admin_auth(env, &admin)?;

        let mut record = Self::get_fee_waiver(env, project_id)
            .ok_or(ContractError::WaiverNotFound)?;

        if record.status != WaiverStatus::Pending {
            return Err(ContractError::WaiverNotPending);
        }

        record.status = if approve {
            WaiverStatus::Approved
        } else {
            WaiverStatus::Rejected
        };
        record.decided_at = env.ledger().timestamp();
        record.decided_by = Some(admin.clone());

        env.storage()
            .persistent()
            .set(&FeeExtensionKey::ProjectFeeWaiver(project_id), &record);

        publish_fee_waiver_decided_event(
            env,
            project_id,
            admin,
            record.status,
            record.waiver_type,
        );

        Ok(record)
    }

    /// Admin: register a proof CID for auto-grant (e.g. a well-known GitHub
    /// OSI registry export CID). Any application citing this CID is
    /// immediately approved without manual review.
    pub fn pre_approve_waiver_proof(
        env: &Env,
        admin: Address,
        proof_cid: String,
        waiver_type: FeeWaiverType,
    ) -> Result<(), ContractError> {
        require_admin_auth(env, &admin)?;
        env.storage()
            .persistent()
            .set(&FeeExtensionKey::PreApprovedWaiverProof(proof_cid), &waiver_type);
        Ok(())
    }

    /// Check whether a project has an active (approved) fee waiver.
    pub fn has_active_waiver(env: &Env, project_id: u64) -> bool {
        Self::get_fee_waiver(env, project_id)
            .map(|r| r.status == WaiverStatus::Approved)
            .unwrap_or(false)
    }

    /// Read the waiver record for a project (any status).
    pub fn get_fee_waiver(env: &Env, project_id: u64) -> Option<FeeWaiverRecord> {
        env.storage()
            .persistent()
            .get(&FeeExtensionKey::ProjectFeeWaiver(project_id))
    }

    /// Admin dashboard: list all project IDs that have a waiver record.
    pub fn list_waiver_project_ids(env: &Env) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&FeeExtensionKey::FeeWaiverProjectIds)
            .unwrap_or_else(|| Vec::new(env))
    }

    fn is_pre_approved_proof(env: &Env, proof_cid: &String) -> bool {
        env.storage()
            .persistent()
            .has(&FeeExtensionKey::PreApprovedWaiverProof(proof_cid.clone()))
    }

    // ── Auto-refund ────────────────────────────────────────────────────────────

    /// Record an auto-refund triggered by a lifecycle event.
    ///
    /// Called internally by verification-rejection and project-deletion paths.
    /// Accumulates amount if an unclaimed record already exists (same behaviour
    /// as `record_verification_refund`).
    pub fn record_auto_refund(
        env: &Env,
        project_id: u64,
        payer: Address,
        amount: u128,
        token: Option<Address>,
        trigger: RefundTrigger,
        reason: String,
    ) -> Result<AutoRefundRecord, ContractError> {
        if amount == 0 {
            // Nothing to record.
            let now = env.ledger().timestamp();
            return Ok(AutoRefundRecord {
                project_id,
                trigger,
                reason,
                payer,
                total_amount: 0,
                remaining_amount: 0,
                token,
                created_at: now,
                settled_at: Some(now),
            });
        }

        // Accumulate into any existing unsettled record.
        let (total, remaining) = match Self::get_auto_refund(env, project_id) {
            Some(existing) if existing.settled_at.is_none() => (
                existing
                    .total_amount
                    .checked_add(amount)
                    .ok_or(ContractError::ArithmeticOverflow)?,
                existing
                    .remaining_amount
                    .checked_add(amount)
                    .ok_or(ContractError::ArithmeticOverflow)?,
            ),
            _ => (amount, amount),
        };

        let record = AutoRefundRecord {
            project_id,
            trigger,
            reason,
            payer: payer.clone(),
            total_amount: total,
            remaining_amount: remaining,
            token,
            created_at: env.ledger().timestamp(),
            settled_at: None,
        };

        env.storage()
            .persistent()
            .set(&FeeExtensionKey::AutoRefundRecord(project_id), &record);

        // Index for pending-refund enumeration.
        let mut pending: Vec<u64> = env
            .storage()
            .persistent()
            .get(&FeeExtensionKey::PendingAutoRefundIds)
            .unwrap_or_else(|| Vec::new(env));
        if !pending.iter().any(|id| id == project_id) {
            pending.push_back(project_id);
            env.storage()
                .persistent()
                .set(&FeeExtensionKey::PendingAutoRefundIds, &pending);
        }

        publish_auto_refund_created_event(env, project_id, payer, amount, trigger);

        Ok(record)
    }

    /// Project owner: request a partial refund with a free-form reason.
    ///
    /// This creates (or adds to) an auto-refund record with `RefundTrigger::Manual`.
    /// The actual token transfer is executed when `claim_auto_refund` is called.
    pub fn request_partial_refund(
        env: &Env,
        project_id: u64,
        caller: Address,
        reason: String,
        amount: u128,
    ) -> Result<AutoRefundRecord, ContractError> {
        let project = ProjectRegistry::get_project(env, project_id)
            .ok_or(ContractError::ProjectNotFound)?;

        require_owner_auth(&caller, &project.owner)?;

        // Determine which token to use for the refund.
        let token = FeeManager::get_fee_payment_details(env, project_id)
            .and_then(|r| r.token)
            .or_else(|| {
                FeeManager::get_fee_config(env).ok().and_then(|c| c.token)
            });

        Self::record_auto_refund(
            env,
            project_id,
            project.owner,
            amount,
            token,
            RefundTrigger::Manual,
            reason,
        )
    }

    /// Claim a pending auto-refund. Callable by the payer or any admin.
    ///
    /// Supports partial settlement: pass `payout_amount` ≤ `remaining_amount`.
    /// When `payout_amount == remaining_amount`, the record is marked settled.
    pub fn claim_auto_refund(
        env: &Env,
        caller: Address,
        project_id: u64,
        payout_amount: u128,
    ) -> Result<(), ContractError> {
        caller.require_auth();

        let mut record = Self::get_auto_refund(env, project_id)
            .ok_or(ContractError::AutoRefundNotFound)?;

        if record.settled_at.is_some() {
            return Err(ContractError::AutoRefundAlreadySettled);
        }

        let is_admin = AdminManager::is_admin(env, &caller);
        if caller != record.payer && !is_admin {
            return Err(ContractError::Unauthorized);
        }

        if payout_amount == 0 || payout_amount > record.remaining_amount {
            return Err(ContractError::InvalidInput);
        }

        let token_address = record.token.clone().ok_or(ContractError::FeeConfigNotSet)?;
        let treasury: Address = env
            .storage()
            .persistent()
            .get(&StorageKey::Treasury)
            .ok_or(ContractError::TreasuryNotSet)?;

        // Checks-effects-interactions: update state BEFORE transfer.
        record.remaining_amount = record
            .remaining_amount
            .checked_sub(payout_amount)
            .ok_or(ContractError::ArithmeticOverflow)?;

        let now = env.ledger().timestamp();
        if record.remaining_amount == 0 {
            record.settled_at = Some(now);
            // Remove from pending index once fully settled.
            let pending: Vec<u64> = env
                .storage()
                .persistent()
                .get(&FeeExtensionKey::PendingAutoRefundIds)
                .unwrap_or_else(|| Vec::new(env));
            let updated_pending = Utils::remove_item_from_vec(env, &pending, &project_id);
            env.storage()
                .persistent()
                .set(&FeeExtensionKey::PendingAutoRefundIds, &updated_pending);
        }

        env.storage()
            .persistent()
            .set(&FeeExtensionKey::AutoRefundRecord(project_id), &record);

        // Execute token transfer.
        treasury.require_auth();
        let token_client = soroban_sdk::token::Client::new(env, &token_address);
        token_client.transfer(&treasury, &record.payer, &(payout_amount as i128));

        publish_auto_refund_claimed_event(env, project_id, record.payer.clone(), payout_amount);

        Ok(())
    }

    /// Read the auto-refund record for a project.
    pub fn get_auto_refund(env: &Env, project_id: u64) -> Option<AutoRefundRecord> {
        env.storage()
            .persistent()
            .get(&FeeExtensionKey::AutoRefundRecord(project_id))
    }

    /// List project IDs that have pending (unsettled) auto-refunds.
    pub fn list_pending_auto_refund_ids(env: &Env) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&FeeExtensionKey::PendingAutoRefundIds)
            .unwrap_or_else(|| Vec::new(env))
    }

    // ── Fee Receipts ───────────────────────────────────────────────────────────

    /// Issue a fee receipt for an existing payment record.
    ///
    /// Called internally after a successful `pay_fee` or
    /// `pay_registration_fee`. Returns the receipt with its new `payment_id`.
    pub fn issue_fee_receipt(
        env: &Env,
        project_id: u64,
        payer: Address,
        amount: u128,
        token: Option<Address>,
        operation: FeeOperation,
    ) -> Result<FeeReceipt, ContractError> {
        // Auto-increment payment_id.
        let mut next_id: u64 = env
            .storage()
            .persistent()
            .get(&FeeExtensionKey::NextFeeReceiptId)
            .unwrap_or(0);
        next_id = next_id.checked_add(1).ok_or(ContractError::ArithmeticOverflow)?;
        env.storage()
            .persistent()
            .set(&FeeExtensionKey::NextFeeReceiptId, &next_id);

        let receipt = FeeReceipt {
            payment_id: next_id,
            project_id,
            payer: payer.clone(),
            amount,
            token,
            operation,
            paid_at: env.ledger().timestamp(),
            receipt_cid: None,
            manager_signature: None,
        };

        env.storage()
            .persistent()
            .set(&FeeExtensionKey::FeeReceipt(next_id), &receipt);

        // Index by project.
        if project_id > 0 {
            let mut ids: Vec<u64> = env
                .storage()
                .persistent()
                .get(&FeeExtensionKey::ProjectFeeReceiptIds(project_id))
                .unwrap_or_else(|| Vec::new(env));
            ids.push_back(next_id);
            env.storage()
                .persistent()
                .set(&FeeExtensionKey::ProjectFeeReceiptIds(project_id), &ids);
        }

        // Index by payer address (for registration fees).
        let mut addr_ids: Vec<u64> = env
            .storage()
            .persistent()
            .get(&FeeExtensionKey::AddressFeeReceiptIds(payer.clone()))
            .unwrap_or_else(|| Vec::new(env));
        addr_ids.push_back(next_id);
        env.storage()
            .persistent()
            .set(&FeeExtensionKey::AddressFeeReceiptIds(payer.clone()), &addr_ids);

        publish_fee_receipt_issued_event(
            env,
            next_id,
            project_id,
            payer,
            amount,
            operation,
        );

        Ok(receipt)
    }

    /// Fetch a receipt by `payment_id`.
    pub fn get_fee_receipt(env: &Env, payment_id: u64) -> Option<FeeReceipt> {
        env.storage()
            .persistent()
            .get(&FeeExtensionKey::FeeReceipt(payment_id))
    }

    /// Admin: attach an IPFS CID and a signature to a receipt for audit trail.
    ///
    /// The signature is a free-form string (e.g. base64-encoded Ed25519 sig
    /// over the receipt CID). Indexers can verify it off-chain.
    pub fn sign_fee_receipt(
        env: &Env,
        admin: Address,
        payment_id: u64,
        receipt_cid: String,
        manager_signature: String,
    ) -> Result<FeeReceipt, ContractError> {
        require_admin_auth(env, &admin)?;

        let mut receipt = Self::get_fee_receipt(env, payment_id)
            .ok_or(ContractError::FeeReceiptNotFound)?;

        receipt.receipt_cid = Some(receipt_cid.clone());
        receipt.manager_signature = Some(manager_signature);

        env.storage()
            .persistent()
            .set(&FeeExtensionKey::FeeReceipt(payment_id), &receipt);

        publish_fee_receipt_signed_event(env, payment_id, admin, receipt_cid);

        Ok(receipt)
    }

    /// List all receipt IDs for a project (verification fees).
    pub fn get_project_fee_receipt_ids(env: &Env, project_id: u64) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&FeeExtensionKey::ProjectFeeReceiptIds(project_id))
            .unwrap_or_else(|| Vec::new(env))
    }

    /// List all receipt IDs for an address (registration fees or any payer).
    pub fn get_address_fee_receipt_ids(env: &Env, address: Address) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&FeeExtensionKey::AddressFeeReceiptIds(address))
            .unwrap_or_else(|| Vec::new(env))
    }

    /// Batch-export receipts by payment IDs (for tax/accounting).
    /// Returns only the receipts that actually exist; missing IDs are skipped.
    pub fn batch_export_receipts(env: &Env, payment_ids: Vec<u64>) -> Vec<FeeReceipt> {
        let mut results: Vec<FeeReceipt> = Vec::new(env);
        for i in 0..payment_ids.len() {
            if let Some(id) = payment_ids.get(i) {
                if let Some(receipt) = Self::get_fee_receipt(env, id) {
                    results.push_back(receipt);
                }
            }
        }
        results
    }

    // ── Fee Escrow ─────────────────────────────────────────────────────────────

    /// Project owner: dispute a fee and lock it in escrow.
    ///
    /// The fee must have been paid and not yet consumed (still pending). The
    /// amount is sourced from the `FeePaymentRecord`. The fee payment record
    /// and flag are cleared immediately (the amount is considered "in escrow")
    /// and an `EscrowedFee` record is created.
    ///
    /// Emits `FeeEscrowCreated`.
    pub fn dispute_fee(
        env: &Env,
        project_id: u64,
        caller: Address,
        reason: String,
    ) -> Result<EscrowedFee, ContractError> {
        let project = ProjectRegistry::get_project(env, project_id)
            .ok_or(ContractError::ProjectNotFound)?;

        require_owner_auth(&caller, &project.owner)?;

        // Fee must be in a paid (pending-to-be-consumed) state.
        if !FeeManager::is_fee_paid(env, project_id) {
            return Err(ContractError::FeeNotDisputable);
        }

        // Prevent duplicate disputes.
        if Self::get_escrowed_fee(env, project_id)
            .map(|e| e.released_at.is_none())
            .unwrap_or(false)
        {
            return Err(ContractError::EscrowAlreadyActive);
        }

        let record = FeeManager::get_fee_payment_details(env, project_id)
            .ok_or(ContractError::FeeNotDisputable)?;

        // Remove the paid flag and payment record (funds stay with treasury;
        // the escrow record is the accounting instrument).
        env.storage()
            .persistent()
            .remove(&StorageKey::FeePaidForProject(project_id));
        env.storage()
            .persistent()
            .remove(&ExtensionKey::FeePaymentDetails(project_id));

        let now = env.ledger().timestamp();
        let locked_until = now
            .checked_add(ESCROW_LOCK_SECONDS)
            .ok_or(ContractError::ArithmeticOverflow)?;

        let escrow = EscrowedFee {
            project_id,
            amount: record.amount,
            token: record.token.clone(),
            locked_until,
            dispute_reason: reason.clone(),
            disputer: project.owner.clone(),
            created_at: now,
            released_at: None,
            resolution: None,
            resolved_by: None,
        };

        env.storage()
            .persistent()
            .set(&FeeExtensionKey::EscrowedFee(project_id), &escrow);

        // Index.
        let mut active: Vec<u64> = env
            .storage()
            .persistent()
            .get(&FeeExtensionKey::ActiveEscrowProjectIds)
            .unwrap_or_else(|| Vec::new(env));
        if !active.iter().any(|id| id == project_id) {
            active.push_back(project_id);
            env.storage()
                .persistent()
                .set(&FeeExtensionKey::ActiveEscrowProjectIds, &active);
        }

        publish_fee_escrow_created_event(
            env,
            project_id,
            project.owner,
            record.amount,
            reason,
            locked_until,
        );

        Ok(escrow)
    }

    /// Admin: resolve a fee dispute.
    ///
    /// - `FullRefund` → transfers the full escrowed amount back to the payer.
    /// - `FeeMaintained` → amount stays in treasury (no transfer).
    /// - `PartialRefund(n)` → transfers `n` tokens back; remainder goes to treasury.
    ///
    /// Emits `FeeEscrowReleased`.
    pub fn resolve_fee_dispute(
        env: &Env,
        admin: Address,
        project_id: u64,
        resolution: FeeDisputeResolution,
    ) -> Result<EscrowedFee, ContractError> {
        require_admin_auth(env, &admin)?;

        let mut escrow = Self::get_escrowed_fee(env, project_id)
            .ok_or(ContractError::EscrowNotFound)?;

        if escrow.released_at.is_some() {
            return Err(ContractError::EscrowNotFound);
        }

        let refund_amount = match resolution {
            FeeDisputeResolution::FullRefund => escrow.amount,
            FeeDisputeResolution::FeeMaintained => 0u128,
            FeeDisputeResolution::PartialRefund(n) => {
                if n > escrow.amount {
                    return Err(ContractError::PartialRefundExceedsEscrow);
                }
                n
            }
        };

        let now = env.ledger().timestamp();
        escrow.released_at = Some(now);
        escrow.resolution = Some(resolution);
        escrow.resolved_by = Some(admin.clone());

        // Write state BEFORE any token transfer (CEI).
        env.storage()
            .persistent()
            .set(&FeeExtensionKey::EscrowedFee(project_id), &escrow);

        // Remove from active index.
        let active: Vec<u64> = env
            .storage()
            .persistent()
            .get(&FeeExtensionKey::ActiveEscrowProjectIds)
            .unwrap_or_else(|| Vec::new(env));
        let updated_active = Utils::remove_item_from_vec(env, &active, &project_id);
        env.storage()
            .persistent()
            .set(&FeeExtensionKey::ActiveEscrowProjectIds, &updated_active);

        // Execute token transfer if a refund is owed.
        if refund_amount > 0 {
            let token_address = escrow.token.clone().ok_or(ContractError::FeeConfigNotSet)?;
            let treasury: Address = env
                .storage()
                .persistent()
                .get(&StorageKey::Treasury)
                .ok_or(ContractError::TreasuryNotSet)?;

            treasury.require_auth();
            let token_client = soroban_sdk::token::Client::new(env, &token_address);
            token_client.transfer(&treasury, &escrow.disputer, &(refund_amount as i128));
        }

        publish_fee_escrow_released_event(
            env,
            project_id,
            admin,
            escrow.amount,
            resolution,
        );

        Ok(escrow)
    }

    /// Read the escrow record for a project.
    pub fn get_escrowed_fee(env: &Env, project_id: u64) -> Option<EscrowedFee> {
        env.storage()
            .persistent()
            .get(&FeeExtensionKey::EscrowedFee(project_id))
    }

    /// List project IDs with an active (unresolved) fee escrow.
    pub fn list_active_escrow_project_ids(env: &Env) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&FeeExtensionKey::ActiveEscrowProjectIds)
            .unwrap_or_else(|| Vec::new(env))
    }
}
