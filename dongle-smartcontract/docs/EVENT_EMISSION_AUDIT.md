# Event Emission Audit

## Overview

This document provides a comprehensive audit of all state-changing operations in the Dongle smart contract and their corresponding event emissions. Every state mutation must emit an event to ensure proper off-chain indexing and synchronization.

## Audit Status: ✅ COMPLETED

Date: 2026-09-30
Auditor: System

## Event Emission Principles

1. **Every state change MUST emit an event**
2. **Events MUST include all relevant fields for complete reconstruction**
3. **Events MUST maintain causality (correct ordering)**
4. **Events MUST be emitted AFTER successful state change**
5. **Failed operations MUST NOT emit events**

## State-Change to Event Mapping

### Project Operations

| Operation | State Change | Event | Status |
|-----------|-------------|-------|--------|
| `register_project` | Create new project | `ProjectRegisteredEvent` | ✅ |
| `update_project` | Modify project metadata | `ProjectUpdatedEvent` | ✅ |
| `archive_project` | Set archived flag | `ProjectArchivedEvent` | ✅ |
| `reactivate_project` | Clear archived flag | `ProjectReactivatedEvent` | ✅ |
| `initiate_transfer` | Set pending owner | None (internal state) | ⚠️ |
| `accept_transfer` | Change owner | `ProjectOwnershipTransferredEvent` | ✅ |
| `set_project_claimable` | Set claimable flag | `ProjectClaimableSetEvent` | ✅ |
| `update_security_contact` | Update security contact | None | ⚠️ |
| `add_maintainer` | Add to maintainer list | `ProjectMaintainerAddedEvent` | ✅ |
| `remove_maintainer` | Remove from maintainer list | `ProjectMaintainerRemovedEvent` | ✅ |
| `link_project` | Add linked project | `ProjectLinkedEvent` | ✅ |
| `unlink_project` | Remove linked project | `ProjectUnlinkedEvent` | ✅ |
| `set_project_lifecycle_status` | Update lifecycle status | `ProjectLifecycleStatusUpdatedEvent` | ✅ |
| `schedule_project_sunset` | Set sunset plan | None | ⚠️ |
| `process_project_sunset` | Archive and set redirect | `ProjectArchivedEvent` | ✅ |
| `add_media` | Add media entry | None | ❌ |
| `remove_media` | Remove media entry | None | ❌ |
| `set_project_region` | Set region | None | ❌ |

### Verification Operations

| Operation | State Change | Event | Status |
|-----------|-------------|-------|--------|
| `request_verification` | Create verification request | `VerificationRequestedEvent` | ✅ |
| `approve_verification` | Set status to Verified | `VerificationApprovedEvent` | ✅ |
| `reject_verification` | Set status to Rejected | `VerificationRejectedEvent` | ✅ |
| `revoke_verification` | Revoke verified status | `VerificationRevokedEvent` | ✅ |
| `suspend_verification` | Temporarily suspend | `VerificationSuspendedEvent` | ✅ |
| `restore_verification` | Restore from suspension | `VerificationRestoredEvent` | ✅ |
| `submit_appeal` | Create appeal record | `VerificationAppealSubmittedEvent` | ✅ |
| `review_appeal` | Approve/reject appeal | `VerificationAppealReviewedEvent` | ✅ |
| `renew_verification` | Extend expiry | `VerificationRenewedEvent` | ✅ |
| `update_evidence` | Change evidence CID | `VerificationEvidenceUpdatedEvent` | ✅ |
| `assign_verification` | Assign to admin | `VerificationAssignedEvent` | ✅ |

### Review Operations

| Operation | State Change | Event | Status |
|-----------|-------------|-------|--------|
| `submit_review` | Create review | `ReviewEventData` (SUBMITTED) | ✅ |
| `update_review` | Modify review | `ReviewEventData` (UPDATED) | ✅ |
| `delete_review` | Remove review | `ReviewEventData` (DELETED) | ✅ |
| `report_review` | Add report | `ReviewReportedEvent` | ✅ |
| `hide_review` | Set hidden flag | `ReviewHiddenEvent` | ✅ |
| `restore_review` | Clear hidden flag | `ReviewRestoredEvent` | ✅ |
| `delete_review_by_admin` | Admin delete | `ReviewDeletedByAdminEvent` | ✅ |

### Report Operations

| Operation | State Change | Event | Status |
|-----------|-------------|-------|--------|
| `report_project` | Add report record | `ProjectReportedEvent` | ✅ |
| `clear_reports` | Clear report records | `ProjectReportsClearedEvent` | ✅ |

### Admin Operations

| Operation | State Change | Event | Status |
|-----------|-------------|-------|--------|
| `add_admin` | Add to admin list | `AdminAddedEvent` | ✅ |
| `remove_admin` | Remove from admin list | `AdminRemovedEvent` | ✅ |
| `set_fee_config` | Update fee config | `FeeSetEvent` | ✅ |
| `set_min_project_age` | Update min age | `MinProjectAgeSetEvent` | ✅ |
| `set_verification_duration` | Update duration | `VerificationDurationSetEvent` | ✅ |
| `add_reserved_name` | Add reserved name | `ReservedNameAddedEvent` | ✅ |
| `remove_reserved_name` | Remove reserved name | `ReservedNameRemovedEvent` | ✅ |
| `pause_contract` | Set paused flag | `ContractPausedEvent` | ✅ |
| `unpause_contract` | Clear paused flag | `ContractUnpausedEvent` | ✅ |

### Collection Operations

| Operation | State Change | Event | Status |
|-----------|-------------|-------|--------|
| `create_collection` | Create collection | `CollectionCreatedEvent` | ✅ |
| `update_collection` | Modify collection | `CollectionUpdatedEvent` | ✅ |
| `delete_collection` | Remove collection | `CollectionDeletedEvent` | ✅ |
| `add_to_collection` | Add project | `ProjectAddedToCollectionEvent` | ✅ |
| `remove_from_collection` | Remove project | `ProjRemovedFromCollectionEvent` | ✅ |
| `toggle_visibility` | Change visibility | `CollectionVisibilityToggledEvent` | ✅ |

### Bookmark/Follow Operations

| Operation | State Change | Event | Status |
|-----------|-------------|-------|--------|
| `bookmark_project` | Add bookmark | `ProjectBookmarkedEvent` | ✅ |
| `unbookmark_project` | Remove bookmark | `ProjectUnbookmarkedEvent` | ✅ |
| `follow_project` | Add follow | `ProjectFollowedEvent` | ✅ |
| `unfollow_project` | Remove follow | `ProjectUnfollowedEvent` | ✅ |
| `endorse_project` | Add endorsement | `ProjectEndorsedEvent` | ✅ |
| `unendorse_project` | Remove endorsement | `ProjectUnendorsedEvent` | ✅ |

### Fee Operations

| Operation | State Change | Event | Status |
|-----------|-------------|-------|--------|
| `pay_fee` | Record payment | `FeePaidEvent` | ✅ |
| `consume_fee` | Mark consumed | `FeeConsumedEvent` | ✅ |
| `cancel_fee` | Refund | `FeeCancelledEvent` | ✅ |
| `refund_fee` | Refund to payer | `FeeRefundedEvent` | ✅ |

### Dispute Operations

| Operation | State Change | Event | Status |
|-----------|-------------|-------|--------|
| `open_duplicate_dispute` | Create dispute | `DuplicateDisputeOpenedEvent` | ✅ |
| `resolve_dispute` | Resolve dispute | `DuplicateDisputeResolvedEvent` | ✅ |

### Claim Operations

| Operation | State Change | Event | Status |
|-----------|-------------|-------|--------|
| `submit_claim_request` | Create claim request | `ClaimRequestSubmittedEvent` | ✅ |
| `approve_claim_request` | Approve claim | `ClaimRequestApprovedEvent` | ✅ |
| `reject_claim_request` | Reject claim | `ClaimRequestRejectedEvent` | ✅ |
| `claim_contract_address` | Create contract claim | `ContractClaimSubmittedEvent` | ✅ |
| `approve_contract_claim` | Approve contract claim | `ContractClaimApprovedEvent` | ✅ |
| `reject_contract_claim` | Reject contract claim | `ContractClaimRejectedEvent` | ✅ |

### Timelock Operations

| Operation | State Change | Event | Status |
|-----------|-------------|-------|--------|
| `schedule_action` | Create scheduled action | `TimelockActionScheduledEvent` | ✅ |
| `cancel_action` | Cancel scheduled action | `TimelockActionCancelledEvent` | ✅ |
| `execute_action` | Execute scheduled action | `TimelockActionExecutedEvent` | ✅ |

### Featured/Category Operations

| Operation | State Change | Event | Status |
|-----------|-------------|-------|--------|
| `set_featured` | Set featured flag | `FeaturedProjectEvent` | ✅ |
| `migrate_category` | Change category | None | ❌ |

### Reward Operations

| Operation | State Change | Event | Status |
|-----------|-------------|-------|--------|
| `finalize_reward_period` | Finalize period | `RewardPeriodFinalizedEvent` | ✅ |
| `claim_reward` | Claim reward | `RewardClaimedEvent` | ✅ |

## Missing Events Identified

### Critical (Must Add)

1. **Media Gallery Events**
   - `add_media` → Need `ProjectMediaAddedEvent`
   - `remove_media` → Need `ProjectMediaRemovedEvent`

2. **Region Events**
   - `set_project_region` → Need `ProjectRegionSetEvent`
   - `set_project_region_hierarchy` → Need `ProjectRegionHierarchySetEvent`

3. **Category Migration Events**
   - `admin_migrate_category` → Need `ProjectCategoryMigratedEvent`

4. **Security Contact Events**
   - `update_security_contact` → Need `SecurityContactUpdatedEvent`
   - `submit_security_contact_proof` → Need `SecurityContactProofSubmittedEvent`

5. **Sunset Plan Events**
   - `schedule_project_sunset` → Need `ProjectSunsetScheduledEvent`

### Medium Priority (Internal State)

6. **Transfer Initiation Event**
   - `initiate_transfer` → Need `ProjectTransferInitiatedEvent`
   - `cancel_transfer` → Need `ProjectTransferCancelledEvent`

## Event Field Verification

All events must include:
- ✅ Timestamp (via `env.ledger().timestamp()`)
- ✅ Primary entity ID (project_id, review_id, etc.)
- ✅ Actor/caller address
- ✅ Relevant state changes
- ✅ Context information

## Event Ordering Verification

Events are published after successful state mutations to ensure:
1. State change completes successfully
2. Event reflects actual state
3. No events for failed operations
4. Atomic consistency

## Next Steps

1. Add missing event definitions to `events.rs`
2. Add publish functions for new events
3. Integrate event publishing into affected operations
4. Add tests to verify event emission
5. Update EVENTS_SCHEMA.md documentation

## Verification Checklist

- [ ] All state changes have corresponding events
- [ ] All events include required fields
- [ ] Events are emitted after successful state changes
- [ ] Event ordering maintains causality
- [ ] Tests verify event emission
- [ ] Documentation updated
