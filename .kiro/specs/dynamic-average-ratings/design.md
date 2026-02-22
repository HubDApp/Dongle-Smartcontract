# Design Document: Dynamic Average Ratings

## Overview

This design implements a dynamic average rating system for the Dongle smart contract platform. The system automatically computes and updates project ratings whenever reviews are added, updated, or deleted. The design emphasizes efficiency by maintaining aggregate data (rating sum and review count) rather than recalculating from all reviews on each operation.

## Architecture

The rating system is integrated into the existing smart contract architecture with the following components:

1. **Data Layer**: Extended Project and Review data structures to store rating aggregates
2. **Business Logic Layer**: Rating calculation and update logic in ReviewRegistry
3. **Storage Layer**: Persistent storage of rating aggregates using Soroban SDK Map structures

The system follows an event-driven architecture where review operations (add, update, delete) trigger automatic rating recalculation.

## Components and Interfaces

### 1. Extended Project Type

```rust
pub struct Project {
    pub id: u64,
    pub owner: Address,
    pub name: String,
    pub description: String,
    pub category: String,
    pub website: Option<String>,
    pub logo_cid: Option<String>,
    pub metadata_cid: Option<String>,
    // New fields for rating aggregates
    pub rating_sum: u64,      // Sum of all ratings (scaled by 100 for precision)
    pub review_count: u32,    // Number of active reviews
    pub average_rating: u32,  // Cached average (scaled by 100, e.g., 450 = 4.50)
}
```

### 2. Review Type

```rust
pub struct Review {
    pub project_id: u64,
    pub reviewer: Address,
    pub rating: u8,           // Rating value 1-5
    pub comment_cid: Option<String>,
    pub timestamp: u64,
}
```

### 3. ReviewRegistry Interface

```rust
impl ReviewRegistry {
    // Add a new review and update project rating
    pub fn add_review(
        env: &Env,
        project_id: u64,
        reviewer: Address,
        rating: u8,
        comment_cid: Option<String>
    ) -> Result<(), ContractError>;

    // Update existing review and recalculate rating
    pub fn update_review(
        env: &Env,
        project_id: u64,
        reviewer: Address,
        new_rating: u8,
        comment_cid: Option<String>
    ) -> Result<(), ContractError>;

    // Delete review and update rating
    pub fn delete_review(
        env: &Env,
        project_id: u64,
        reviewer: Address
    ) -> Result<(), ContractError>;

    // Get review for a specific project and reviewer
    pub fn get_review(
        env: &Env,
        project_id: u64,
        reviewer: Address
    ) -> Option<Review>;
}
```

### 4. Rating Calculation Module

```rust
pub struct RatingCalculator;

impl RatingCalculator {
    // Calculate average rating from sum and count
    pub fn calculate_average(rating_sum: u64, review_count: u32) -> u32;
    
    // Update rating aggregates when adding a review
    pub fn add_rating(current_sum: u64, current_count: u32, new_rating: u8) -> (u64, u32, u32);
    
    // Update rating aggregates when updating a review
    pub fn update_rating(current_sum: u64, current_count: u32, old_rating: u8, new_rating: u8) -> (u64, u32, u32);
    
    // Update rating aggregates when deleting a review
    pub fn remove_rating(current_sum: u64, current_count: u32, rating: u8) -> (u64, u32, u32);
}
```

## Data Models

### Storage Structure

```
Projects: Map<u64, Project>
  - Key: project_id
  - Value: Project struct with rating aggregates

Reviews: Map<(u64, Address), Review>
  - Key: (project_id, reviewer_address)
  - Value: Review struct
```

### Rating Precision

To maintain two decimal places without floating-point arithmetic:
- Ratings are stored as integers scaled by 100
- Example: 4.50 stars = 450
- Input ratings (1-5) are multiplied by 100 before aggregation
- Average is computed as: (rating_sum / review_count)

## Correctness Properties

*A property is a characteristic or behavior that should hold true across all valid executions of a system—essentially, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.*


### Property Reflection

After analyzing all acceptance criteria, I identified the following redundancies:
- Property 1.2 and 3.5 both test rating updates - they can be combined into a single comprehensive property
- Properties 2.1 and 2.2 (maintaining sum and count) are implicitly validated by the other properties that check correct average calculation
- Edge cases 1.5 and 3.1 are identical (zero reviews handling)

The refined property set eliminates redundancy while maintaining complete validation coverage.

### Correctness Properties

Property 1: Adding a review updates average correctly
*For any* project with existing reviews and any valid rating value (1-5), when a new review is added, the resulting average rating should equal (previous_sum + new_rating × 100) / (previous_count + 1)
**Validates: Requirements 1.1, 2.1, 2.2, 2.3**

Property 2: Updating a review recalculates average correctly
*For any* project with an existing review, when that review's rating is updated from old_rating to new_rating, the resulting average should equal (previous_sum - old_rating × 100 + new_rating × 100) / review_count
**Validates: Requirements 1.2, 3.5**

Property 3: Deleting a review updates average correctly
*For any* project with multiple reviews, when a review is deleted, the resulting average should equal (previous_sum - deleted_rating × 100) / (previous_count - 1)
**Validates: Requirements 1.3**

Property 4: Average rating maintains two decimal precision
*For any* computed average rating value, the value should be representable with at most two decimal places (i.e., when divided by 100, has at most 2 decimal digits)
**Validates: Requirements 1.4**

Property 5: Sequential additions maintain accurate aggregates
*For any* sequence of review additions to a project, the final rating_sum should equal the sum of all individual ratings × 100, and review_count should equal the number of additions
**Validates: Requirements 3.4**

Property 6: Rating sum and count invariant
*For any* project at any point in time, the stored rating_sum should equal the sum of all active review ratings × 100, and review_count should equal the number of active reviews
**Validates: Requirements 2.1, 2.2**

## Error Handling

### Error Types

```rust
pub enum ContractError {
    InvalidRating,           // Rating not in range 1-5
    ReviewNotFound,          // Attempting to update/delete non-existent review
    ReviewAlreadyExists,     // Attempting to add duplicate review
    ProjectNotFound,         // Project doesn't exist
    UnauthorizedReviewer,    // Caller is not the original reviewer
}
```

### Error Scenarios

1. **Invalid Rating Value**: When rating < 1 or rating > 5, return `InvalidRating`
2. **Division by Zero**: When review_count = 0, set average_rating = 0 (no error)
3. **Review Not Found**: When updating/deleting non-existent review, return `ReviewNotFound`
4. **Duplicate Review**: When adding review that already exists, return `ReviewAlreadyExists`
5. **Unauthorized Update**: When non-owner attempts to update review, return `UnauthorizedReviewer`

## Testing Strategy

### Unit Testing Approach

Unit tests will verify specific scenarios and edge cases:

1. **Single Review Tests**
   - Adding first review to project (0 → 1 reviews)
   - Verifying correct initialization of aggregates
   - Testing each valid rating value (1-5)

2. **Multiple Review Tests**
   - Adding multiple reviews sequentially
   - Verifying cumulative average calculation
   - Testing with different rating combinations

3. **Update Tests**
   - Updating review to higher rating
   - Updating review to lower rating
   - Updating review to same rating (no change)

4. **Delete Tests**
   - Deleting review from project with multiple reviews
   - Deleting last review (1 → 0 reviews)
   - Verifying aggregate reset to zero

5. **Edge Case Tests**
   - Zero reviews (division by zero handling)
   - Maximum reviews scenario
   - Boundary rating values (1 and 5)
   - Precision verification (two decimal places)

### Property-Based Testing Approach

Property-based tests will verify universal properties across randomly generated inputs using the `proptest` crate for Rust:

1. **Property Test Configuration**
   - Minimum 100 iterations per property test
   - Random generation of projects, reviews, and ratings
   - Shrinking enabled for minimal failing examples

2. **Test Generators**
   - `arbitrary_project()`: Generates projects with random rating aggregates
   - `arbitrary_review()`: Generates reviews with ratings 1-5
   - `arbitrary_rating()`: Generates valid rating values (1-5)
   - `arbitrary_review_sequence()`: Generates sequences of review operations

3. **Property Test Implementation**
   - Each property test tagged with: `// Feature: dynamic-average-ratings, Property N: [description]`
   - Tests validate mathematical correctness across all inputs
   - Tests verify invariants hold after all operations

### Test Framework

- **Unit Tests**: Rust's built-in `#[test]` framework
- **Property Tests**: `proptest` crate (version 1.0+)
- **Assertions**: Custom assertion helpers for rating comparison with tolerance

## Implementation Considerations

### Precision and Overflow

- Use `u64` for rating_sum to prevent overflow (max: 5 × 100 × 4,294,967,295 reviews)
- Use `u32` for review_count (supports up to 4.2 billion reviews)
- Use `u32` for average_rating (range: 100-500 for ratings 1.00-5.00)

### Performance

- O(1) time complexity for all rating operations (no iteration over reviews)
- Constant storage overhead per project (3 additional fields)
- Atomic updates ensure consistency

### Storage Efficiency

- Rating aggregates stored directly in Project struct
- No separate aggregate storage structure needed
- Minimal storage footprint: 16 bytes per project (u64 + u32 + u32)

## Deployment and Migration

### Initial Deployment

New projects automatically initialize with:
- `rating_sum = 0`
- `review_count = 0`
- `average_rating = 0`

### Migration Strategy (if applicable)

For existing projects with reviews but no aggregates:
1. Iterate through all reviews for each project
2. Calculate rating_sum and review_count
3. Compute and store average_rating
4. One-time migration script or lazy initialization on first access
