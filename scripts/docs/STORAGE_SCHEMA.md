# Storage Schema

## Project Entry

A project is stored under the key `Project(u64)` and contains the following fields:

| Field | Type | Description | Optional |
|-------|------|-------------|----------|
| `owner` | `Address` | The project owner | No |
| `name` | `String` | Project name (max 64 chars) | No |
| `slug` | `String` | URL-friendly identifier (unique, max 64 chars) | No |
| `description` | `String` | Project description (max 1024 chars) | No |
| `category` | `String` | Project category (e.g., "DeFi", "NFT") | No |
| `tags` | `Vec<String>` | Associated tags (max 10) | Yes |
| `website` | `String` | Project website URL | Yes |
| `logo_cid` | `String` | IPFS CID for project logo | Yes |
| `metadata_cid` | `String` | IPFS CID for additional metadata | Yes |
| `social_links` | `Vec<SocialLink>` | Social media links | Yes |
| `launch_timestamp` | `u64` | UNIX timestamp of project launch | Yes |
| `bounty_url` | `String` | URL to bug bounty program or disclosure policy | Yes |
| `bounty_cid` | `String` | IPFS CID for detailed bounty/disclosure documentation | Yes |
| `maintainers` | `Vec<Address>` | Maintainer addresses (defaults to empty list) | Yes |
| `archived` | `bool` | Whether the project is archived | No (default false) |
| `created_at` | `u64` | Timestamp of project creation | No |
| `updated_at` | `u64` | Timestamp of last update | No |

## Validation Rules

- `bounty_url`: If provided, must be a valid HTTP/HTTPS URL (starts with `http://` or `https://`).
- `bounty_cid`: If provided, must be a valid IPFS CID (v0 starting with `Qm` and 46 characters, or v1 starting with `b` and at least 40 characters).

## Indexes

| Key | Value | Description |
|-----|-------|-------------|
| `OwnerProjects(Address)` | `Vec<u64>` | List of project IDs owned by an address |
| `ProjectCount` | `u64` | Total number of registered projects |
| `ProjectSlug(String)` | `u64` | Slug-to-ID lookup |
| `FeaturedProjects` | `Vec<u64>` | List of featured project IDs |
