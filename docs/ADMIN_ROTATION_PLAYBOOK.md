# Administrator Key Rotation Playbook

Operational guide for rotating Dongle smart contract administrator keys on Stellar/Soroban deployments.

## 1. Rotation Overview

Dongle stores admins in on-chain storage (`AdminList`, `Admin(address)`). Privileged actions require `Address::require_auth()` from an admin account. Rotation means **adding a new admin with the new key, verifying behavior, then removing the old admin** — never the reverse.

Goals:

- Maintain uninterrupted moderation and configuration capability
- Avoid a window with zero admins
- Produce an auditable trail via `AdminActionLog` and admin events

## 2. Threat Model

| Threat | Impact | Mitigation |
|--------|--------|------------|
| Compromised admin key | Unauthorized fee changes, verification abuse, review deletion | Multisig threshold, timelocks, rapid rotation |
| Lost admin key | Contract becomes ungovernable if last admin | Maintain ≥2 admins; secure offline backups |
| Insider collusion | Threshold admins bypass controls | Separate key custody, monitoring, incident runbooks |
| Rotation mistake (remove before add) | Zero admins; irreversible without upgrade | Always add-before-remove checklist |

See also [THREAT_MODEL.md](../THREAT_MODEL.md) for broader contract assumptions.

## 3. Safe Rotation Procedure

### Prerequisites

- Soroban CLI / Stellar CLI configured for target network
- Contract ID and current admin list (`get_admin_list`)
- New admin key generated on hardware wallet or HSM
- Maintenance window communicated to operators

### Steps

1. **Inventory** — Record current admins, approval threshold, and pending timelock/proposal actions.
2. **Generate new key** — Create the replacement admin keypair; store seed material offline.
3. **Add new admin** (existing admin signs):
   ```bash
   stellar contract invoke \
     --id <CONTRACT_ID> \
     --source-account <CURRENT_ADMIN> \
     -- add_admin \
     --caller <CURRENT_ADMIN> \
     --new_admin <NEW_ADMIN>
   ```
4. **Verify addition** — Confirm `is_admin(new_admin) == true` and admin count increased.
5. **Testnet validation** — Run the checklist in Section 6 on testnet before mainnet.
6. **Operational smoke test** — With the new key, perform a non-destructive admin read (`get_admin_list`) and a reversible action on testnet (e.g. add/remove a test reserved name).
7. **Remove old admin** (new or remaining admin signs):
   ```bash
   stellar contract invoke \
     --id <CONTRACT_ID> \
     --source-account <NEW_ADMIN> \
     -- remove_admin \
     --caller <NEW_ADMIN> \
     --admin_to_remove <OLD_ADMIN>
   ```
8. **Final verification** — Old key must fail `is_admin`; new key must succeed; at least one admin remains.

### Multisig deployments

When `get_admin_approval_threshold() > 1`, use the admin proposal flow instead of direct `add_admin` / `remove_admin`. Collect threshold approvals before execution.

## 4. Rollback Procedure

If the new admin misbehaves or was misconfigured **before old admin removal**:

1. Do **not** remove the old admin.
2. Remove the faulty new admin with a trusted remaining admin.
3. Investigate root cause; restart rotation from Step 3.

If the old admin was already removed and the new admin is lost:

- There is **no on-chain recovery** without a remaining admin or contract upgrade.
- Escalate to incident response (Section 8).

## 5. Verification Steps

After every rotation:

| Check | Command / method | Expected |
|-------|------------------|----------|
| Admin count | `get_admin_count` | ≥ 1, matches expectation |
| New admin active | `is_admin(new_admin)` | `true` |
| Old admin revoked | `is_admin(old_admin)` | `false` |
| Event log | Index `AdminAdded` / `AdminRemoved` events | Matches rotation timeline |
| Action log | `list_admin_actions` (if indexed) | Records admin changes |

## 6. Testnet Validation Checklist

Run on testnet before mainnet rotation:

- [ ] Deploy or locate testnet contract ID
- [ ] Fund current and new admin accounts
- [ ] `add_admin` succeeds and emits event
- [ ] New admin can call a gated read-only admin endpoint
- [ ] New admin can execute one reversible admin action
- [ ] `remove_admin` on old key succeeds
- [ ] `CannotRemoveLastAdmin` triggers when attempting to remove sole admin
- [ ] Multisig/timelock paths tested if enabled on deployment
- [ ] Document transaction hashes and ledger sequence numbers

## 7. Recommended Operational Practices

- Maintain **at least two** independent admins in production
- Use hardware wallets for admin keys; never commit secrets to CI
- Rotate on a **scheduled cadence** (e.g. quarterly) and after personnel changes
- Monitor admin events via indexer alerts
- Prefer timelocked fee/config changes on mainnet
- Keep this playbook and contract ID in your internal ops wiki

## 8. Incident Response — Compromised Key

**Assume active attacker if a admin private key may be exposed.**

### Immediate (0–15 minutes)

1. Identify compromised address in `get_admin_list`
2. If another admin is available: **add a clean emergency admin** from uncompromised key
3. If multisig: submit emergency proposal to remove compromised admin
4. Notify team; preserve ledger/event evidence

### Short-term (15–60 minutes)

5. Remove compromised admin once replacement is active
6. Review `AdminActionLog` and recent transactions for unauthorized:
   - Fee changes
   - Verification approvals/revocations
   - Review moderation (`hide_review`, `admin_delete_review`)
   - Treasury or timelock executions
7. Revoke off-chain API keys tied to the compromised identity

### Recovery

8. Re-run Section 6 checklist on testnet with new keys
9. Publish internal post-incident summary with timeline and ledger hashes
10. Update custody procedures; schedule follow-up rotation

### If all admins compromised

- Pause dependent frontends that rely on admin-curated data
- Coordinate contract upgrade or migration with stakeholders
- Do **not** attempt ad-hoc mainnet experiments without testnet proof

## 9. Emergency Contacts Template

| Role | Name | Contact | Notes |
|------|------|---------|-------|
| Primary admin custodian | | | |
| Backup admin custodian | | | |
| Security lead | | | |
| Indexer/on-call | | | |

---

**Related docs:** [INITIALIZATION_DEPLOYMENT_CHECKLIST.md](./INITIALIZATION_DEPLOYMENT_CHECKLIST.md), [THREAT_MODEL.md](../THREAT_MODEL.md), [CONTRACT_INTERFACE.md](../CONTRACT_INTERFACE.md)
