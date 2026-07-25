# Dongle Bug Bounty Program

Responsible disclosure for the Dongle smart contract (Stellar / Soroban).

## Scope

**In scope**

- `dongle-smartcontract/` — on-chain contract logic (Rust / Soroban)
- Auth / access-control bypasses (owner, admin, maintainer)
- Incorrect state transitions (verification, claims, disputes, reviews)
- Unauthorized fund or fee diversion
- Storage corruption or denial-of-service that permanently bricks contract state
- Logic bugs that allow spoofing identity-critical project fields after verification

**Out of scope**

- Frontend / indexer / off-chain services (not in this repository)
- Issues that require a compromised admin key or social engineering of admins
- IPFS / CID content availability or off-chain data integrity
- Theoretical gas / fee griefing without a concrete permanent impact
- Reports without a clear reproduction path
- Issues already disclosed in [THREAT_MODEL.md](../THREAT_MODEL.md) as accepted residual risk

## How to report

1. Email **security@dongle.app** with a clear subject: `[Bug Bounty] <short title>`.
2. Include:
   - Affected function(s) / module path
   - Network / deployment context (local, testnet, mainnet) if relevant
   - Step-by-step reproduction
   - Expected vs actual behavior
   - Impact assessment (funds at risk, privilege escalation, state corruption)
   - Suggested fix if you have one
3. Do **not** open a public GitHub issue for unfixed vulnerabilities.

## Response & disclosure

| Stage | Target |
| --- | --- |
| Acknowledgement | Within 3 business days |
| Initial triage | Within 7 business days |
| Fix / mitigation plan | Coordinated with severity |

Please allow time for a fix before public disclosure. Coordinated disclosure is preferred.

## Severity guidance (informal)

- **Critical** — unauthorized fund movement, permanent takeover of verified project identity, or irreversible ledger corruption
- **High** — privilege escalation past owner/admin checks, invalid verification/status transitions with lasting impact
- **Medium** — incorrect access on non-critical paths, moderation/reporting bypass with limited blast radius
- **Low** — informational issues, missing input validation that returns a clean error instead of panicking

Rewards (if any) are determined case-by-case based on severity, quality of the report, and whether a practical exploit is demonstrated. This program may be updated without notice; check this file for the current policy.

## Safe harbor

Good-faith research conducted within this scope and reported privately will not be treated as a malicious attack. Do not access user data beyond what is needed to demonstrate the issue, and do not disrupt production services.
