/// RatingCalculator provides utility functions for computing and updating
/// project rating aggregates efficiently without floating-point arithmetic.
///
/// All ratings are scaled by 100 to maintain two decimal places of precision.
/// For example, a rating of 4.50 is stored as 450.
pub struct RatingCalculator;

#[allow(dead_code)]
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
    #[allow(dead_code)]
    pub fn add_rating(current_sum: u64, current_count: u32, new_rating: u32) -> (u64, u32, u32) {
        let scaled_rating = (new_rating as u64) * 100;
        let new_sum = current_sum + scaled_rating;
        let new_count = current_count + 1;
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
    #[allow(dead_code)]
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
}
