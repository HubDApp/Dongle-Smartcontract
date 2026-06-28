# Storage Schema

## Project

| Field | Type | Description |
|-------|------|-------------|
| id | u64 | Unique project identifier |
| owner | Address | Project owner address |
| name | String | Project name |
| slug | String | Unique URL-friendly identifier |
| description | String | Project description |
| category | String | Project category |
| website | Option<String> | Project website URL |
| logo_cid | Option<String> | IPFS CID for project logo |
| metadata_cid | Option<String> | IPFS CID for project metadata |
| tags | Option<Vec<String>> | Tags for discovery |
| social_links | Option<Vec<String>> | Social media links |
| launch_timestamp | Option<u64> | Launch timestamp |
| bounty_url | Option<String> | Bug bounty program URL |
| bounty_cid | Option<String> | IPFS CID for bug bounty details |
| created_at | u64 | Creation timestamp |
| updated_at | u64 | Last update timestamp |
| archived | bool | Whether project is archived |
| maintainers | Option<Vec<Address>> | Additional maintainers |

## Indexes

### OwnerProjects(Address) → Vec<u64>

Maximum size: `MAX_PROJECTS_PER_USER` (50)
Enforced on registration, transfer, and claim.

### TODO: Add other indexes
