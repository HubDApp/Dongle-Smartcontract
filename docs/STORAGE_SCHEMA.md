# Storage Schema

## Project Object

| Field | Type | Description | Constraints |
|-------|------|-------------|-------------|
| `id` | `u64` | Unique project identifier | Auto-incremented |
| `owner` | `Address` | Project owner address | Must be valid Stellar address |
| `name` | `String` | Project display name | 1-100 characters |
| `slug` | `String` | URL-friendly identifier | 3-50 chars, alphanumeric + hyphens |
| `description` | `String` | Project description | 1-2000 characters |
| `category` | `String` | Project category | One of: DeFi, NFT, Infrastructure, Social, Gaming, Utility, Other |
| `website` | `Option<String>` | Project website URL | Optional, no validation enforced |
| `license` | `Option<String>` | License type | Optional |
| `logo_cid` | `Option<String>` | IPFS CID for logo | Optional |
| `metadata_cid` | `Option<String>` | IPFS CID for metadata | Optional |
| `tags` | `Option<Vec<String>>` | Search tags | Optional, max 5 tags |
| `social_links` | `Option<String>` | JSON-encoded social links | Optional |
| `launch_timestamp` | `Option<u64>` | Project launch time | Optional, Unix timestamp |
| `bounty_url` | `Option<String>` | Bug bounty URL or IPFS CID | Optional; must start with http:// or https:// (URL) or be a valid CID |
| `maintainers` | `Option<Vec<Address>>` | Maintainer addresses | Optional, max 10 |
| `archived` | `bool` | Whether project is archived | Default false |
| `created_at` | `u64` | Creation timestamp | Set on registration |
| `updated_at` | `u64` | Last update timestamp | Updated on changes |
| `verification_status` | `VerificationStatus` | Verification state | One of: Unverified, Pending, Verified, Rejected |
| `verified_at` | `Option<u64>` | Verification timestamp | Set when status becomes Verified |

### Bounty URL Validation

When a `bounty_url` is provided during project registration or update, the contract validates it as follows:
- If the string starts with `http://` or `https://`, it is treated as a URL and must have a minimum length of 10 characters.
- Otherwise, the string is considered an IPFS CID (e.g., `Qm...` or `bafy...`). In the current implementation, only HTTP URLs are accepted; CID validation is reserved for future release.

## Indexes

### OwnerProjects
- Key: `StorageKey::OwnerProjects(Address)`
- Value: `Vec<u64>` (list of project IDs)
- Max size: `MAX_PROJECTS_PER_USER` (50)

### SlugIndex
- Key: `SlugIndexKey { slug: String }`
- Value: `u64` (project ID)
- Used for slug uniqueness enforcement
