# Admin Role-Based Access Control (RBAC) & Migration Plan

## 1. Overview & Objectives

Historically, the Dongle Smart Contract operated on a single binary admin model: any address listed in `StorageKey::AdminList` held full operational authority over all contract capabilities. As governance matured and compliance requirements expanded, least-privilege access control became necessary.

This upgrade introduces:
1. **Granular Permissions** (`AdminPermission`) to scope administrative authority.
2. **Role Definitions** (`AdminRole`) combining permission sets.
3. **Multi-Admin Approval Workflows** (`ApprovalWorkflow`) for sensitive operations.
4. **Emergency Timelock Delay Bypass** with justification CID auditing.
5. **Comprehensive Action Logging & Auditing** with suspicious activity detection.

---

## 2. Admin Permission & Role Architecture

### Granular Permissions (`AdminPermission`)

| Permission | Ordinal | Allowed Capabilities |
| :--- | :--- | :--- |
| `ApproveVerification` | 1 | Approve, reject, revoke, suspend, and restore project verification statuses; assign verification workloads. |
| `ManageFees` | 2 | Configure verification/registration fees, update treasury address, process fee refunds. |
| `ManageCollections` | 3 | Create, edit, delete, and curate official and admin collections. |
| `ManageAdmins` | 4 | Add admins, assign roles, remove admins, set governance thresholds, configure approval workflows. |
| `EmergencyPause` | 5 | Halt (`pause`) and resume (`unpause`) contract operations; propose/vote on emergency timelock bypasses. |

### Standard Admin Roles (`AdminRole`)

```rust
pub enum AdminRole {
    /// Holds all 5 permissions (Superuser).
    SuperAdmin,
    /// Holds `ApproveVerification` only.
    VerificationAdmin,
    /// Holds `ManageFees` only.
    FeeAdmin,
    /// Holds `ManageCollections` only.
    CollectionAdmin,
    /// Holds `EmergencyPause` only.
    EmergencyAdmin,
    /// Explicit set of custom permissions.
    Custom(Vec<AdminPermission>),
}
```

---

## 3. Migration Plan for Existing Admins

### Phase 1: Zero-Downtime Backward-Compatible Fallback
- Existing contracts deployed prior to this upgrade contain admin addresses in `StorageKey::AdminList` without corresponding `AdminSecurityKey::AdminRole` entries.
- The contract access control layer checks:
  ```rust
  pub fn get_admin_role(env: &Env, address: &Address) -> Option<AdminRole> {
      if !Self::is_admin(env, address) {
          return None;
      }
      Some(
          env.storage()
              .persistent()
              .get(&AdminSecurityKey::AdminRole(address.clone()))
              .unwrap_or(AdminRole::SuperAdmin),
      )
  }
  ```
- Any admin lacking an explicit role record automatically evaluates to `AdminRole::SuperAdmin`. No existing admin functionality is interrupted upon contract binary upgrade.

### Phase 2: On-Chain Batch Migration
A dedicated idempotent migration function is provided:
```rust
migrate_existing_admins(caller: Address) -> Result<u32, ContractError>
```
- **Caller requirement**: Must hold `AdminPermission::ManageAdmins`.
- **Execution**: Iterates over all addresses in `StorageKey::AdminList`. For any address missing an `AdminSecurityKey::AdminRole` record, sets `AdminRole::SuperAdmin` and publishes `AdminRoleAssignedEvent`.
- **Audit Logging**: Emits an `AdminActionLog` record with `AdminActionType::AdminRoleSet`.

### Phase 3: Least-Privilege Reassignment
Once migration is complete, governance operators can reassign admins to specialized roles using `add_admin_with_role(caller, admin_address, new_role)`.

Example reassignment:
- Security operations keys: assigned `AdminRole::EmergencyAdmin`.
- Verification auditors: assigned `AdminRole::VerificationAdmin`.
- Finance operators: assigned `AdminRole::FeeAdmin`.
- Operations leads: assigned `AdminRole::CollectionAdmin`.
- Multi-sig quorum: retains `AdminRole::SuperAdmin`.

---

## 4. Multi-Admin Approval Workflows for Sensitive Actions

Critical actions can no longer be executed unilaterally when an approval workflow is active.

### Sensitive Action Types
- `ConfigChange`: System parameters and governance configurations.
- `LargeFeeAdjustment`: Significant modifications to registration/verification fee schedules.
- `AdminRemoval`: Revoking admin rights.
- `EmergencyPause`: Toggling global contract pause state.

### Quorum & Staging
Workflows specify:
- `required_quorum`: Minimum admin votes needed (e.g. 3 of 5).
- `total_stages`: Multi-stage approval support (e.g., stage 1 vote, stage 2 timelock/cooldown).
- `stage_delay`: Cooldown delay in seconds before execution is unlocked.

---

## 5. Emergency Timelock Delay Bypass Runbook

When a scheduled timelock action (such as critical security patch or emergency fee adjustment) requires immediate execution:
1. An admin with `EmergencyPause` permission invokes `propose_emergency_bypass(timelock_id, justification_cid)`.
2. The justification IPFS CID document must contain the incident report, CVE details, and root cause analysis.
3. Admins vote via `approve_emergency_bypass(bypass_id)`.
4. When approvals reach the configured `emergency_bypass_threshold` (supermajority, e.g. 5 of 7):
   - Proposal status is set to approved.
   - Any admin calls `execute_emergency_bypass(bypass_id)`.
   - The timelocked action is immediately unlocked and executable without waiting for `execution_timestamp`.
   - The entire bypass lifecycle is recorded in `AdminActionLog` and on-chain events.
