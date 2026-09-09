# Admin Timelock

Sensitive admin actions can be **scheduled** to execute only after a
mandatory waiting period. The delay gives the other admins and the wider
community a guaranteed window to observe a pending change and react (rotate
keys, raise the alarm, cancel the action) before it takes effect.

Relevant source: `dongle-smartcontract/src/timelock_manager.rs`,
`constants::TIMELOCK_MIN_DELAY`, `constants::TIMELOCK_MAX_DELAY`.

## Scheduled action types

| Schedule call | Executes | Execute call |
| --- | --- | --- |
| `schedule_set_fee` | Fee config + treasury change | `execute_set_fee` |
| `schedule_add_admin` | Add an admin | `execute_add_admin` |
| `schedule_remove_admin` | Remove an admin | `execute_remove_admin` |

Each `schedule_*` call is admin-authenticated, returns an `action_id`, and
emits a `scheduled` event. A pending action can be cancelled by any admin
with `cancel_action` before it is executed.

## Delay bounds

`schedule_*` takes an absolute `execution_timestamp` (Unix seconds). It is
validated against the current ledger time `now`:

| Constant | Value | Rule |
| --- | --- | --- |
| — | — | `execution_timestamp > now` (must be in the future) |
| `TIMELOCK_MIN_DELAY` | `86_400` (1 day) | `execution_timestamp >= now + TIMELOCK_MIN_DELAY` |
| `TIMELOCK_MAX_DELAY` | `7_776_000` (90 days) | `execution_timestamp <= now + TIMELOCK_MAX_DELAY` |

Any violation returns `InvalidInput` (error code 36).

### Edge cases

| Requested delay | Result |
| --- | --- |
| `execution_timestamp` in the past | `InvalidInput` |
| `execution_timestamp == now` (zero delay) | `InvalidInput` — immediate execution is not allowed; it would defeat the timelock |
| `now + 1 second` … `now + 1 day − 1s` | `InvalidInput` — below the minimum |
| exactly `now + 1 day` | **OK** — minimum boundary is inclusive |
| `now + 30 days` | OK |
| exactly `now + 90 days` | **OK** — maximum boundary is inclusive |
| `now + 90 days + 1s` or more | `InvalidInput` — above the maximum |
| absurd values (e.g. `u64::MAX`) | `InvalidInput` (the `now + MAX_DELAY` bound is computed with `saturating_add`, so it never overflows) |

The bounds are checked **only at scheduling time**. Lowering them in a future
release does not retroactively invalidate already-scheduled actions; those
still execute at their stored timestamp once it passes (see `execute_*` /
`require_expired`).

## Execution

`execute_*` can be called by any admin once `now >= execution_timestamp`.
Before it runs it re-checks:

- the action is still pending (not executed, not cancelled),
- the action has expired (delay elapsed),
- the stored parameters are present.

`execute_add_admin` / `execute_remove_admin` / `execute_set_fee` then delegate
to the normal `AdminManager` / `FeeManager` mutators, so all of the usual
invariants (e.g. "cannot remove the last admin") still apply at execution
time.

## Operational limits for admins

- **Minimum 1 day.** Do not expect to push through an emergency change faster
  than 24 h via the timelock path. For genuine emergencies use the
  `emergency_pause` mechanism, then schedule the corrective action.
- **Maximum 90 days.** If you need a change to land further out, schedule it
  closer to the target date, or schedule now and plan to `cancel_action` +
  re-`schedule_*` if circumstances change. Do not treat the timelock queue as
  a long-term calendar.
- **Review the queue during incident response.** `list_scheduled_actions` /
  `get_scheduled_action_count` — every rotation and incident-response run
  should inventory pending actions and cancel anything unexpected.
- **Changing the constants** requires a contract upgrade (they are compile-time
  `const`s), not an admin transaction. Coordinate via `DEPLOYMENT.md`.

## Reading

- `get_action(action_id) -> Option<TimelockAction>`
- `list_scheduled_actions(start_index, limit) -> Vec<TimelockAction>`
- `get_scheduled_action_count() -> u64`
