use starknet::ContractAddress;
use starknet::get_caller_address;
use starknet::storage::*;
use super::super::interfaces::{IRatings, Review, ReviewAdded, ReviewUpdated};
use super::super::cid::Cid;
use super::super::pagination::{safe_slice, validate_pagination};

#[starknet::contract]
pub mod Ratings {
    use super::*;

    #[storage]
    pub struct Storage {
        pub reviews: Map<u32, Review>,
        pub review_count: u32,
        pub last_review_id_by_user_dapp: Map<(ContractAddress, u32), u32>,
        pub agg_sum: Map<u32, u32>,
        pub agg_count: Map<u32, u32>,
    }

    #[event]
    #[derive(Drop, starknet::Event)]
    pub enum Event {
        ReviewAdded: ReviewAdded,
        ReviewUpdated: ReviewUpdated,
    }

    #[constructor]
    fn constructor(ref self: ContractState) {
        self.review_count.write(0);
    }

    #[abi(embed_v0)]
    impl RatingsImpl of IRatings<ContractState> {
        /// Add a review for a dApp
        fn add_review(
            ref self: ContractState,
            dapp_id: u32,
            stars: u8,
            review_cid: Cid
        ) -> u32 {
            // Validate stars range
            assert(stars >= 1 && stars <= 5, 'ERR_INVALID_STARS');
            
            let caller = get_caller_address();
            let review_id = self.review_count.read() + 1;
            
            // Check if user already reviewed this dApp
            let existing_review_id = self.last_review_id_by_user_dapp.read((caller, dapp_id));
            
            if existing_review_id != 0 {
                // User already reviewed, subtract old stars from aggregate
                let old_review = self.reviews.read(existing_review_id);
                let current_sum = self.agg_sum.read(dapp_id);
                self.agg_sum.write(dapp_id, current_sum - old_review.stars.into());
                // Don't change agg_count as reviewer remains counted
            } else {
                // New reviewer, increment count
                let current_count = self.agg_count.read(dapp_id);
                self.agg_count.write(dapp_id, current_count + 1);
            }
            
            // Create new review
            let review = Review {
                id: review_id,
                dapp_id,
                reviewer: caller,
                stars,
                review_cid,
            };
            
            self.reviews.write(review_id, review);
            self.review_count.write(review_id);
            self.last_review_id_by_user_dapp.write((caller, dapp_id), review_id);
            
            // Add new stars to aggregate
            let current_sum = self.agg_sum.read(dapp_id);
            self.agg_sum.write(dapp_id, current_sum + stars.into());
            
            self.emit(Event::ReviewAdded(ReviewAdded {
                review_id,
                dapp_id,
                reviewer: caller,
                stars,
                review_cid,
            }));
            
            review_id
        }

        /// Update an existing review
        fn update_review(ref self: ContractState, review_id: u32, stars: u8, review_cid: Cid) {
            // Validate stars range
            assert(stars >= 1 && stars <= 5, 'ERR_INVALID_STARS');
            
            let caller = get_caller_address();
            let review = self.reviews.read(review_id);
            
            // Only original reviewer can update
            assert(review.reviewer == caller, 'ERR_NOT_REVIEWER');
            
            let dapp_id = review.dapp_id;
            let old_stars = review.stars;
            
            // Update review
            let mut updated_review = review;
            updated_review.stars = stars;
            updated_review.review_cid = review_cid;
            self.reviews.write(review_id, updated_review);
            
            // Adjust aggregate sum
            let current_sum = self.agg_sum.read(dapp_id);
            self.agg_sum.write(dapp_id, current_sum - old_stars.into() + stars.into());
            
            self.emit(Event::ReviewUpdated(ReviewUpdated {
                review_id,
                dapp_id,
                reviewer: caller,
                stars,
                review_cid,
            }));
        }

        /// Get average rating for a dApp (times 100 for precision)
        fn get_average(self: @ContractState, dapp_id: u32) -> (u16, u32) {
            let count = self.agg_count.read(dapp_id);
            if count == 0 {
                return (0, 0);
            };
            
            let sum = self.agg_sum.read(dapp_id);
            let avg_times_100 = ((sum * 100) / count).try_into().unwrap();
            (avg_times_100, count)
        }

        /// List reviews for a dApp with pagination
        fn list_reviews(self: @ContractState, dapp_id: u32, offset: u32, limit: u32) -> Array<Review> {
            validate_pagination(offset, limit);
            
            let count = self.review_count.read();
            let mut reviews = array![];
            
            let mut i = 1;
            loop {
                if i > count {
                    break;
                };
                let review = self.reviews.read(i);
                if review.dapp_id == dapp_id {
                    reviews.append(review);
                };
                i += 1;
            };
            
            safe_slice(reviews, offset, limit)
        }
    }
}
