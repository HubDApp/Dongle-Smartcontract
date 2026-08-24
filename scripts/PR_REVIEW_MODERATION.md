# Pull Request: Review Moderation Feature

## Title
feat: implement review moderation feature

## Description

This PR implements the Review Moderation feature, enabling users to report abusive reviews and allowing administrators to hide or restore reviews.

## Changes

### Core Implementation
- Add `report_review()` method for users to report reviews
- Add `hide_review()` method for admins to hide reported reviews  
- Add `restore_review()` method for admins to restore hidden reviews
- Add `hidden` and `report_count` fields to Review struct
- Update `list_reviews()` to exclude hidden reviews by default
- Automatically recalculate stats when reviews are hidden/restored

### Data Model
- Added `hidden: bool` field to Review struct (default: false)
- Added `report_count: u32` field to Review struct (default: 0)
- Added ReviewReport storage key for tracking (project_id, reviewer_address, reporter_address)

### Events
- ReviewReportedEvent - Emitted when a review is reported
- ReviewHiddenEvent - Emitted when a review is hidden by admin
- ReviewRestoredEvent - Emitted when a review is restored by admin

### Error Handling
- ReviewAlreadyReported (39) - User already reported this review
- ReviewAlreadyHidden (40) - Review is already hidden
- ReviewNotHidden (41) - Review is not hidden

## Test Coverage

Comprehensive test suite with 20+ test cases:

### Report Review Tests (5 tests)
- ✅ test_report_review_success
- ✅ test_report_review_multiple_reporters
- ✅ test_report_review_duplicate_reporter_fails
- ✅ test_report_review_nonexistent_review_fails
- ✅ test_report_review_nonexistent_project_fails

### Hide Review Tests (6 tests)
- ✅ test_hide_review_success
- ✅ test_hide_review_updates_stats
- ✅ test_hide_review_already_hidden_fails
- ✅ test_hide_review_non_admin_fails
- ✅ test_hide_review_nonexistent_review_fails
- ✅ test_hide_review_nonexistent_project_fails

### Restore Review Tests (5 tests)
- ✅ test_restore_review_success
- ✅ test_restore_review_updates_stats
- ✅ test_restore_review_not_hidden_fails
- ✅ test_restore_review_non_admin_fails
- ✅ test_restore_review_nonexistent_review_fails

### List Reviews Tests (2 tests)
- ✅ test_list_reviews_excludes_hidden
- ✅ test_list_reviews_all_hidden

### Complex Scenario Tests (5 tests)
- ✅ test_hide_restore_hide_cycle
- ✅ test_report_then_hide
- ✅ test_stats_with_mixed_hidden_reviews
- ✅ test_get_review_returns_hidden_review
- ✅ test_multiple_projects_independent_moderation

## Acceptance Criteria

✅ Users can report a review
✅ Admins can hide or restore a review
✅ Hidden reviews are excluded from default list APIs and rating stats
✅ Tests cover reporting, hiding, restoring, and stats behavior

## Files Changed

1. **src/types.rs**
   - Added `hidden: bool` field to Review struct
   - Added `report_count: u32` field to Review struct

2. **src/errors.rs**
   - Added ReviewAlreadyReported error (39)
   - Added ReviewAlreadyHidden error (40)
   - Added ReviewNotHidden error (41)

3. **src/events.rs**
   - Added ReviewReportedEvent struct
   - Added ReviewHiddenEvent struct
   - Added ReviewRestoredEvent struct
   - Added publish_review_reported_event() function
   - Added publish_review_hidden_event() function
   - Added publish_review_restored_event() function

4. **src/storage_keys.rs**
   - Added ReviewReport(u64, Address, Address) storage key

5. **src/review_registry.rs**
   - Updated list_reviews() to exclude hidden reviews
   - Added report_review() method
   - Added hide_review() method
   - Added restore_review() method

6. **src/lib.rs**
   - Added report_review() contract method
   - Added hide_review() contract method
   - Added restore_review() contract method

7. **src/tests/moderation.rs** (NEW)
   - Comprehensive test suite with 23 test cases

8. **src/tests/mod.rs**
   - Registered moderation test module

9. **REVIEW_MODERATION_FEATURE.md** (NEW)
   - Complete feature documentation

## Key Design Decisions

1. **Hidden reviews still accessible via get_review()**: Allows admins to verify why a review was hidden and potentially restore it.

2. **Report count preserved on hide**: When a review is hidden, the report count is preserved for audit purposes.

3. **Automatic stats recalculation**: Stats are automatically updated when reviews are hidden/restored, ensuring consistency.

4. **Duplicate report prevention**: Each user can only report a review once, preventing spam.

5. **Admin-only moderation**: Only admins can hide or restore reviews.

6. **Separate report tracking**: Reports are tracked separately from the review for audit trails.

## Integration Points

- **Admin Manager**: Uses AdminManager::is_admin() for access control
- **Project Registry**: Validates project existence
- **Rating Calculator**: Recalculates stats on hide/restore
- **Storage Manager**: Extends TTL for data persistence
- **Events**: Publishes moderation events for indexing

## Branch Information

- **Branch**: feature/review-moderation
- **Base**: main
- **Commit**: 1a6c901
- **Files Changed**: 9
- **Insertions**: 1022
- **Deletions**: 1

## How to Test

1. Checkout the feature branch:
   ```bash
   git checkout feature/review-moderation
   ```

2. Run the test suite:
   ```bash
   cargo test --lib moderation
   ```

3. Run all tests:
   ```bash
   cargo test
   ```

## Deployment Notes

- No database migrations required
- No breaking changes to existing APIs
- Backward compatible with existing reviews
- New fields default to safe values (hidden=false, report_count=0)

## Future Enhancements

1. Add report reasons for better moderation insights
2. Implement moderation queue for admin review
3. Auto-hide reviews after reaching report threshold
4. Track moderation history (who hid/restored and when)
5. Implement appeal mechanism for reviewers
