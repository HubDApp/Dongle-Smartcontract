# Review Moderation Feature

## Overview

The Review Moderation feature enables users to report abusive reviews and allows administrators to hide or restore reviews. This feature ensures the integrity of the review system by providing a mechanism to manage inappropriate content.

## Acceptance Criteria

✅ Users can report a review  
✅ Admins can hide or restore a review  
✅ Hidden reviews are excluded from default list APIs and rating stats  
✅ Tests cover reporting, hiding, restoring, and stats behavior  

## Implementation Details

### Data Model Changes

#### Review Struct (src/types.rs)
Added two new fields to the `Review` struct:
- `hidden: bool` - Whether the review is hidden by moderation (default: false)
- `report_count: u32` - Number of times this review has been reported (default: 0)

### Storage Keys (src/storage_keys.rs)

Added new storage key for tracking reports:
- `ReviewReport(u64, Address, Address)` - Tracks (project_id, reviewer_address, reporter_address) to prevent duplicate reports from the same user

### Error Types (src/errors.rs)

Added three new error types:
- `ReviewAlreadyReported` (39) - Raised when a user tries to report the same review twice
- `ReviewAlreadyHidden` (40) - Raised when trying to hide an already hidden review
- `ReviewNotHidden` (41) - Raised when trying to restore a review that is not hidden

### Events (src/events.rs)

Added three new event types:
- `ReviewReportedEvent` - Emitted when a review is reported
- `ReviewHiddenEvent` - Emitted when a review is hidden by an admin
- `ReviewRestoredEvent` - Emitted when a review is restored by an admin

### Core Implementation (src/review_registry.rs)

#### 1. report_review()
```rust
pub fn report_review(
    env: &Env,
    project_id: u64,
    reviewer: Address,
    reporter: Address,
) -> Result<(), ContractError>
```

**Behavior:**
- Requires reporter authentication
- Validates project exists
- Validates review exists
- Prevents duplicate reports from the same reporter
- Increments `report_count` on the review
- Tracks the report in storage to prevent duplicates
- Emits `ReviewReportedEvent`

**Errors:**
- `ProjectNotFound` - If project doesn't exist
- `ReviewNotFound` - If review doesn't exist
- `ReviewAlreadyReported` - If reporter has already reported this review

#### 2. hide_review()
```rust
pub fn hide_review(
    env: &Env,
    project_id: u64,
    reviewer: Address,
    admin: Address,
) -> Result<(), ContractError>
```

**Behavior:**
- Requires admin authentication
- Validates admin status
- Validates project exists
- Validates review exists
- Sets `hidden = true` on the review
- **Recalculates project stats to exclude the hidden review:**
  - Removes the review's rating from `rating_sum`
  - Decrements `review_count`
  - Recalculates `average_rating`
- Extends TTL for review and stats data
- Emits `ReviewHiddenEvent`

**Errors:**
- `AdminOnly` - If caller is not an admin
- `ProjectNotFound` - If project doesn't exist
- `ReviewNotFound` - If review doesn't exist
- `ReviewAlreadyHidden` - If review is already hidden

#### 3. restore_review()
```rust
pub fn restore_review(
    env: &Env,
    project_id: u64,
    reviewer: Address,
    admin: Address,
) -> Result<(), ContractError>
```

**Behavior:**
- Requires admin authentication
- Validates admin status
- Validates project exists
- Validates review exists
- Sets `hidden = false` on the review
- **Recalculates project stats to include the restored review:**
  - Adds the review's rating back to `rating_sum`
  - Increments `review_count`
  - Recalculates `average_rating`
- Extends TTL for review and stats data
- Emits `ReviewRestoredEvent`

**Errors:**
- `AdminOnly` - If caller is not an admin
- `ProjectNotFound` - If project doesn't exist
- `ReviewNotFound` - If review doesn't exist
- `ReviewNotHidden` - If review is not hidden

### API Changes (src/lib.rs)

Added three new contract methods:
- `report_review(project_id, reviewer, reporter) -> Result<(), ContractError>`
- `hide_review(project_id, reviewer, admin) -> Result<(), ContractError>`
- `restore_review(project_id, reviewer, admin) -> Result<(), ContractError>`

### List Reviews Behavior (src/review_registry.rs)

Updated `list_reviews()` to exclude hidden reviews by default:
- Iterates through project reviews
- Skips reviews where `hidden == true`
- Returns only visible reviews

**Note:** `get_review()` still returns hidden reviews (for admin access and verification)

### Stats Calculation

Hidden reviews are automatically excluded from stats:
- When a review is hidden, its rating is removed from `rating_sum` and `review_count` is decremented
- When a review is restored, its rating is added back to `rating_sum` and `review_count` is incremented
- `average_rating` is recalculated based on the updated values

## Test Coverage

Comprehensive test suite in `src/tests/moderation.rs` with 20+ test cases:

### Report Review Tests
- ✅ `test_report_review_success` - Basic reporting functionality
- ✅ `test_report_review_multiple_reporters` - Multiple users can report the same review
- ✅ `test_report_review_duplicate_reporter_fails` - Same user cannot report twice
- ✅ `test_report_review_nonexistent_review_fails` - Cannot report non-existent review
- ✅ `test_report_review_nonexistent_project_fails` - Cannot report review for non-existent project

### Hide Review Tests
- ✅ `test_hide_review_success` - Basic hiding functionality
- ✅ `test_hide_review_updates_stats` - Stats are updated when review is hidden
- ✅ `test_hide_review_already_hidden_fails` - Cannot hide already hidden review
- ✅ `test_hide_review_non_admin_fails` - Only admins can hide reviews
- ✅ `test_hide_review_nonexistent_review_fails` - Cannot hide non-existent review
- ✅ `test_hide_review_nonexistent_project_fails` - Cannot hide review for non-existent project

### Restore Review Tests
- ✅ `test_restore_review_success` - Basic restoration functionality
- ✅ `test_restore_review_updates_stats` - Stats are updated when review is restored
- ✅ `test_restore_review_not_hidden_fails` - Cannot restore non-hidden review
- ✅ `test_restore_review_non_admin_fails` - Only admins can restore reviews
- ✅ `test_restore_review_nonexistent_review_fails` - Cannot restore non-existent review

### List Reviews Tests
- ✅ `test_list_reviews_excludes_hidden` - Hidden reviews are excluded from listings
- ✅ `test_list_reviews_all_hidden` - Empty list when all reviews are hidden

### Complex Scenario Tests
- ✅ `test_hide_restore_hide_cycle` - Multiple hide/restore cycles work correctly
- ✅ `test_report_then_hide` - Report count is preserved when hiding
- ✅ `test_stats_with_mixed_hidden_reviews` - Stats correctly calculated with mixed hidden/visible reviews
- ✅ `test_get_review_returns_hidden_review` - get_review() returns hidden reviews (for admin access)
- ✅ `test_multiple_projects_independent_moderation` - Moderation on one project doesn't affect others

## Usage Examples

### Reporting a Review
```rust
// User reports an abusive review
client.report_review(&project_id, &reviewer_address, &reporter_address)?;
```

### Hiding a Review
```rust
// Admin hides a reported review
client.hide_review(&project_id, &reviewer_address, &admin_address)?;
```

### Restoring a Review
```rust
// Admin restores a previously hidden review
client.restore_review(&project_id, &reviewer_address, &admin_address)?;
```

### Checking Review Status
```rust
// Get review (returns hidden reviews too)
let review = client.get_review(&project_id, &reviewer_address)?;
if review.hidden {
    println!("This review is hidden");
}
println!("Report count: {}", review.report_count);
```

### Listing Reviews
```rust
// List reviews (automatically excludes hidden ones)
let reviews = client.list_reviews(&project_id, &0, &100);
// Only visible reviews are returned
```

## Key Design Decisions

1. **Hidden reviews still accessible via get_review()**: Allows admins to verify why a review was hidden and potentially restore it if needed.

2. **Report count preserved on hide**: When a review is hidden, the report count is preserved. This allows admins to see how many reports led to the hiding decision.

3. **Automatic stats recalculation**: Stats are automatically updated when reviews are hidden/restored, ensuring consistency without requiring manual recalculation.

4. **Duplicate report prevention**: Each user can only report a review once, preventing spam and ensuring accurate report counts.

5. **Admin-only moderation**: Only admins can hide or restore reviews, ensuring moderation decisions are made by authorized personnel.

6. **Separate report tracking**: Reports are tracked separately from the review itself, allowing for audit trails and analytics.

## Integration Points

- **Admin Manager**: Uses `AdminManager::is_admin()` to verify admin status
- **Project Registry**: Validates project existence before moderation operations
- **Rating Calculator**: Recalculates stats when reviews are hidden/restored
- **Storage Manager**: Extends TTL for review and stats data
- **Events**: Publishes moderation events for off-chain indexing

## Future Enhancements

1. **Report reasons**: Add optional reason field to reports for better moderation insights
2. **Moderation queue**: Implement a queue of reported reviews for admin review
3. **Auto-hide threshold**: Automatically hide reviews after reaching a certain report count
4. **Moderation history**: Track who hid/restored reviews and when
5. **Appeal mechanism**: Allow reviewers to appeal hidden reviews
