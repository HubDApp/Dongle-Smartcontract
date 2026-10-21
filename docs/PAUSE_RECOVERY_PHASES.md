# Pause Recovery Phases

Operational guide for the **staged resumption** of operations after an
emergency pause (issue #903).

Audience: contract admins and incident responders. Pair this with
[EMERGENCY_PAUSE_RECOVERY.md](EMERGENCY_PAUSE_RECOVERY.md) (the global pause
state machine and the post-unpause validation) and
[ADMIN_ROTATION_SCHEDULES.md](ADMIN_ROTATION_SCHEDULES.md) (key custody).

---

## 1. Why staged recovery?

The emergency pause is all-or-nothing: while `StorageKey::ContractPaused` is
set, every guarded mutating call is rejected. During an incident that is often
too coarse. A registry that is only serving reads is not useful, and a full
`unpause` after an incomplete investigation can re-open the very path that
caused the incident.

Staged recovery lets an operator bring the system back **one area at a time**,
validate that area, and roll back to the previous area if something still
misbehaves.

The implementation lives in `src/pause_recovery.rs` (`PauseRecovery`) and is
exposed through four contract functions:

| Function | Auth | Effect |
|---|---|---|
| `resume_operations_phase(admin, phase)` | admin | Re-enables a single recovery phase. Emits `PAUSE/PHASE`. |
| `rollback_recovery_phase(admin)` | admin | Halts the most recently resumed phase again. Emits `PAUSE/PHASE`. |
| `get_pause_recovery_state() -> PauseRecoveryState` | none (read) | Current plan, live phases, completed phases, last change. |
| `set_pause_recovery_plan(admin, phases)` | admin | Installs a custom recovery ladder. Emits `PAUSE/PHASE`. |

---

## 2. The recovery ladder

| Phase | Name | Re-enables |
|---|---|---|
| 0 | `FULL_PAUSE` | nothing — the contract is fully halted |
| 1 | `READ_OPS` | read-only traffic (reads are never gated by the pause) |
| 2 | `CORE_WRITES` | project register / update / archive / link / region / changelog |
| 3 | `REVIEWS` | review create / update / delete, review reports |
| 4 | `VERIFICATION` | verification request (and the verification admin paths) |
| 5 | `FEES` | fee payment / cancellation / refund |
| 6 | `ALL` | everything (equivalent to a full resume) |

The default plan is `1 → 2 → 3 → 4 → 5 → 6`. It extends the ladder named in
the issue (`read ops → reviews → verification → fees`) with an explicit
*core writes* step, because project registration and metadata updates are the
first write path an operator usually wants back.

Use `set_pause_recovery_plan` to install a different ladder — for example
`[1, 3, 4, 6]` to skip core writes entirely during a data-integrity incident.
The plan is only consulted while the contract is paused.

---

## 3. Non-cumulative semantics

Resuming a phase **only** re-enables that phase. Resuming `REVIEWS` does not
re-enable core writes, fee payments or verification requests.

This is deliberate: it is what "recover one area at a time" means. Each area
can therefore be validated in isolation before the next one is let through.

Two special cases:

* `ALL` is a wildcard — once phase 6 is resumed, every phase is live.
* `pause()` resets the recovery state: a new incident always starts from a
  full pause with no live phase.

---

## 4. Phase progress and rollback

Every phase change is pushed onto a stack:

* `resume_operations_phase` pushes the phase,
* `rollback_recovery_phase` pops the most recent phase and halts that area
  again,
* `get_pause_recovery_state` reports the **plan**, the **enabled phases**
  (the stack) and the **completed phases** (the phases of the plan the
  operator already moved past).

Rolling back is the "undo" for a bad resumption: e.g. if reviews are still
causing trouble, roll the phase back, keep the incident open, and fix the
underlying problem before resuming again.

---

## 5. Incident runbook

1. **Halt everything.**

   ```
   pause(admin)
   ```

   All recovery phases are reset. `get_pause_recovery_state()` now reports
   `is_paused = true` and an empty `enabled_phases` list.

2. **Investigate with reads only.**

   Read traffic keeps working during the pause, so the indexer, the front-end
   and the dashboards can be used to scope the incident.

3. **Bring the first area back.**

   ```
   resume_operations_phase(admin, 2)   // core writes
   ```

   Watch the `PAUSE/PHASE` event and the error rate of the resumed area.

4. **Validate, then continue with the next area.**

   ```
   resume_operations_phase(admin, 3)   // reviews
   resume_operations_phase(admin, 4)   // verification
   resume_operations_phase(admin, 5)   // fees
   ```

5. **Roll back if an area misbehaves.**

   ```
   rollback_recovery_phase(admin)
   ```

   The most recent phase is halted again; the previous areas stay live.

6. **Finish the incident.**

   ```
   unpause(admin)
   ```

   `unpause` clears the recovery state and restores the contract to full
   operation. Follow the post-unpause validation checklist in
   [EMERGENCY_PAUSE_RECOVERY.md](EMERGENCY_PAUSE_RECOVERY.md).

---

## 6. Events

Every phase change emits a `PAUSE/PHASE` event:

| Field | Meaning |
|---|---|
| `phase` | the phase that is now current (`0` = nothing live) |
| `previous_phase` | the phase that was current before the change |
| `enabled_phases` | all live phases |
| `completed_phases` | phases of the plan already moved past |
| `admin` | the operator that performed the change |
| `rolled_back` | `true` when the change was a rollback |

Indexers should treat `rolled_back = true` as "the area was halted again".

---

## 7. Error codes

| Error | Meaning |
|---|---|
| `ContractNotPaused` (157) | there is no incident to recover from — the contract is running |
| `InvalidPausePhase` (156) | the phase code is unknown or not part of the active plan |
| `NoPhaseToRollBack` (158) | no phase has been resumed yet, so there is nothing to roll back |
| `ContractPaused` (61) | the call belongs to a phase that is still halted |

---

## 8. Guard mapping

| Operation group | Guard |
|---|---|
| project register / update / archive / link / region / changelog | `PAUSE_PHASE_CORE_WRITES` |
| review create / update / delete, review reports | `PAUSE_PHASE_REVIEWS` |
| verification requests and verification admin paths | `PAUSE_PHASE_VERIFICATION` |
| fee payment / cancellation / refund | `PAUSE_PHASE_FEES` |
| everything else that mutates state | `PAUSE_PHASE_CORE_WRITES` |
