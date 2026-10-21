# Admin Rotation Schedules

How the automatic admin-key rotation of issue #900 works: how the schedule is
installed, what the pre-flight checklist covers, how a rotation is executed and
how the affected key holders are notified.

Audience: contract admins and key custodians. Pair this with
[ADMIN_ROTATION_PLAYBOOK.md](ADMIN_ROTATION_PLAYBOOK.md) (the manual key
rotation procedure) and [PAUSE_RECOVERY_PHASES.md](PAUSE_RECOVERY_PHASES.md)
(staged incident recovery).

---

## 1. Why rotate admin keys?

A long-lived admin key is a standing risk: a leaked key stays valid until
somebody notices. The rotation schedule makes key hygiene enforceable
on-chain: the contract itself removes the oldest admin key and promotes a
replacement once the configured interval has elapsed.

---

## 2. Contract functions

| Function | Auth | Effect |
|---|---|---|
| `set_rotation_schedule(admin, interval_days, replacement_pool)` | admin | Installs (or replaces) the schedule. |
| `get_rotation_schedule() -> Option<AdminRotationSchedule>` | none (read) | The active schedule. |
| `get_rotation_preflight_checklist(admin) -> Vec<RotationChecklistItem>` | admin | The checklist that must pass before a rotation. |
| `confirm_rotation_preflight(admin) -> AdminRotationSchedule` | admin | Confirms the checklist. |
| `execute_scheduled_rotation(caller) -> AdminRotationRecord` | any caller | Runs the rotation (once due and confirmed). |
| `get_admin_rotation_record(rotation_id)` | none (read) | One rotation record. |
| `list_admin_rotation_records(start_index, limit)` | none (read) | All rotation records. |
| `get_admin_rotation_notifications(rotation_id)` | none (read) | The notifications of one rotation. |

---

## 3. The schedule

`AdminRotationSchedule` holds:

| Field | Meaning |
|---|---|
| `enabled` | whether the schedule is armed |
| `interval_days` | the rotation interval (30, 60 or 90 days) |
| `next_rotation_at` | when the next rotation becomes due |
| `last_rotation_at` | when the last rotation ran |
| `replacement_pool` | addresses that may be promoted into the admin set |
| `preflight_confirmed` | whether an admin confirmed the checklist |
| `preflight_confirmed_at` | when the checklist was confirmed |
| `rotation_count` | number of rotations executed so far |

Supported intervals are **30, 60 and 90 days**; any other value is rejected
with `InvalidRotationInterval` (160). The pool holds at most
`MAX_ROTATION_POOL_SIZE` (10) distinct addresses, none of which may already
be an admin.

Installing a schedule back-fills the join timestamp of admins that were added
before the schedule existed, so installing a schedule never rotates an admin
out immediately.

---

## 4. The pre-flight checklist

Before a rotation runs, an admin works through the checklist and confirms it
on-chain:

| Check | Meaning |
|---|---|
| `schedule_configured` | a rotation schedule exists |
| `interval_supported` | the interval is 30, 60 or 90 days |
| `replacement_pool_not_empty` | at least one replacement candidate is available |
| `replacement_candidates_valid` | the candidates are distinct and none is a current admin |
| `more_than_one_admin` | more than one admin is registered |
| `rotation_due` | the scheduled rotation is actually due |

`confirm_rotation_preflight` fails with
`RotationPreflightChecklistIncomplete` (165) while any item is unsatisfied.

The confirmation is **single-use**: it is cleared again as soon as a rotation
executes, so the next rotation needs a fresh confirmation.

---

## 5. Executing a rotation

`execute_scheduled_rotation(caller)` can be called by any authenticated keeper
(an operator, a cron job, a front-end bot) once:

1. a schedule exists and is enabled,
2. `now >= next_rotation_at` (otherwise `RotationNotDue`), and
3. the pre-flight checklist was confirmed (otherwise
   `RotationPreflightNotConfirmed`).

The rotation then:

1. picks the admin that joined the **longest ago** and removes it from the
   admin set (never the last admin — otherwise
   `RotationWouldRemoveLastAdmin`),
2. promotes the first candidate of the replacement pool that is not an admin
   yet,
3. records a notification for **every** affected key holder (removed and
   added),
4. stores an `AdminRotationRecord`,
5. re-arms the schedule for the next interval and clears the pre-flight
   confirmation.

Events: `ADMINROT/SCHEDULED`, `ADMINROT/PREFLIGHT`, `ADMINROT/EXECUTED` and
one `ADMINROT/NOTIFY` per affected admin (`removed = true|false`).

Off-chain operators subscribe to `ADMINROT/NOTIFY` (or poll
`get_admin_rotation_notifications`) to warn the incoming and outgoing key
holders.

---

## 6. Admin join tracking

The rotation needs to know which key is the oldest. Every admin addition
records the join timestamp in `AdminRotationKey::AdminJoinedAt(address)`:

* `initialize()` — the first admin,
* `add_admin()` — a directly added admin,
* `execute_proposal()` — an admin added through the multi-sig proposal flow,
* `approve_emergency_recovery()` — an admin added through emergency recovery.

---

## 7. Error codes

| Error | Meaning |
|---|---|
| `RotationScheduleNotSet` (159) | no schedule is configured (or it is disabled) |
| `InvalidRotationInterval` (160) | the interval is not 30, 60 or 90 days |
| `RotationNotDue` (161) | the scheduled rotation is not due yet |
| `RotationPreflightNotConfirmed` (162) | the checklist has not been confirmed |
| `NoAdminsDueForRotation` (163) | no admin is eligible for rotation |
| `RotationWouldRemoveLastAdmin` (164) | the rotation would brick the contract |
| `RotationPreflightChecklistIncomplete` (165) | at least one checklist item fails |
