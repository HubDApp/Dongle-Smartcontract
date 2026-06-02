# Code Changes Reference - Archive & Reactivate Feature

## Quick Navigation

This document provides exact line numbers and locations for all code changes made to implement the archive/reactivate feature.

## Modified Files

### 1. src/types.rs

**Change**: Added `archived: bool` field to Project struct

**Location**: Lines 76-90

```rust
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Project {
    pub id: u64,
    pub owner: Address,
    pub name: String,
    pub description: String,
    pub category: String,
    pub website: Option<String>,
    pub logo_cid: Option<String>,
    pub metadata_cid: Option<String>,
    pub verification_status: VerificationStatus,
    pub created_at: u64,
    pub updated_at: u64,
    pub archived: bool,  // ← NEW FIELD (Line 90)
}
```

**Impact**: All Project instances now include archive status

---

### 2. src/errors.rs

**Change**: Added two new error variants

**Location**: Lines 72-75

```rust
    /// Caller is not the designated recipient of the pending transfer
    NotPendingTransferRecipient = 32,
    /// Project is already archived
    ProjectAlreadyArchived = 33,  // ← NEW (Line 73)
    /// Project is not archived
    ProjectNotArchived = 34,      // ← NEW (Line 75)
}
```

**Impact**: New error types for archive/reactivate validation

---

### 3. src/events.rs

**Change 1**: Added ProjectArchivedEvent struct

**Location**: Lines 78-84

```rust
/// Emitted when a project is archived.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectArchivedEvent {
    pub project_id: u64,
    pub owner: Address,
    pub timestamp: u64,
}
```

**Change 2**: Added ProjectReactivatedEvent struct

**Location**: Lines 86-92

```rust
/// Emitted when a project is reactivated.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectReactivatedEvent {
    pub project_id: u64,
    pub owner: Address,
    pub timestamp: u64,
}
```

**Change 3**: Added event publishing functions

**Location**: Lines 337-365

```rust
pub fn publish_project_archived_event(env: &Env, project_id: u64, owner: Address) {
    let event_data = ProjectArchivedEvent {
        project_id,
        owner,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("PROJECT"),
            symbol_short!("ARCHIVED"),
            project_id,
        ),
        event_data,
    );
}

pub fn publish_project_reactivated_event(env: &Env, project_id: u64, owner: Address) {
    let event_data = ProjectReactivatedEvent {
        project_id,
        owner,
        timestamp: env.ledger().timestamp(),
    };
    env.events().publish(
        (
            symbol_short!("PROJECT"),
            symbol_short!("REACTIVATED"),
            project_id,
        ),
        event_data,
    );
}
```

**Impact**: Events for off-chain tracking of archive/reactivate operations

---

### 4. src/project_registry.rs

**Change 1**: Updated imports

**Location**: Lines 1-11

```rust
use crate::events::{
    publish_ownership_transferred_event, publish_project_archived_event,  // ← NEW
    publish_project_reactivated_event, publish_project_registered_event,  // ← NEW
    publish_project_updated_event,
};
```

**Change 2**: Initialize archived field in register_project()

**Location**: Line 87

```rust
let project = Project {
    id: count,
    owner: params.owner.clone(),
    name: params.name.clone(),
    description: params.description,
    category: params.category,
    website: params.website,
    logo_cid: params.logo_cid,
    metadata_cid: params.metadata_cid,
    verification_status: VerificationStatus::Unverified,
    created_at: now,
    updated_at: now,
    archived: false,  // ← NEW (Line 87)
};
```

**Change 3**: Updated get_projects_by_owner() to exclude archived

**Location**: Lines 318-331

```rust
pub fn get_projects_by_owner(env: &Env, owner: Address) -> Vec<Project> {
    let ids: Vec<u64> = env
        .storage()
        .persistent()
        .get(&StorageKey::OwnerProjects(owner))
        .unwrap_or_else(|| Vec::new(env));

    let mut projects = Vec::new(env);
    let len = ids.len();
    for i in 0..len {
        if let Some(project_id) = ids.get(i) {
            if let Some(project) = Self::get_project(env, project_id) {
                if !project.archived {  // ← FILTER ADDED (Line 327)
                    projects.push_back(project);
                }
            }
        }
    }

    projects
}
```

**Change 4**: Updated list_projects_by_status() to exclude archived

**Location**: Lines 395-410

```rust
pub fn list_projects_by_status(
    env: &Env,
    status: VerificationStatus,
    start_id: u64,
    limit: u32,
) -> Vec<Project> {
    // ... pagination logic ...
    let mut collected: u32 = 0;
    for id in first..=count {
        if collected >= effective_limit {
            break;
        }
        if let Some(project) = Self::get_project(env, id) {
            if project.verification_status == status && !project.archived {  // ← FILTER ADDED (Line 404)
                projects.push_back(project);
                collected += 1;
            }
        }
    }
    projects
}
```

**Change 5**: Updated list_projects() to exclude archived

**Location**: Lines 430-455

```rust
pub fn list_projects(env: &Env, start_id: u64, limit: u32) -> Vec<Project> {
    // ... pagination logic ...
    let mut collected: u32 = 0;
    for id in first..end {
        if collected >= effective_limit {
            break;
        }
        if let Some(project) = Self::get_project(env, id) {
            if !project.archived {  // ← FILTER ADDED (Line 450)
                projects.push_back(project);
                collected += 1;
            }
        }
    }
    projects
}
```

**Change 6**: Updated list_projects_by_category() to exclude archived

**Location**: Lines 475-500

```rust
pub fn list_projects_by_category(
    env: &Env,
    category: String,
    start_id: u32,
    limit: u32,
) -> Vec<Project> {
    // ... pagination logic ...
    let mut collected: u32 = 0;
    for i in start_id..end {
        if collected >= effective_limit {
            break;
        }
        if let Some(id) = category_projects.get(i) {
            if let Some(project) = Self::get_project(env, id) {
                if !project.archived {  // ← FILTER ADDED (Line 492)
                    projects.push_back(project);
                    collected += 1;
                }
            }
        }
    }
    projects
}
```

**Change 7**: Added archive_project() method

**Location**: Lines 626-656

```rust
/// Archive a project. Only the project owner can archive their project.
/// Archived projects no longer appear in listing APIs.
pub fn archive_project(
    env: &Env,
    project_id: u64,
    caller: Address,
) -> Result<(), ContractError> {
    let mut project =
        Self::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;

    caller.require_auth();
    if project.owner != caller {
        return Err(ContractError::Unauthorized);
    }

    if project.archived {
        return Err(ContractError::ProjectAlreadyArchived);
    }

    project.archived = true;
    project.updated_at = env.ledger().timestamp();
    env.storage()
        .persistent()
        .set(&StorageKey::Project(project_id), &project);

    StorageManager::extend_project_ttl(env, project_id);

    publish_project_archived_event(env, project_id, caller);
    Ok(())
}
```

**Change 8**: Added reactivate_project() method

**Location**: Lines 658-688

```rust
/// Reactivate an archived project. Only the project owner can reactivate their project.
/// Reactivated projects appear again in listing APIs.
/// Updates updated_at timestamp.
pub fn reactivate_project(
    env: &Env,
    project_id: u64,
    caller: Address,
) -> Result<(), ContractError> {
    let mut project =
        Self::get_project(env, project_id).ok_or(ContractError::ProjectNotFound)?;

    caller.require_auth();
    if project.owner != caller {
        return Err(ContractError::Unauthorized);
    }

    if !project.archived {
        return Err(ContractError::ProjectNotArchived);
    }

    project.archived = false;
    project.updated_at = env.ledger().timestamp();
    env.storage()
        .persistent()
        .set(&StorageKey::Project(project_id), &project);

    StorageManager::extend_project_ttl(env, project_id);

    publish_project_reactivated_event(env, project_id, caller);
    Ok(())
}
```

**Impact**: Core archive/reactivate functionality and listing API filtering

---

### 5. src/lib.rs

**Change**: Added archive_project() and reactivate_project() to contract interface

**Location**: Lines 149-165

```rust
    pub fn archive_project(
        env: Env,
        project_id: u64,
        caller: Address,
    ) -> Result<(), ContractError> {
        ProjectRegistry::archive_project(&env, project_id, caller)
    }

    pub fn reactivate_project(
        env: Env,
        project_id: u64,
        caller: Address,
    ) -> Result<(), ContractError> {
        ProjectRegistry::reactivate_project(&env, project_id, caller)
    }
```

**Impact**: Exposes archive/reactivate methods to contract callers

---

### 6. src/tests/mod.rs

**Change**: Added archive test module

**Location**: Line 14

```rust
//! Test suite organized by domain area.

// Existing test modules
mod admin;
mod error_handling_tests;
mod fee;
mod indexer;
mod registration;
mod review;
mod transfer;
mod verification;

// New test modules
mod authorization;
mod events;
mod pagination;
mod archive;  // ← NEW (Line 14)

// Test infrastructure
pub mod fixtures;
```

**Impact**: Registers archive test module

---

## New Files Created

### 1. src/tests/archive.rs

**Purpose**: Comprehensive test suite for archive/reactivate functionality

**Size**: ~400 lines

**Test Cases**: 20

**Location**: `src/tests/archive.rs`

**Key Tests**:
- `test_archive_project_by_owner()` - Basic archive functionality
- `test_archive_project_updates_timestamp()` - Timestamp update verification
- `test_archive_project_unauthorized()` - Authorization check
- `test_archive_nonexistent_project()` - Error handling
- `test_archive_already_archived_project()` - State validation
- `test_reactivate_project_by_owner()` - Basic reactivate functionality
- `test_reactivate_project_updates_timestamp()` - Timestamp update verification
- `test_reactivate_project_unauthorized()` - Authorization check
- `test_reactivate_nonexistent_project()` - Error handling
- `test_reactivate_non_archived_project()` - State validation
- `test_archived_project_excluded_from_list_projects()` - Listing API filtering
- `test_archived_project_excluded_from_list_projects_by_status()` - Status filtering
- `test_archived_project_excluded_from_list_projects_by_category()` - Category filtering
- `test_archived_project_excluded_from_get_projects_by_owner()` - Owner filtering
- `test_archive_reactivate_lifecycle()` - Full lifecycle
- `test_multiple_archive_reactivate_cycles()` - Multiple cycles
- `test_archived_project_still_accessible_via_get_project()` - Direct access
- `test_archive_preserves_project_metadata()` - Data preservation
- `test_reactivate_preserves_project_metadata()` - Data preservation

---

## Summary of Changes

| File | Type | Changes | Lines |
|------|------|---------|-------|
| src/types.rs | Modified | Added `archived: bool` field | 1 line added |
| src/errors.rs | Modified | Added 2 error variants | 3 lines added |
| src/events.rs | Modified | Added 2 event types + 2 publishing functions | 30 lines added |
| src/project_registry.rs | Modified | Added 2 methods + updated 4 listing methods | 100+ lines added/modified |
| src/lib.rs | Modified | Exposed 2 new methods | 16 lines added |
| src/tests/mod.rs | Modified | Added archive test module | 1 line added |
| src/tests/archive.rs | Created | New test suite | 400 lines |

**Total**: 6 files modified, 1 file created, ~550 lines of code added

---

## Testing

**Run all archive tests**:
```bash
cargo test archive
```

**Run specific test**:
```bash
cargo test archive::test_archive_project_by_owner
```

**Run with output**:
```bash
cargo test archive -- --nocapture
```

---

## Verification Checklist

- [ ] All 20 tests pass
- [ ] No compilation warnings
- [ ] Code follows existing patterns
- [ ] Documentation complete
- [ ] Backward compatibility verified
- [ ] Performance impact minimal
- [ ] Security review passed

---

## Related Documentation

- `ARCHIVE_REACTIVATE_IMPLEMENTATION.md` - Detailed implementation guide
- `ARCHIVE_QUICK_REFERENCE.md` - Quick reference guide
- `IMPLEMENTATION_SUMMARY.md` - High-level summary
- `CODE_CHANGES_REFERENCE.md` - This file
