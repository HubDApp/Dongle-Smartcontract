# Instructions to Create Pull Requests

Since the GitHub CLI (`gh`) is not available in your environment, here are the manual steps to create the three pull requests:

---

## PR 1: Project Slug Feature

### Step 1: Go to GitHub
Visit: https://github.com/mayasimi/Dongle-Smartcontract

### Step 2: Create Pull Request
1. Click on "Pull requests" tab
2. Click "New pull request" button
3. Set base branch to: `main`
4. Set compare branch to: `feature/project-slug`
5. Click "Create pull request"

### Step 3: Fill in PR Details
**Title:**
```
feat: implement project slug feature
```

**Description:**
```
## Overview
This PR implements the Project Slug feature, enabling projects to be identified by stable, URL-friendly slugs in addition to numeric IDs.

## Changes
- Add `slug: String` field to Project struct
- Implement slug validation (lowercase alphanumeric, hyphens, underscores, max 64 chars)
- Implement `get_project_by_slug()` method for O(1) lookups
- Add ProjectBySlug storage key for duplicate detection
- Add duplicate slug detection during registration and updates
- Update test fixtures to include slug parameter
- Create comprehensive test suite with 20+ test cases

## Test Coverage
- 20+ comprehensive test cases
- All scenarios covered
- Edge cases handled

## Acceptance Criteria
✅ Project registration accepts a unique slug
✅ Slug format is validated
✅ Projects can be fetched by slug
✅ Updating slug handles duplicate checks and old slug cleanup

## Files Changed
- src/types.rs - Added slug field
- src/errors.rs - Added slug validation errors
- src/constants.rs - Added MAX_SLUG_LEN
- src/utils.rs - Added validate_project_slug()
- src/storage_keys.rs - Added ProjectBySlug key
- src/project_registry.rs - Slug implementation
- src/lib.rs - Exposed get_project_by_slug
- src/tests/slug.rs - Test suite
- src/tests/fixtures.rs - Updated create_test_project helper
```

### Step 4: Submit
Click "Create pull request"

---

## PR 2: Review Moderation Feature

### Step 1: Go to GitHub
Visit: https://github.com/mayasimi/Dongle-Smartcontract

### Step 2: Create Pull Request
1. Click on "Pull requests" tab
2. Click "New pull request" button
3. Set base branch to: `main`
4. Set compare branch to: `feature/review-moderation`
5. Click "Create pull request"

### Step 3: Fill in PR Details
**Title:**
```
feat: implement review moderation feature
```

**Description:**
```
## Overview
This PR implements the Review Moderation feature, enabling users to report abusive reviews and allowing administrators to hide or restore reviews.

## Changes
- Add `report_review()` method for users to report reviews
- Add `hide_review()` method for admins to hide reported reviews
- Add `restore_review()` method for admins to restore hidden reviews
- Add `hidden` and `report_count` fields to Review struct
- Update `list_reviews()` to exclude hidden reviews by default
- Automatically recalculate stats when reviews are hidden/restored
- Add ReviewReport storage key for tracking duplicate reports
- Add moderation events: ReviewReportedEvent, ReviewHiddenEvent, ReviewRestoredEvent
- Add moderation errors: ReviewAlreadyReported, ReviewAlreadyHidden, ReviewNotHidden

## Test Coverage
- 23 comprehensive test cases covering:
  - Reporting functionality and duplicate prevention
  - Hiding reviews and stats recalculation
  - Restoring reviews and stats restoration
  - List reviews excluding hidden reviews
  - Complex scenarios with multiple operations
  - Admin-only access control
  - Error handling for all edge cases

## Acceptance Criteria
✅ Users can report a review
✅ Admins can hide or restore a review
✅ Hidden reviews are excluded from default list APIs and rating stats
✅ Tests cover reporting, hiding, restoring, and stats behavior

## Files Changed
- src/types.rs - Added hidden and report_count fields to Review
- src/errors.rs - Added moderation error types
- src/events.rs - Added moderation event types
- src/storage_keys.rs - Added ReviewReport storage key
- src/review_registry.rs - Implemented moderation methods
- src/lib.rs - Exposed moderation methods in contract interface
- src/tests/moderation.rs - Comprehensive test suite
- src/tests/mod.rs - Registered moderation test module
```

### Step 4: Submit
Click "Create pull request"

---

## PR 3: Verification Renewal Feature

### Step 1: Go to GitHub
Visit: https://github.com/mayasimi/Dongle-Smartcontract

### Step 2: Create Pull Request
1. Click on "Pull requests" tab
2. Click "New pull request" button
3. Set base branch to: `main`
4. Set compare branch to: `feature/verification-renewal`
5. Click "Create pull request"

### Step 3: Fill in PR Details
**Title:**
```
feat: implement verification renewal feature
```

**Description:**
```
## Overview
This PR implements the Verification Renewal feature, enabling project owners to renew their verification before or after expiry, and allowing administrators to approve or reject renewal requests.

## Changes
- Add `request_renewal()` method for owners to request renewal
- Add `approve_renewal()` method for admins to approve renewals
- Add `reject_renewal()` method for admins to reject renewals
- Add `get_renewal_request()` to retrieve current renewal request
- Add `get_renewal_history()` to retrieve renewal history with pagination
- Add `is_verification_expired()` to check verification expiry status
- Add `expires_at` and `last_renewed_at` fields to VerificationRecord
- Add VerificationRenewalRecord struct for tracking renewals
- Add VerificationRenewal, VerificationRenewalHistory, VerificationRenewalCount storage keys
- Add VerificationRenewalNotFound, VerificationRenewalAlreadyPending, CannotRenewUnverified, VerificationNotExpired errors
- Add VerificationRenewalRequestedEvent, VerificationRenewalApprovedEvent, VerificationRenewalRejectedEvent
- Add VERIFICATION_VALIDITY_PERIOD constant (365 days)

## Test Coverage
- 20+ comprehensive test cases covering:
  - Renewal request functionality
  - Admin approval and rejection
  - Renewal history tracking
  - Expiry checking
  - Complex scenarios with multiple operations
  - Access control validation
  - Error handling for all edge cases

## Acceptance Criteria
✅ Verified projects can request renewal before or after expiry
✅ Renewal uses separate state and record history
✅ Admin approval extends verification validity
✅ Tests cover renewal request, approval, rejection, and invalid transitions

## Files Changed
- src/types.rs - Added renewal record types
- src/errors.rs - Added renewal error types
- src/events.rs - Added renewal event types
- src/storage_keys.rs - Added renewal storage keys
- src/constants.rs - Added verification validity period
- src/verification_registry.rs - Implemented renewal methods
- src/lib.rs - Exposed renewal methods
- src/tests/renewal.rs - Comprehensive test suite
- src/tests/mod.rs - Registered renewal test module
```

### Step 4: Submit
Click "Create pull request"

---

## Alternative: Using Web Interface Directly

If you prefer, you can also create PRs directly from the GitHub web interface:

1. Go to: https://github.com/mayasimi/Dongle-Smartcontract
2. Click "Pull requests" tab
3. Click "New pull request"
4. Select the feature branch as "compare" and "main" as "base"
5. Fill in the title and description
6. Click "Create pull request"

---

## Verification

After creating each PR, you should see:
- PR title and description
- List of commits
- List of files changed
- Test results (if CI/CD is configured)

---

## Notes

- All three PRs should be created against the `main` branch
- Each PR is independent and can be merged separately
- The PRs are ready for review and can be merged immediately after approval
- No additional changes are needed before merging

---

## Quick Links

- **Project Slug PR**: https://github.com/mayasimi/Dongle-Smartcontract/compare/main...feature/project-slug
- **Review Moderation PR**: https://github.com/mayasimi/Dongle-Smartcontract/compare/main...feature/review-moderation
- **Verification Renewal PR**: https://github.com/mayasimi/Dongle-Smartcontract/compare/main...feature/verification-renewal

You can use these direct links to create the PRs!
