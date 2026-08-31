/// RatingCalculator provides utility functions for computing and updating
/// project rating aggregates efficiently without floating-point arithmetic.
///
/// All ratings are scaled by 100 to maintain two decimal places of precision.
/// For example, a rating of 4.50 is stored as 450.
pub struct RatingCalculator;

impl RatingCalculator {
    /// Calculate average rating from sum and count.
    /// Returns 0 if review_count is 0 (handles division by zero).
    ///
    /// # Arguments
    /// * `rating_sum` - Sum of all ratings (scaled by 100)
    /// * `review_count` - Number of active reviews
    ///
    /// # Returns
    /// Average rating scaled by 100 (e.g., 450 = 4.50)
    pub fn calculate_average(rating_sum: u64, review_count: u32) -> u32 {
        if review_count == 0 {
            return 0;
        }
        (rating_sum / review_count as u64) as u32
    }

    /// Update rating aggregates when adding a new review.
    ///
    /// # Arguments
    /// * `current_sum` - Current rating sum (scaled by 100)
    /// * `current_count` - Current review count
    /// * `new_rating` - New rating value (1-5)
    ///
    /// # Returns
    /// Tuple of (new_sum, new_count, new_average)
    pub fn add_rating(current_sum: u64, current_count: u32, new_rating: u32) -> (u64, u32, u32) {
        let scaled_rating = (new_rating as u64) * 100;
        // Saturating, matching update_rating/remove_rating below (issue #694):
        // a rating aggregate degrading gracefully by clamping at the type's
        // bound, rather than panicking/wrapping, is the existing convention
        // for this struct — nothing about accumulating one more rating
        // should abort a review submission with a hard error.
        let new_sum = current_sum.saturating_add(scaled_rating);
        let new_count = current_count.saturating_add(1);
        let new_average = Self::calculate_average(new_sum, new_count);
        (new_sum, new_count, new_average)
    }

    /// Update rating aggregates when updating an existing review.
    ///
    /// # Arguments
    /// * `current_sum` - Current rating sum (scaled by 100)
    /// * `current_count` - Current review count
    /// * `old_rating` - Previous rating value (1-5)
    /// * `new_rating` - New rating value (1-5)
    ///
    /// # Returns
    /// Tuple of (new_sum, new_count, new_average)
    pub fn update_rating(
        current_sum: u64,
        current_count: u32,
        old_rating: u32,
        new_rating: u32,
    ) -> (u64, u32, u32) {
        let scaled_old = (old_rating as u64) * 100;
        let scaled_new = (new_rating as u64) * 100;
        let new_sum = current_sum
            .saturating_sub(scaled_old)
            .saturating_add(scaled_new);
        let new_average = Self::calculate_average(new_sum, current_count);
        (new_sum, current_count, new_average)
    }

    /// Update rating aggregates when deleting a review.
    ///
    /// # Arguments
    /// * `current_sum` - Current rating sum (scaled by 100)
    /// * `current_count` - Current review count
    /// * `rating` - Rating value being removed (1-5)
    ///
    /// # Returns
    /// Tuple of (new_sum, new_count, new_average)
    pub fn remove_rating(current_sum: u64, current_count: u32, rating: u32) -> (u64, u32, u32) {
        let scaled_rating = (rating as u64) * 100;
        let new_sum = current_sum.saturating_sub(scaled_rating);
        let new_count = current_count.saturating_sub(1);
        let new_average = Self::calculate_average(new_sum, new_count);
        (new_sum, new_count, new_average)
    }

    /// Calculate Bayesian weighted rating using stored aggregates.
    ///
    /// # Algorithm Overview
    /// This implements a Bayesian average (also known as a weighted rating) to provide
    /// more reliable ratings for projects with few reviews. The algorithm smooths
    /// extreme ratings by blending actual review data with a prior belief about what
    /// a typical project rating should be.
    ///
    /// # Formula
    /// ```text
    /// weighted = (C * m + rating_sum) / (C + review_count)
    /// ```
    ///
    /// Where:
    /// - `C` = `WEIGHTED_RATING_PRIOR_COUNT` (5) - represents the "strength" of the prior
    /// - `m` = `WEIGHTED_RATING_PRIOR_MEAN` (350 = 3.50 stars) - the prior mean rating
    /// - `rating_sum` = sum of individual ratings each scaled by 100
    /// - `review_count` = number of actual reviews
    ///
    /// # Weight Factors Explained
    ///
    /// ## Prior Count (C = 5)
    /// - Represents the weight given to the prior belief vs actual data
    /// - A value of 5 means the prior is treated as if it were 5 hypothetical reviews
    /// - Higher values give more weight to the prior (more conservative ratings)
    /// - Lower values give more weight to actual reviews (more volatile ratings)
    /// - Chosen as 5 to balance stability with responsiveness to genuine feedback
    ///
    /// ## Prior Mean (m = 3.50 stars)
    /// - Represents the baseline rating for an "average" project
    /// - Set to 3.50 (middle of 1-5 scale, slightly above midpoint) to assume
    ///   most projects are decent but not perfect
    /// - Prevents new projects from starting at the extremes (1.0 or 5.0)
    /// - As review count grows, the actual average dominates this prior
    ///
    /// ## Current Implementation Notes
    /// The current algorithm uses a simple Bayesian average with two factors:
    /// - Review count (implicitly weighted through the Bayesian formula)
    /// - Prior mean (fixed at 3.50)
    ///
    /// Future enhancements could add additional weight factors:
    /// - Review age: decay older reviews to favor recent feedback
    /// - Reviewer reputation: weight reviews from trusted reviewers more heavily
    /// - Review helpfulness: weight reviews marked as helpful by other users
    ///
    /// # Edge Cases
    /// - `review_count == 0` → returns prior mean `m` (3.50 stars)
    /// - `review_count == 1` → blends prior with the single review (weighted toward prior)
    /// - `review_count == C` (5) → prior and actual data have equal weight
    /// - `review_count >> C` → converges toward the arithmetic mean (actual data dominates)
    ///
    /// # Example Calculations
    /// ```text
    /// Project with 0 reviews: weighted = (5 * 3.50 + 0) / (5 + 0) = 3.50
    /// Project with 1 review of 5.0: weighted = (5 * 3.50 + 5.0) / (5 + 1) = 3.75
    /// Project with 5 reviews of 5.0: weighted = (5 * 3.50 + 25.0) / (5 + 5) = 4.25
    /// Project with 100 reviews of 4.0: weighted = (5 * 3.50 + 400.0) / (5 + 100) ≈ 3.98
    /// ```
    ///
    /// # Returns
    /// Weighted rating scaled by 100 (e.g., 375 = 3.75 stars)
    pub fn calculate_weighted(rating_sum: u64, review_count: u32) -> u32 {
        use crate::constants::{WEIGHTED_RATING_PRIOR_COUNT, WEIGHTED_RATING_PRIOR_MEAN};
        let c = WEIGHTED_RATING_PRIOR_COUNT as u64;
        let m = WEIGHTED_RATING_PRIOR_MEAN as u64;
        let numerator = c.saturating_mul(m).saturating_add(rating_sum);
        let denominator = c.saturating_add(review_count as u64);
        if denominator == 0 {
            return WEIGHTED_RATING_PRIOR_MEAN;
        }
        (numerator / denominator) as u32
    }
}

#[cfg(test)]
mod prop_tests {
    extern crate std;
    use super::RatingCalculator;
    use proptest::prelude::*;

    // Valid rating range: 1–5 (matches RATING_MIN / RATING_MAX constants)
    const RATING_RANGE: core::ops::RangeInclusive<u32> = 1..=5;
    // Reasonable ceiling so arithmetic never overflows u64
    const MAX_SUM: u64 = 500_000;
    const MAX_COUNT: u32 = 1_000;

    proptest! {
        /// Adding a rating and then immediately removing it restores the original (sum, count, avg).
        #[test]
        fn prop_add_then_remove_is_identity(
            sum in 0u64..MAX_SUM,
            count in 0u32..MAX_COUNT,
            rating in RATING_RANGE,
        ) {
            let (new_sum, new_count, _) = RatingCalculator::add_rating(sum, count, rating);
            let (restored_sum, restored_count, restored_avg) =
                RatingCalculator::remove_rating(new_sum, new_count, rating);
            prop_assert_eq!(restored_sum, sum);
            prop_assert_eq!(restored_count, count);
            prop_assert_eq!(restored_avg, RatingCalculator::calculate_average(sum, count));
        }

        /// Updating a rating to the same value never changes sum, count, or average.
        #[test]
        fn prop_update_same_rating_is_identity(
            sum in 0u64..MAX_SUM,
            count in 1u32..MAX_COUNT,
            rating in RATING_RANGE,
        ) {
            prop_assume!(sum >= (rating as u64) * 100);
            let (new_sum, new_count, new_avg) =
                RatingCalculator::update_rating(sum, count, rating, rating);
            prop_assert_eq!(new_sum, sum);
            prop_assert_eq!(new_count, count);
            prop_assert_eq!(new_avg, RatingCalculator::calculate_average(sum, count));
        }

        /// calculate_average is exactly integer division of sum by count.
        #[test]
        fn prop_average_equals_integer_division(
            rating_sum in 0u64..1_000_000u64,
            review_count in 1u32..MAX_COUNT,
        ) {
            let avg = RatingCalculator::calculate_average(rating_sum, review_count);
            prop_assert_eq!(avg, (rating_sum / review_count as u64) as u32);
        }

        /// calculate_average returns 0 for zero reviews regardless of sum.
        #[test]
        fn prop_average_zero_for_empty(sum in 0u64..MAX_SUM) {
            prop_assert_eq!(RatingCalculator::calculate_average(sum, 0), 0);
        }

        /// add_rating increases sum by exactly rating * 100 and increments count by 1.
        #[test]
        fn prop_add_increases_sum_and_count(
            sum in 0u64..MAX_SUM,
            count in 0u32..MAX_COUNT,
            rating in RATING_RANGE,
        ) {
            let (new_sum, new_count, _) = RatingCalculator::add_rating(sum, count, rating);
            prop_assert_eq!(new_sum, sum + (rating as u64) * 100);
            prop_assert_eq!(new_count, count + 1);
        }

        /// update_rating changes sum by (new - old) * 100 and leaves count unchanged.
        #[test]
        fn prop_update_sum_delta_and_stable_count(
            sum in 0u64..MAX_SUM,
            count in 1u32..MAX_COUNT,
            old_rating in RATING_RANGE,
            new_rating in RATING_RANGE,
        ) {
            let (new_sum, new_count, _) =
                RatingCalculator::update_rating(sum, count, old_rating, new_rating);
            let expected = sum
                .saturating_sub((old_rating as u64) * 100)
                .saturating_add((new_rating as u64) * 100);
            prop_assert_eq!(new_sum, expected);
            prop_assert_eq!(new_count, count);
        }

        /// remove_rating decreases sum by rating * 100 (saturating) and count by 1 (saturating).
        #[test]
        fn prop_remove_decreases_sum_and_count(
            sum in 0u64..MAX_SUM,
            count in 1u32..MAX_COUNT,
            rating in RATING_RANGE,
        ) {
            let (new_sum, new_count, _) = RatingCalculator::remove_rating(sum, count, rating);
            prop_assert_eq!(new_sum, sum.saturating_sub((rating as u64) * 100));
            prop_assert_eq!(new_count, count - 1);
        }

        /// The average returned by add_rating matches independently computed average.
        #[test]
        fn prop_add_average_consistent(
            sum in 0u64..MAX_SUM,
            count in 0u32..MAX_COUNT,
            rating in RATING_RANGE,
        ) {
            let (new_sum, new_count, new_avg) = RatingCalculator::add_rating(sum, count, rating);
            prop_assert_eq!(
                new_avg,
                RatingCalculator::calculate_average(new_sum, new_count)
            );
        }

        /// The average returned by remove_rating matches independently computed average.
        #[test]
        fn prop_remove_average_consistent(
            sum in 0u64..MAX_SUM,
            count in 1u32..MAX_COUNT,
            rating in RATING_RANGE,
        ) {
            let (new_sum, new_count, new_avg) = RatingCalculator::remove_rating(sum, count, rating);
            prop_assert_eq!(
                new_avg,
                RatingCalculator::calculate_average(new_sum, new_count)
            );
        }

        /// The average returned by update_rating matches independently computed average.
        #[test]
        fn prop_update_average_consistent(
            sum in 0u64..MAX_SUM,
            count in 1u32..MAX_COUNT,
            old_rating in RATING_RANGE,
            new_rating in RATING_RANGE,
        ) {
            let (new_sum, new_count, new_avg) =
                RatingCalculator::update_rating(sum, count, old_rating, new_rating);
            prop_assert_eq!(
                new_avg,
                RatingCalculator::calculate_average(new_sum, new_count)
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_average_zero_reviews() {
        let avg = RatingCalculator::calculate_average(0, 0);
        assert_eq!(avg, 0);
    }

    #[test]
    fn test_calculate_average_single_review() {
        let avg = RatingCalculator::calculate_average(500, 1);
        assert_eq!(avg, 500); // 5.00
    }

    #[test]
    fn test_calculate_average_multiple_reviews() {
        // (4.00 + 5.00 + 3.00) / 3 = 4.00
        let avg = RatingCalculator::calculate_average(1200, 3);
        assert_eq!(avg, 400);
    }

    #[test]
    fn test_calculate_average_precision() {
        // (4.50 + 3.75 + 4.25) / 3 = 4.166... ≈ 4.16
        let avg = RatingCalculator::calculate_average(1250, 3);
        assert_eq!(avg, 416);
    }

    #[test]
    fn test_add_rating_first_review() {
        let (sum, count, avg) = RatingCalculator::add_rating(0, 0, 4);
        assert_eq!(sum, 400);
        assert_eq!(count, 1);
        assert_eq!(avg, 400); // 4.00
    }

    #[test]
    fn test_add_rating_subsequent_review() {
        let (sum, count, avg) = RatingCalculator::add_rating(400, 1, 5);
        assert_eq!(sum, 900);
        assert_eq!(count, 2);
        assert_eq!(avg, 450); // 4.50
    }

    #[test]
    fn test_update_rating_increase() {
        let (sum, count, avg) = RatingCalculator::update_rating(800, 2, 3, 5);
        assert_eq!(sum, 1000);
        assert_eq!(count, 2);
        assert_eq!(avg, 500); // 5.00
    }

    #[test]
    fn test_update_rating_decrease() {
        let (sum, count, avg) = RatingCalculator::update_rating(900, 2, 5, 3);
        assert_eq!(sum, 700);
        assert_eq!(count, 2);
        assert_eq!(avg, 350); // 3.50
    }

    #[test]
    fn test_update_rating_no_change() {
        let (sum, count, avg) = RatingCalculator::update_rating(800, 2, 4, 4);
        assert_eq!(sum, 800);
        assert_eq!(count, 2);
        assert_eq!(avg, 400); // 4.00
    }

    #[test]
    fn test_remove_rating_multiple_reviews() {
        let (sum, count, avg) = RatingCalculator::remove_rating(1200, 3, 4);
        assert_eq!(sum, 800);
        assert_eq!(count, 2);
        assert_eq!(avg, 400); // 4.00
    }

    #[test]
    fn test_remove_rating_last_review() {
        let (sum, count, avg) = RatingCalculator::remove_rating(400, 1, 4);
        assert_eq!(sum, 0);
        assert_eq!(count, 0);
        assert_eq!(avg, 0);
    }

    // ── Weighted Rating Tests ───────────────────────────────────────────────

    #[test]
    fn test_calculate_weighted_zero_reviews() {
        // Project with 0 reviews: weighted = (5 * 3.50 + 0) / (5 + 0) = 3.50
        let weighted = RatingCalculator::calculate_weighted(0, 0);
        assert_eq!(weighted, 350); // 3.50 stars
    }

    #[test]
    fn test_calculate_weighted_single_review_five_stars() {
        // Project with 1 review of 5.0: weighted = (5 * 3.50 + 5.0) / (5 + 1) = 3.75
        let rating_sum = 500; // 5.00 * 100
        let weighted = RatingCalculator::calculate_weighted(rating_sum, 1);
        assert_eq!(weighted, 375); // 3.75 stars
    }

    #[test]
    fn test_calculate_weighted_single_review_one_star() {
        // Project with 1 review of 1.0: weighted = (5 * 3.50 + 1.0) / (5 + 1) = 3.08
        let rating_sum = 100; // 1.00 * 100
        let weighted = RatingCalculator::calculate_weighted(rating_sum, 1);
        assert_eq!(weighted, 308); // 3.08 stars (rounded down)
    }

    #[test]
    fn test_calculate_weighted_five_reviews_five_stars() {
        // Project with 5 reviews of 5.0: weighted = (5 * 3.50 + 25.0) / (5 + 5) = 4.25
        let rating_sum = 2500; // 5.00 * 5 * 100
        let weighted = RatingCalculator::calculate_weighted(rating_sum, 5);
        assert_eq!(weighted, 425); // 4.25 stars
    }

    #[test]
    fn test_calculate_weighted_five_reviews_one_star() {
        // Project with 5 reviews of 1.0: weighted = (5 * 3.50 + 5.0) / (5 + 5) = 2.25
        let rating_sum = 500; // 1.00 * 5 * 100
        let weighted = RatingCalculator::calculate_weighted(rating_sum, 5);
        assert_eq!(weighted, 225); // 2.25 stars
    }

    #[test]
    fn test_calculate_weighted_many_reviews_four_stars() {
        // Project with 100 reviews of 4.0: weighted = (5 * 3.50 + 400.0) / (5 + 100) ≈ 3.98
        let rating_sum = 40000; // 4.00 * 100 * 100
        let weighted = RatingCalculator::calculate_weighted(rating_sum, 100);
        assert_eq!(weighted, 398); // 3.98 stars (rounded down)
    }

    #[test]
    fn test_calculate_weighted_mixed_reviews() {
        // Project with 10 reviews: two 5s, three 4s, three 3s, two 2s
        // Average = (10 + 12 + 9 + 4) / 10 = 3.50
        // Weighted = (5 * 3.50 + 35.0) / (5 + 10) = 3.50
        let rating_sum = 3500; // 3.50 * 10 * 100
        let weighted = RatingCalculator::calculate_weighted(rating_sum, 10);
        assert_eq!(weighted, 350); // 3.50 stars
    }

    #[test]
    fn test_calculate_weighted_convergence_to_average() {
        // With many reviews, weighted rating should converge to arithmetic mean
        // 1000 reviews of 4.5 stars
        let rating_sum = 450000; // 4.50 * 1000 * 100
        let weighted = RatingCalculator::calculate_weighted(rating_sum, 1000);
        // Calculation: (5 * 350 + 450000) / (5 + 1000) = 451750 / 1005 = 449.50 → 449
        assert_eq!(weighted, 449); // Very close to 450 (4.50 stars)
    }

    #[test]
    fn test_calculate_weighted_prior_dominance() {
        // With few reviews, prior should dominate
        // 1 review of 5.0 should be pulled down toward 3.50
        let rating_sum = 500; // 5.00 * 100
        let weighted = RatingCalculator::calculate_weighted(rating_sum, 1);
        // Result (375) should be closer to prior (350) than to actual (500)
        assert!(weighted < 450 && weighted > 350);
    }
}
