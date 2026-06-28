# Storage Schema

## Projects

### Fields

| Field | Type | Description |
|-------|------|-------------|
| ... (existing fields) | ... | ... |
| `bounty_url` | `String` (optional) | URL to bug bounty program |
| `bounty_cid` | `String` (optional) | IPFS CID of bug bounty policy document |

### Validation

- `bounty_url`: Must start with `http://` or `https://`.
- `bounty_cid`: Must be a valid IPFS CID (v0 starting with `Qm` or v1 starting with `baf`).
