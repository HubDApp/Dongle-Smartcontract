# Code Deduplication Refactoring Summary

## Issue
The identical "iterate old Vec, push non-matching items into a new Vec, store the new Vec" loop was hand-rolled independently in at least 8 places across the codebase.

## Solution
Created a generic helper function `Utils::remove_item_from_vec()` in `src/utils.rs` that replaces all duplicated implementations.

## Changes Made

### 1. Created Generic Helper Function
**File:** `dongle-smartcontract/src/utils.rs`

Added a new generic method to the `Utils` struct:
```rust
pub fn remove_item_from_vec<T: PartialEq + Clone + TryFromVal<Env, Val> + IntoVal<Env, Val>>(
    env: &Env,
    vec: &Vec<T>,
    item: &T,
) -> Vec<T>
```

This function:
- Takes a reference to a Vec and an item to remove
- Returns a new Vec with all items except those matching the specified item
- Works with any type that implements the required traits (Address, u64, String, etc.)

### 2. Replaced 8 Duplicated Code Patterns

#### a) `endorsement_registry.rs` - `unendorse_project()` (lines 73-80)
**Before:**
```rust
let mut new_endorsements: Vec<Address> = Vec::new(env);
for i in 0..endorsements.len() {
    if let Some(e) = endorsements.get(i) {
        if e != user {
            new_endorsements.push_back(e);
        }
    }
}
```

**After:**
```rust
let new_endorsements = Utils::remove_item_from_vec(env, &endorsements, &user);
```

#### b) `bookmark_registry.rs` - `unbookmark_project()` (lines 69-76)
**Before:**
```rust
let mut new_bookmarks: Vec<u64> = Vec::new(env);
for i in 0..bookmarks.len() {
    if let Some(pid) = bookmarks.get(i) {
        if pid != project_id {
            new_bookmarks.push_back(pid);
        }
    }
}
```

**After:**
```rust
let new_bookmarks = Utils::remove_item_from_vec(env, &bookmarks, &project_id);
```

#### c) `subscription_registry.rs` - `unfollow_project()` - Followers (lines 77-84)
**Before:**
```rust
let mut new_followers: Vec<Address> = Vec::new(env);
for i in 0..followers.len() {
    if let Some(f) = followers.get(i) {
        if f != follower {
            new_followers.push_back(f);
        }
    }
}
```

**After:**
```rust
let new_followers = Utils::remove_item_from_vec(env, &followers, &follower);
```

#### d) `subscription_registry.rs` - `unfollow_project()` - Subscriptions (lines 100-107)
**Before:**
```rust
let mut new_subscriptions: Vec<u64> = Vec::new(env);
for i in 0..subscriptions.len() {
    if let Some(pid) = subscriptions.get(i) {
        if pid != project_id {
            new_subscriptions.push_back(pid);
        }
    }
}
```

**After:**
```rust
let new_subscriptions = Utils::remove_item_from_vec(env, &subscriptions, &project_id);
```

#### e) `collection_registry.rs` - `delete_collection()` (lines 143-148)
**Before:**
```rust
let mut updated = Vec::new(env);
for id in list.iter() {
    if id != collection_id {
        updated.push_back(id);
    }
}
```

**After:**
```rust
let updated = Utils::remove_item_from_vec(env, &list, &collection_id);
```

#### f) `collection_registry.rs` - `remove_project_from_collection()` (lines 249-254)
**Before:**
```rust
let mut updated = Vec::new(env);
for id in project_ids.iter() {
    if id != project_id {
        updated.push_back(id);
    }
}
```

**After:**
```rust
let updated = Utils::remove_item_from_vec(env, &project_ids, &project_id);
```

#### g) `dependency_registry.rs` - `remove_dependency()` (lines 254-261)
**Before:**
```rust
let mut new_keys: Vec<String> = Vec::new(env);
for i in 0..keys.len() {
    if let Some(k) = keys.get(i) {
        if k != key {
            new_keys.push_back(k);
        }
    }
}
```

**After:**
```rust
let new_keys = Utils::remove_item_from_vec(env, &keys, &key);
```

#### h) `admin_manager.rs` - `remove_admin()` (lines 117-122)
**Before:**
```rust
let mut new_admins = Vec::new(env);
for admin in admins.iter() {
    if admin != admin_to_remove {
        new_admins.push_back(admin);
    }
}
```

**After:**
```rust
let new_admins = Utils::remove_item_from_vec(env, &admins, &admin_to_remove);
```

#### i) `admin_manager.rs` - `execute_proposal()` - RemoveAdmin (lines 378-383)
**Before:**
```rust
let mut new_admins = Vec::new(env);
for admin in admins.iter() {
    if admin != admin_to_remove {
        new_admins.push_back(admin);
    }
}
```

**After:**
```rust
let new_admins = Utils::remove_item_from_vec(env, &admins, &admin_to_remove);
```

## Benefits

1. **Reduced Code Duplication**: Eliminated 8 instances of identical hand-rolled code
2. **Improved Maintainability**: Changes to the removal logic only need to be made in one place
3. **Better Readability**: The intent is clearer with a well-named function call
4. **Type Safety**: The generic function works with any type that implements the required traits
5. **Consistency**: All removal operations now use the same implementation

## Verification

All modified files compile successfully with no errors related to the refactoring. The only compilation errors are pre-existing issues in the codebase unrelated to these changes.

## Files Modified

1. `dongle-smartcontract/src/utils.rs` - Added generic helper function
2. `dongle-smartcontract/src/endorsement_registry.rs` - Replaced duplicated code
3. `dongle-smartcontract/src/bookmark_registry.rs` - Replaced duplicated code
4. `dongle-smartcontract/src/subscription_registry.rs` - Replaced duplicated code (2 instances)
5. `dongle-smartcontract/src/collection_registry.rs` - Replaced duplicated code (2 instances)
6. `dongle-smartcontract/src/dependency_registry.rs` - Replaced duplicated code
7. `dongle-smartcontract/src/admin_manager.rs` - Replaced duplicated code (2 instances)