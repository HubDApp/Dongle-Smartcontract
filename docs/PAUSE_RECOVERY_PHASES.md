# Pause Recovery Phases — staged resumption (issue #904)

When an emergency pause is lifted it is rarely safe to flip everything back on
at once: the area that caused the incident is usually not the area that
breaks first when traffic returns. The contract therefore supports a **staged
recovery**: an operator walks the platform back to life one area at a time and
can roll back a phase if the next area misbehaves.

## The recovery plan

| Phase | Resumed | Pause state after the transition | Generic mutations (`All`) |
|-------|---------|----------------------------------|---------------------------|
| 0 | nothing (fully paused) | `PausedAll` | blocked |
| 1 | read operations | `PausedAll` | blocked |
| 2 | reviews | `PausedVerificationAndFees` | blocked |
| 3 | verification | `PausedFees` | blocked |
| 4 | fees — the contract is fully running again | `Running` | allowed |

Read operations are available in every phase, so users can always see what is
going on while the recovery is in progress.

## API

| Endpoint | Purpose |
|----------|---------|
| `resume_operations_phase(admin, phase)` | Move to `phase` (0..=4). Emits `CONTRACT/PHASE` with the new and the previous phase. |
| `rollback_operations_phase(admin)` | Move back one phase. Fails with `InvalidPhaseRollback` when no phase has been resumed yet. |
| `get_resume_phase()` | Current phase (0 = no staged recovery in progress). |
| `get_phase_history()` | Ordered list of the phase transitions (capped at 50 entries). |
| `get_contract_status()` | Full status, including the current phase and the pause reason. |

`resume_operations_phase` accepts any phase in `0..=MAX_RESUME_PHASE`
(`MAX_RESUME_PHASE = 4`). Passing a **lower** phase number than the current
one is a rollback, so an operator can move forward and backward with a single
endpoint; `rollback_operations_phase` is the explicit, safer form of the same
operation.

An unknown phase (> 4) is rejected with `InvalidPausePhase` and leaves the
state untouched.

## Operator guide

### 1. Declare the incident

```json
{
  "function": "set_pause_with_reason",
  "args": ["<admin>", "Fee token migration – treasury balances inconsistent", 1800000000]
}
```

* The reason is a short operator note (1..=256 bytes).
* `estimated_resume` is the expected resume time as a **Unix timestamp**
  (`0` means "no estimate").  It is shown by `get_contract_status()` and
  included in the `CONTRACT/PAUSED` event.
* The reason is stored on the pause record and pushed to every address that
  subscribed to pause notifications (`subscribe_pause_notifications`).

Prefer a *partial* pause when only one area is affected:

```json
{ "function": "set_pause_state", "args": ["<admin>", "PausedVerification", "Verification queue overflow", 0] }
```

### 2. Confirm the blast radius

```json
{ "function": "get_contract_status" }
```

Returns the pause state, the reason, the estimated resume time, the pause
timestamp, the current recovery phase and the list of blocked operation
families.  The same information is emitted as `CONTRACT/NOTIFY` to the
subscribers.

### 3. Recover one area at a time

```json
{ "function": "resume_operations_phase", "args": ["<admin>", 1] }   // reads confirmed
{ "function": "resume_operations_phase", "args": ["<admin>", 2] }   // reviews back on
{ "function": "resume_operations_phase", "args": ["<admin>", 3] }   // verification back on
{ "function": "resume_operations_phase", "args": ["<admin>", 4] }   // fees back on, fully running
```

After **every** transition:

* the new phase is stored on the pause record (`get_resume_phase()`),
* the transition is appended to the phase history (`get_phase_history()`),
* a `CONTRACT/PHASE` event is emitted with the new phase and the previous
  phase,
* the final phase (`4`) also emits `CONTRACT/UNPAUSED` and clears the pause
  reason.

Between two phases, watch the metrics of the area that was just resumed.  If
something looks wrong, roll back immediately:

```json
{ "function": "rollback_operations_phase", "args": ["<admin>"] }
```

### 4. Close the incident

```json
{ "function": "unpause", "args": ["<admin>"] }
```

`unpause` resets the phase to `0`, clears the reason and the estimated resume
time and emits `CONTRACT/UNPAUSED`.

## Checklist

- [ ] The reason names the failing subsystem and the incident ticket.
- [ ] The subscriber list contains the on-call operators.
- [ ] `get_contract_status()` matches the expected state before every step.
- [ ] Each phase is confirmed with the metrics of the resumed area before the
      next phase is started.
- [ ] A rollback was rehearsed (or at least is one transaction away).
- [ ] The phase history is archived with the incident report.

## Failure modes

| Symptom | Cause | Action |
|---------|-------|--------|
| `InvalidPausePhase` | `phase > 4` | Use a phase between 0 and 4. |
| `InvalidPhaseRollback` | No phase has been resumed yet | Resume a phase first, or use `unpause`. |
| `ContractPaused` after a phase change | The call belongs to a family that is still closed | Check `get_contract_status().paused_operations`. |
| `ContractPaused` on a generic mutation | The recovery has not reached phase 4 | Finish the recovery plan or accept the restriction. |

## See also

* [PAUSE_MATRIX.md](PAUSE_MATRIX.md) — which operation families are blocked by
  which pause state.
* [EMERGENCY_PAUSE_RECOVERY.md](EMERGENCY_PAUSE_RECOVERY.md) — the general
  emergency-pause runbook.
