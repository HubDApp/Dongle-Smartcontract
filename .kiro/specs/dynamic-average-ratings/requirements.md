# Requirements Document

## Introduction

This document specifies the requirements for implementing dynamic average ratings for projects in the Dongle smart contract platform. The feature ensures that project ratings are automatically computed and updated whenever reviews are added, updated, or deleted, providing accurate and up-to-date ratings without requiring off-chain recalculation.

## Glossary

- **Project**: A registered entity on the Dongle platform that can receive reviews and ratings
- **Review**: A user-submitted evaluation of a project consisting of a rating (1-5) and optional comment
- **Average Rating**: The mean of all rating values for a specific project, computed dynamically
- **Rating Sum**: The cumulative total of all rating values for a project
- **Review Count**: The total number of active reviews for a project
- **Reviewer**: A user (identified by Address) who submits a review for a project
- **Smart Contract**: The Soroban-based blockchain contract managing the Dongle platform

## Requirements

### Requirement 1

**User Story:** As a platform user, I want to see accurate average ratings for projects, so that I can make informed decisions about project quality.

#### Acceptance Criteria

1. WHEN a review is added to a project, THE Smart Contract SHALL update the project's average rating by incorporating the new rating value
2. WHEN a review is updated with a different rating, THE Smart Contract SHALL recalculate the project's average rating using the new rating value
3. WHEN a review is deleted from a project, THE Smart Contract SHALL recalculate the project's average rating excluding the deleted rating value
4. WHEN the average rating is computed, THE Smart Contract SHALL round the result to two decimal places
5. WHEN a project has zero reviews, THE Smart Contract SHALL return a rating of zero or null

### Requirement 2

**User Story:** As a smart contract developer, I want the rating system to maintain aggregate data efficiently, so that rating calculations are performant and cost-effective.

#### Acceptance Criteria

1. THE Smart Contract SHALL maintain a running total of all rating values for each project
2. THE Smart Contract SHALL maintain a count of active reviews for each project
3. WHEN computing average rating, THE Smart Contract SHALL divide the rating sum by the review count
4. WHEN storing rating aggregates, THE Smart Contract SHALL persist both rating sum and review count in the project data structure
5. THE Smart Contract SHALL update rating aggregates atomically with review operations

### Requirement 3

**User Story:** As a quality assurance engineer, I want comprehensive edge case handling for ratings, so that the system behaves correctly under all conditions.

#### Acceptance Criteria

1. WHEN a project has zero reviews, THE Smart Contract SHALL handle the division-by-zero case gracefully
2. WHEN the first review is added to a project, THE Smart Contract SHALL initialize rating aggregates correctly
3. WHEN the last review is deleted from a project, THE Smart Contract SHALL reset rating aggregates to zero state
4. WHEN multiple reviews are added in sequence, THE Smart Contract SHALL maintain accurate cumulative totals
5. WHEN a review rating is updated from value A to value B, THE Smart Contract SHALL subtract A and add B to the rating sum

### Requirement 4

**User Story:** As a developer, I want comprehensive unit tests for the rating system, so that I can verify correctness and prevent regressions.

#### Acceptance Criteria

1. THE Smart Contract SHALL include unit tests that verify average rating calculation for single reviews
2. THE Smart Contract SHALL include unit tests that verify average rating calculation for multiple reviews
3. THE Smart Contract SHALL include unit tests that verify rating updates when reviews are modified
4. THE Smart Contract SHALL include unit tests that verify rating updates when reviews are deleted
5. THE Smart Contract SHALL include unit tests that verify edge cases including zero reviews and boundary values
