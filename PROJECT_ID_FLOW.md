# Project ID Assignment Flow

## Sequential ID Counter System

```
┌─────────────────────────────────────────────────────────────┐
│                    Project Registration Flow                 │
└─────────────────────────────────────────────────────────────┘

User A registers "Project Alpha"
    │
    ├─> next_project_id() → Returns 1 (default)
    ├─> Create Project { id: 1, ... }
    ├─> Store Project(1) → Project data
    ├─> set_next_project_id(1) → Stores 2
    └─> Emit ProjectRegistered { project_id: 1, ... }

User B registers "Project Beta"
    │
    ├─> next_project_id() → Returns 2
    ├─> Create Project { id: 2, ... }
    ├─> Store Project(2) → Project data
    ├─> set_next_project_id(2) → Stores 3
    └─> Emit ProjectRegistered { project_id: 2, ... }

User A registers "Project Gamma"
    │
    ├─> next_project_id() → Returns 3
    ├─> Create Project { id: 3, ... }
    ├─> Store Project(3) → Project data
    ├─> set_next_project_id(3) → Stores 4
    └─> Emit ProjectRegistered { project_id: 3, ... }
```

## Storage Structure

```
Persistent Storage:
┌──────────────────────────────────────────────────────────┐
│ Key: NextProjectId          │ Value: 4                   │
├──────────────────────────────────────────────────────────┤
│ Key: Project(1)             │ Value: Project { id: 1 }   │
├──────────────────────────────────────────────────────────┤
│ Key: Project(2)             │ Value: Project { id: 2 }   │
├──────────────────────────────────────────────────────────┤
│ Key: Project(3)             │ Value: Project { id: 3 }   │
└──────────────────────────────────────────────────────────┘
```

## Key Functions

### `next_project_id(env: &Env) -> u64`
```rust
// Retrieves the next available ID
// Returns 1 if no projects exist yet
env.storage()
    .persistent()
    .get(&DataKey::NextProjectId)
    .unwrap_or(1)
```

### `set_next_project_id(env: &Env, id: u64)`
```rust
// Increments the counter for next registration
// Stores (current_id + 1)
env.storage()
    .persistent()
    .set(&DataKey::NextProjectId, &(id + 1))
```

## Guarantees

1. **Uniqueness**: Each ID is used exactly once
2. **Sequential**: IDs increment by 1 (1, 2, 3, 4, ...)
3. **Persistent**: Counter survives contract restarts
4. **Atomic**: No race conditions in Soroban's execution model
5. **Immutable**: Project IDs never change after creation

## Usage Across Contract

### Project Registry
- `register_project()` - Assigns new ID
- `get_project(id)` - Retrieves by ID
- `update_project(id, ...)` - Updates using ID (ID unchanged)

### Review Registry
- `add_review(project_id, ...)` - References project by ID
- `get_review(project_id, reviewer)` - Looks up reviews by project ID

### Verification Registry
- `request_verification(project_id, ...)` - Links verification to project ID
- `get_verification(project_id)` - Retrieves verification by project ID

### Events
- `ProjectRegistered { project_id, ... }` - Emitted with ID
- `ReviewAdded { project_id, ... }` - References project ID
- `VerificationRequested { project_id, ... }` - References project ID

## Test Coverage

| Test | Purpose |
|------|---------|
| `test_ids_are_sequential` | Basic sequential assignment (1, 2) |
| `test_multiple_users_unique_sequential_ids` | Multiple users get unique IDs |
| `test_project_retrieval_returns_correct_id` | Retrieval returns correct ID |
| `test_events_contain_correct_project_ids` | Events include correct IDs |
| `test_ids_continue_incrementing` | IDs continue to 10+ |
| `test_no_duplicate_ids_rapid_registration` | No duplicates in rapid registration |
| `test_first_project_id_is_one` | First ID is 1, not 0 |
| `test_ids_sequential_across_categories` | Categories don't affect IDs |
| `test_update_preserves_project_id` | Updates don't change ID |
