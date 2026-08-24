# Project Slug

URL-friendly, stable identifiers for projects. Slugs enable cleaner frontend URLs and better indexing while keeping numeric project IDs.

## Acceptance criteria

1. **Registration accepts a unique slug** — required on `ProjectRegistrationParams`; duplicates rejected
2. **Format validated** — lowercase alphanumeric, hyphens, underscores; must start/end alphanumeric; max 64 chars (`MAX_SLUG_LEN`)
3. **Fetch by slug** — `get_project_by_slug` returns the full `Project`
4. **Updates handle duplicates and cleanup** — optional slug on `ProjectUpdateParams`; old `ProjectBySlug` mapping removed; new mapping written

## Implementation

| Area | Location |
|------|----------|
| `Project.slug` / registration & update params | `dongle-smartcontract/src/types.rs` |
| Errors `InvalidProjectSlug` (35), `ProjectSlugTooLong` (36), `InvalidProjectSlugFormat` (37), `ProjectSlugAlreadyExists` (38) | `dongle-smartcontract/src/errors.rs` |
| `MAX_SLUG_LEN = 64` | `dongle-smartcontract/src/constants.rs` |
| `Utils::validate_project_slug` | `dongle-smartcontract/src/utils.rs` |
| `StorageKey::ProjectBySlug(String)` → `project_id` | `dongle-smartcontract/src/storage_keys.rs` |
| Register / update / lookup | `dongle-smartcontract/src/project_registry.rs` |
| Contract entrypoint `get_project_by_slug` | `dongle-smartcontract/src/lib.rs` |
| Tests | `dongle-smartcontract/src/tests/slug.rs` |

### Validation rules

Pattern: `^[a-z0-9]([a-z0-9_-]*[a-z0-9])?$`

- Not empty or whitespace-only
- Max 64 characters
- Lowercase `a-z`, digits, `-`, `_` only
- Start and end with alphanumeric

**Valid:** `my-project`, `project_123`, `awesome-app-v2`, `a`, `123`  
**Invalid:** `My-Project`, `-project`, `project-`, `my project`, `my@project`

### Storage

- `Project` stores `slug`
- `ProjectBySlug(slug)` maps slug → `project_id` for O(1) lookup
- On update: remove old mapping, write new mapping (duplicate check excludes current project)

### API

```rust
pub fn register_project(env: Env, params: ProjectRegistrationParams) -> Result<u64, ContractError>
pub fn get_project_by_slug(env: Env, slug: String) -> Option<Project>
pub fn update_project(env: Env, params: ProjectUpdateParams) -> Result<Project, ContractError>
```

`ProjectRegistrationParams.slug` is required. `ProjectUpdateParams.slug` is `Option<String>`.

### Tests

```bash
cd dongle-smartcontract && cargo test slug
```

Coverage includes registration, lookup, uniqueness, format edge cases, and update/cleanup (~20 cases).

## Compatibility & performance

- Numeric IDs unchanged; `get_project` still works
- Slug lookup O(1); validation O(n) with n ≤ 64
- One extra persistent key per project

## Use cases

- Frontend: `/projects/my-awesome-project` instead of `/projects/123`
- Sharing memorable links while keeping ID-based APIs

## Migration notes

For existing projects: derive slugs from names, suffix duplicates (`my-project-2`), validate, write `ProjectBySlug` mappings, verify ID and slug lookups agree.
