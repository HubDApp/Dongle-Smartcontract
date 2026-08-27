# Treasury Rotation Playbook

This playbook documents the rules, architecture, and step-by-step operations for updating and rotating the Treasury address in the Dongle Smart Contract.

---

## 1. Overview & Architecture

The Treasury address is the designated Soroban address that receives project verification fees and registration fees when owners pay through `pay_fee` or `pay_registration_fee`.

### Security Controls
- **Admin Authorization Required**: Only authorized contract admins can rotate or update the treasury address.
- **Multisig Support**: When the admin threshold is greater than 1 (`admin_approval_threshold > 1`), direct updates via `set_treasury` or `set_fee` are blocked. Instead, treasury updates must be executed via the admin multisig proposal flow (`ProposalPayload::SetTreasury` or `ProposalPayload::SetFee`).
- **Timelock Support**: Treasury changes can be scheduled through the timelock mechanism (`schedule_set_fee`) with a enforced minimum delay before execution (`TIMELOCK_MIN_DELAY`).

---

## 2. Emitted Audit Events

Whenever a treasury address is rotated or updated (either standalone via `set_treasury` or as part of a fee reconfiguration via `set_fee`), the contract emits a dedicated audit event.

### Event Definition
- **Topic:** `(Symbol("CONFIG"), Symbol("TREASURY"))`
- **Data Payload (`TreasuryUpdatedEvent`):**
  - `admin` (`Address`): Admin address who initiated the change.
  - `old_treasury` (`Option<Address>`): Previous treasury address (if set).
  - `new_treasury` (`Address`): Updated treasury address.
  - `timestamp` (`u64`): Ledger Unix timestamp of the update.

---

## 3. Operations & Playbook Steps

### Method A: Single-Admin Direct Treasury Rotation
If the contract is in single-admin mode (`threshold == 1`):
1. Call `set_treasury(admin, new_treasury_address)`.
2. The contract verifies admin authorization.
3. The old treasury is retrieved and replaced with `new_treasury_address`.
4. The `TreasuryUpdatedEvent` is emitted on topic `("CONFIG", "TREASURY")`.
5. An entry is recorded in the `AdminActionLog` with action type `AdminActionType::TreasuryUpdated`.

### Method B: Multisig Admin Proposal Rotation
If the contract operates under a threshold higher than 1:
1. An admin submits a proposal with payload `ProposalPayload::SetTreasury(new_treasury_address)`.
2. Co-admins approve the proposal until the threshold is met.
3. Call `execute_proposal(caller, proposal_id)`.
4. The proposal payload executes, updating storage, emitting `TreasuryUpdatedEvent`, and logging the action.

---

## 4. Off-Chain Indexing & Audit Verification

Off-chain monitoring scripts and indexers should subscribe to the event topic `("CONFIG", "TREASURY")`.

Example log verification:
```json
{
  "topics": ["CONFIG", "TREASURY"],
  "data": {
    "admin": "GADMIN...XXXX",
    "old_treasury": "GOLD...XXXX",
    "new_treasury": "GNEW...XXXX",
    "timestamp": 1782391000
  }
}
```
