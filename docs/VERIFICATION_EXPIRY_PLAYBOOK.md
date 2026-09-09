# Verification Expiry Operational Playbook (#624)

This playbook outlines the operational procedures for identifying, monitoring, and processing expired verification records on the Dongle Smartcontract platform.

---

## Architecture & Design Decision

Soroban smart contracts operate deterministically during transaction execution and do not run on-chain background cron jobs or implicit timers.

To manage verification expiry cleanly:
1. **Read-Time Automatic Evaluation**: Read query entry points (`is_verification_active`, `is_verification_expired`) evaluate the current ledger timestamp (`env.ledger().timestamp()`) against `expires_at`. An expired record (`now >= expires_at`) is immediately treated as invalid/inactive on read paths without requiring mandatory persistent writes.
2. **Explicit State Transition Entry Point**: The contract exposes `process_verification_expiry(project_id)`. Calling this function updates persistent storage (`VerificationRecord.status` -> `Unverified`, `Project.verification_status` -> `Unverified`) and publishes a `VerificationExpired` event.
3. **Auto-Process on State-Mutating Entry Points**: Mutating operations (e.g. `request_verification`) automatically trigger expiry resolution if the project's verification has elapsed.

---

## Operator Procedure

### 1. Identifying Expired Verifications

Operators or automated off-chain indexers can identify expired verifications by:
- Listening for `VerificationExpired` events published when `is_verification_active` or `process_verification_expiry` detects an expired verification.
- Querying `is_verification_expired(project_id)` for registered projects.
- Scanning verification records where `expires_at != 0` and `ledger_timestamp >= expires_at`.

### 2. Contract Operation to Call

To persist the status transition for an expired project, call the contract entry point:

```rust
pub fn process_verification_expiry(env: Env, project_id: u64) -> Result<bool, ContractError>
```

- **Arguments**: `project_id: u64`
- **Return Value**:
  - `Ok(true)`: Expiry was processed and storage state transitioned to `Unverified`.
  - `Ok(false)`: Verification is not expired, has no expiry set (`expires_at == 0`), or was not in `Verified` status.

### 3. Permissions & Execution

- `process_verification_expiry` is permissionless. It can be executed by any automated worker node, operator script, admin, or project owner.
- The transaction relies on deterministic ledger time (`env.ledger().timestamp()`) and requires no administrative key signatures.

### 4. Verifying Resulting State

After invoking `process_verification_expiry(project_id)`:
1. Query `get_project(project_id)`. Ensure `verification_status == VerificationStatus::Unverified`.
2. Query `get_verification(project_id)`. Ensure `status == VerificationStatus::Unverified`.
3. Query `is_verification_active(project_id)`. Ensure it returns `false`.

### 5. Failure & Edge Case Handling

- **Project Not Found (`ContractError::ProjectNotFound`)**: Verify the `project_id`. The project may not have been registered yet.
- **Verification Not Found (`ContractError::VerificationNotFound`)**: The project has no verification request on record.
- **Idempotency**: Executing `process_verification_expiry` multiple times or on an already unverified project returns `Ok(false)` cleanly without error or extra side effects.
- **Transaction Interruption / Reversion**: If a transaction fails (e.g. out of gas), Soroban atomicity guarantees that no partial storage updates persist. Operators can safely retry.

### 6. Handling Large Numbers of Expired Records (Batch Processing)

For bulk processing across thousands of projects:
- Workers can scan project IDs in pages via off-chain indexers.
- For each expired project, invoke `process_verification_expiry(project_id)` in parallel or batched client invocations.
- Re-run batch passes periodically as ledger timestamps advance.
