# Review Moderation - Quick Reference

## API Methods

### report_review()
```rust
pub fn report_review(
    env: Env,
    project_id: u64,
    reviewer: Address,
    reporter: Address,
) -> Result<(), ContractError>
```
**Access**: Any user (requires auth)  
**Purpose**: Report an abusive review  
**Errors**: ProjectNotFound, ReviewNotFound, ReviewAlreadyReported  

### hide_review()
```rust
pub fn hide_review(
    env: Env,
    project_id: u64,
    reviewer: Address,
    admin: Address,
) -> Result<(), ContractError>
```
**Access**: Admin only  
**Purpose**: Hide a reported review  
**Errors**: AdminOnly, ProjectNotFound, ReviewNotFound, ReviewAlreadyHidden  

### restore_review()
```rust
pub fn restore_review(
    env: Env,
    project_id: u64,
    reviewer: Address,
    admin: Address,
) -> Result<(), ContractError>
```
**Access**: Admin only  
**Purpose**: Restore a hidden review  
**Errors**: AdminOnly, ProjectNotFound, ReviewNotFound, ReviewNotHidden  

---

## Data Model

### Review Struct
```rust
pub struct Review {
    pub project_id: u64,
    pub reviewer: Address,
    pub rating: u32,
    pub ipfs_cid: Option<String>,
    pub comment_cid: Option<String>,
    pub owner_response: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
    pub hidden: bool,              // NEW
    pub report_count: u32,         // NEW
}
```

---

## Error Types

| Error | Code | Meaning |
|-------|------|---------|
| ReviewAlreadyReported | 39 | User already reported this review |
| ReviewAlreadyHidden | 40 | Review is already hidden |
| ReviewNotHidden | 41 | Review is not hidden |

---

## Events

### ReviewReportedEvent
```rust
pub struct ReviewReportedEvent {
    pub project_id: u64,
    pub reviewer: Address,
    pub reporter: Address,
    pub timestamp: u64,
}
```

### ReviewHiddenEvent
```rust
pub struct ReviewHiddenEvent {
    pub project_id: u64,
    pub reviewer: Address,
    pub admin: Address,
    pub timestamp: u64,
}
```

### ReviewRestoredEvent
```rust
pub struct ReviewRestoredEvent {
    pub project_id: u64,
    pub reviewer: Address,
    pub admin: Address,
    pub timestamp: u64,
}
```

---

## Storage Keys

```rust
ReviewReport(u64, Address, Address)  // (project_id, reviewer, reporter)
```

---

## Behavior Summary

### Reporting
- User calls report_review()
- System checks if reporter already reported
- Increments report_count
- Tracks report in storage
- Emits ReviewReportedEvent

### Hiding
- Admin calls hide_review()
- System verifies admin status
- Sets hidden = true
- Recalculates stats (removes rating)
- Emits ReviewHiddenEvent

### Restoring
- Admin calls restore_review()
- System verifies admin status
- Sets hidden = false
- Recalculates stats (adds rating back)
- Emits ReviewRestoredEvent

---

## Stats Behavior

### When Hiding
```
new_rating_sum = old_rating_sum - review.rating
new_review_count = old_review_count - 1
new_average_rating = new_rating_sum / new_review_count
```

### When Restoring
```
new_rating_sum = old_rating_sum + review.rating
new_review_count = old_review_count + 1
new_average_rating = new_rating_sum / new_review_count
```

---

## List Reviews Behavior

```rust
// Hidden reviews are excluded
let reviews = client.list_reviews(&project_id, &0, &100);
// Only visible reviews returned

// But get_review() returns hidden reviews
let review = client.get_review(&project_id, &reviewer);
// Returns hidden review if it exists
```

---

## Test Coverage

| Category | Tests | Status |
|----------|-------|--------|
| Report Review | 5 | ✅ |
| Hide Review | 6 | ✅ |
| Restore Review | 5 | ✅ |
| List Reviews | 2 | ✅ |
| Complex Scenarios | 5 | ✅ |
| **Total** | **23** | **✅** |

---

## Usage Examples

### Report a Review
```rust
client.report_review(&project_id, &reviewer_address, &reporter_address)?;
```

### Hide a Review
```rust
client.hide_review(&project_id, &reviewer_address, &admin_address)?;
```

### Restore a Review
```rust
client.restore_review(&project_id, &reviewer_address, &admin_address)?;
```

### Check Review Status
```rust
let review = client.get_review(&project_id, &reviewer_address)?;
println!("Hidden: {}", review.hidden);
println!("Reports: {}", review.report_count);
```

### List Reviews (excludes hidden)
```rust
let reviews = client.list_reviews(&project_id, &0, &100);
```

---

## Access Control

| Method | Access | Auth Required |
|--------|--------|---------------|
| report_review() | Any user | Yes |
| hide_review() | Admin only | Yes (admin) |
| restore_review() | Admin only | Yes (admin) |
| get_review() | Any user | No |
| list_reviews() | Any user | No |

---

## Integration Points

- **Admin Manager**: Verifies admin status
- **Project Registry**: Validates project existence
- **Rating Calculator**: Recalculates stats
- **Storage Manager**: Extends TTL
- **Events**: Publishes moderation events

---

## Key Design Decisions

1. **Hidden reviews still accessible via get_review()** - Allows admin verification
2. **Report count preserved on hide** - Maintains audit trail
3. **Automatic stats recalculation** - Ensures consistency
4. **Duplicate report prevention** - Prevents spam
5. **Admin-only moderation** - Ensures authorized decisions
6. **Separate report tracking** - Enables audit trails

---

## Files Modified

- src/types.rs
- src/errors.rs
- src/events.rs
- src/storage_keys.rs
- src/review_registry.rs
- src/lib.rs
- src/tests/mod.rs
- src/tests/moderation.rs (NEW)

---

## Documentation

- REVIEW_MODERATION_FEATURE.md - Full documentation
- PR_REVIEW_MODERATION.md - PR template
- TASK3_COMPLETION_SUMMARY.md - Task summary
- MODERATION_QUICK_REFERENCE.md - This file

---

## Status

✅ Implementation: COMPLETE  
✅ Tests: COMPLETE (23 tests)  
✅ Documentation: COMPLETE  
✅ Ready for: PULL REQUEST  
