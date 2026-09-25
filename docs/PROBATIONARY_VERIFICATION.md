# Probationary Verification Status

This document describes the design, storage model, and API for the **probationary verification status** feature in the Dongle Smart Contract.

---

## 1. Overview

When a project's verification request is approved by an administrator, it enters an initial **30-day probationary period** under **enhanced monitoring**. During this period:

1. **30-Day Probationary Window**: The project is subject to an active 30-day probation duration (`2,592,000` seconds).
2. **Enhanced Monitoring**: Auditing tools, indexers, and community observers can query active probationary projects. Incidents, flags, and anomalies are tracked with lower reporting thresholds.
3. **Auto-Promotion**: After 30 days elapse without revocation, the project is automatically promoted to full `Verified` status without requiring manual administrative re-approval.
4. **Fast-Track Revocation**: If discrepancies or security issues arise during probation, administrators can immediately revoke verification via `revoke_during_probation` without waiting for a lengthy dispute resolution, appeal, or full review workflow.

```text
               ┌───────────────────────┐
               │   Pending Approval    │
               └───────────┬───────────┘
                           │ approve_verification
                           ▼
               ┌───────────────────────┐
               │     Probationary      │
               │  (30-day monitoring)  │
               └─────┬───────────┬─────┘
                     │           │
   30 days elapse    │           │ revoke_during_probation
   (no revocation)   │           │ (without full review)
                     ▼           ▼
        ┌──────────────────┐  ┌──────────────────┐
        │ Verified (Full)  │  │    Unverified    │
        └──────────────────┘  └──────────────────┘
```

---

## 2. Acceptance Criteria Fulfillment

| Acceptance Criterion | Mechanism | Contract Method |
|---|---|---|
| **30-day probationary period after approval** | Approval initializes a `ProbationRecord` with `started_at = now` and `probation_until = now + 30 days` (`2,592,000` seconds). | `approve_verification`<br>`get_probation_record`<br>`is_in_probation` |
| **Enhanced monitoring during probation** | Sets `enhanced_monitoring = true`, indexes active probationary projects, and records incident/anomaly reports. | `list_probationary_projects`<br>`record_probation_incident`<br>`get_probation_incident_count` |
| **Auto-promote to full verification after period** | Once `ledger_time >= probation_until`, the project is automatically promoted out of probation. `is_in_probation` resolves to `false` and `check_and_promote_probation` finalizes storage state. | `check_and_promote_probation`<br>`is_in_probation` |
| **Can revoke during probation without full review** | Administrators can invoke fast-track revocation directly, bypassing dispute delays or full review requirements. | `revoke_during_probation` |

---

## 3. Data Model

### `VerificationStatus`
`types.rs` includes the `Probationary` variant:
```rust
pub enum VerificationStatus {
    Unverified,
    Pending,
    Verified,
    Suspended,
    Rejected,
    Probationary,
}
```

### `ProbationRecord`
Stored under `ProbationKey::ProjectProbation(project_id)`:
```rust
pub struct ProbationRecord {
    pub project_id: u64,
    pub request_id: u64,
    pub approved_by: Address,
    pub started_at: u64,
    pub probation_until: u64,        // started_at + 30 days
    pub is_promoted: bool,
    pub is_revoked: bool,
    pub enhanced_monitoring: bool,
    pub incident_count: u32,
}
```

### `ProbationIncident`
Stored under `ProbationKey::ProbationIncident(project_id, incident_id)`:
```rust
pub struct ProbationIncident {
    pub incident_id: u32,
    pub project_id: u64,
    pub reporter: Address,
    pub details: String,
    pub recorded_at: u64,
}
```

---

## 4. Public API Reference

| Entrypoint | Access | Description |
|---|---|---|
| `is_in_probation(project_id)` | Public | Returns `true` if the project is currently within its 30-day probation window. |
| `get_probation_record(project_id)` | Public | Returns the `ProbationRecord` containing probation timestamps, promotion status, and incident count. |
| `get_effective_verification(project_id)` | Public | Returns `Probationary` if within active probation, or `Verified` if promoted / fully verified. |
| `check_and_promote_probation(project_id)` | Public | Finalizes auto-promotion once 30 days have elapsed. Emits `ProbationAutoPromotedEvent`. |
| `revoke_during_probation(admin, project_id, reason)` | Admin-only | Immediately revokes verification during probation without requiring a full review or dispute period. |
| `record_probation_incident(reporter, project_id, details)` | Public / Monitor | Records an incident or anomaly against a probationary project, incrementing its incident counter. |
| `get_probation_incident_count(project_id)` | Public | Returns the number of incidents recorded during the project's probationary period. |
| `list_probationary_projects(start, limit)` | Public | Paginated list of project IDs currently subject to active probation and enhanced monitoring. |
| `get_probation_duration()` | Public | Returns the configured probationary duration in seconds (default: 30 days / `2,592,000`s). |
| `set_probation_duration(admin, duration_secs)` | Admin-only | Updates the default probationary duration. |

---

## 5. Events

The module emits dedicated Soroban events under the `"PROBATION"` symbol:

- `("PROBATION", "STARTED", project_id)`: Emitted upon verification approval. Carries `ProbationStartedEvent` with `request_id`, `approved_by`, `started_at`, and `probation_until`.
- `("PROBATION", "PROMOTED", project_id)`: Emitted upon auto-promotion after the 30-day period. Carries `ProbationAutoPromotedEvent`.
- `("PROBATION", "REVOKED", project_id)`: Emitted when an admin revokes verification during probation without full review. Carries `ProbationRevokedEvent`.
- `("PROBATION", "INCIDENT", project_id)`: Emitted when an incident is recorded during enhanced monitoring. Carries `ProbationIncidentEvent`.
