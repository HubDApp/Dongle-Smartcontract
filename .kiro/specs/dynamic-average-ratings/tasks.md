# Implementation Plan

- [x] 1. Set up project dependencies and type definitions
  - Add `proptest` dependency to Cargo.toml for property-based testing
  - Define extended Project struct with rating aggregate fields (rating_sum, review_count, average_rating)
  - Define Review struct with all required fields
  - Define ContractError enum with rating-specific error variants
  - _Requirements: 2.1, 2.2, 2.4_

- [x] 2. Implement rating calculation module
  - Create RatingCalculator struct with static methods
  - Implement calculate_average function with two-decimal precision
  - Implement add_rating function to update aggregates when adding reviews
  - Implement update_rating function to update aggregates when modifying reviews
  - Implement remove_rating function to update aggregates when deleting reviews
  - Handle division-by-zero case (return 0 when review_count is 0)
  - _Requirements: 1.4, 2.3, 3.1_

- [ ]* 2.1 Write property test for rating calculation
  - **Property 4: Average rating maintains two decimal precision**
  - **Validates: Requirements 1.4**

- [ ]* 2.2 Write unit tests for rating calculator edge cases
  - Test zero reviews case (division by zero)
  - Test single review case
  - Test precision with various rating combinations
  - _Requirements: 1.5, 3.1, 3.2_

- [x] 3. Implement ReviewRegistry add_review functionality
  - Validate rating is in range 1-5
  - Check if review already exists for this project and reviewer
  - Create and store new Review struct
  - Retrieve current project and update rating aggregates using RatingCalculator
  - Save updated project with new aggregates
  - _Requirements: 1.1, 2.1, 2.2, 2.5_

- [ ]* 3.1 Write property test for add_review
  - **Property 1: Adding a review updates average correctly**
  - **Validates: Requirements 1.1, 2.1, 2.2, 2.3**

- [ ]* 3.2 Write property test for sequential additions
  - **Property 5: Sequential additions maintain accurate aggregates**
  - **Validates: Requirements 3.4**

- [ ]* 3.3 Write unit tests for add_review edge cases
  - Test adding first review to project (0 → 1)
  - Test adding review with each valid rating (1-5)
  - Test error when rating is invalid (<1 or >5)
  - Test error when review already exists
  - _Requirements: 3.2, 1.1_

- [x] 4. Implement ReviewRegistry update_review functionality
  - Validate caller is the original reviewer
  - Retrieve existing review
  - Validate new rating is in range 1-5
  - Update review with new rating and comment
  - Retrieve project and update aggregates using RatingCalculator (subtract old, add new)
  - Save updated review and project
  - _Requirements: 1.2, 3.5_

- [ ]* 4.1 Write property test for update_review
  - **Property 2: Updating a review recalculates average correctly**
  - **Validates: Requirements 1.2, 3.5**

- [ ]* 4.2 Write unit tests for update_review scenarios
  - Test updating to higher rating
  - Test updating to lower rating
  - Test updating to same rating (no change)
  - Test error when review doesn't exist
  - Test error when unauthorized user attempts update
  - _Requirements: 1.2_

- [x] 5. Implement ReviewRegistry delete_review functionality
  - Validate caller is the original reviewer
  - Retrieve existing review
  - Remove review from storage
  - Retrieve project and update aggregates using RatingCalculator
  - Handle case where this is the last review (reset to zero)
  - Save updated project
  - _Requirements: 1.3, 3.3_

- [ ]* 5.1 Write property test for delete_review
  - **Property 3: Deleting a review updates average correctly**
  - **Validates: Requirements 1.3**

- [ ]* 5.2 Write unit tests for delete_review scenarios
  - Test deleting review from project with multiple reviews
  - Test deleting last review (1 → 0 reviews, verify reset)
  - Test error when review doesn't exist
  - Test error when unauthorized user attempts deletion
  - _Requirements: 1.3, 3.3_

- [x] 6. Implement rating aggregate invariant verification
  - Create helper function to verify rating_sum equals sum of all review ratings
  - Create helper function to verify review_count equals actual number of reviews
  - _Requirements: 2.1, 2.2_

- [ ]* 6.1 Write property test for aggregate invariants
  - **Property 6: Rating sum and count invariant**
  - **Validates: Requirements 2.1, 2.2**

- [x] 7. Integrate rating system with ProjectRegistry
  - Initialize rating aggregates to zero when registering new projects
  - Ensure get_project returns projects with rating information
  - Add helper method to retrieve project average rating
  - _Requirements: 2.4_

- [ ]* 7.1 Write integration tests for complete workflow
  - Test full lifecycle: register project, add reviews, update, delete
  - Test multiple reviewers on same project
  - Verify ratings persist correctly across operations
  - _Requirements: 1.1, 1.2, 1.3_

- [x] 8. Final checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.
