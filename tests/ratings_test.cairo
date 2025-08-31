use starknet::ContractAddress;
use starknet::contract_address_const;
use snforge_std::{declare, ContractClassTrait, DeclareResultTrait, EventSpyAssertionsTrait, spy_events, start_cheat_caller_address, stop_cheat_caller_address};
use dongle::interfaces::{IRatingsDispatcher, IRatingsDispatcherTrait, Review, ReviewAdded, ReviewUpdated};
use dongle::cid::cid_from_parts;

fn deploy_ratings() -> IRatingsDispatcher {
    let contract = declare("Ratings");
    let mut constructor_args = array![];
    
    let (contract_address, _) = contract
        .unwrap()
        .contract_class()
        .deploy(@constructor_args)
        .unwrap();
    
    IRatingsDispatcher { contract_address }
}

#[test]
fn test_add_first_review() {
    let dispatcher = deploy_ratings();
    let mut spy = spy_events();
    
    let reviewer: ContractAddress = contract_address_const::<'reviewer'>();
    start_cheat_caller_address(dispatcher.contract_address, reviewer);
    
    let dapp_id = 1_u32;
    let stars = 4_u8;
    let review_cid = cid_from_parts(array!['review']);
    
    let review_id = dispatcher.add_review(dapp_id, stars, review_cid);
    assert(review_id == 1, 'Wrong review id');
    
    // Check aggregates
    let (avg_times_100, count) = dispatcher.get_average(dapp_id);
    assert(avg_times_100 == 400, 'Wrong average'); // 4.00 * 100
    assert(count == 1, 'Wrong count');
    
    // Check review
    let reviews = dispatcher.list_reviews(dapp_id, 0, 10);
    assert(reviews.len() == 1, 'Wrong count');
    assert(*reviews.at(0).dapp_id == dapp_id, 'Wrong dapp id');
    assert(*reviews.at(0).reviewer == reviewer, 'Wrong reviewer');
    assert(*reviews.at(0).stars == stars, 'Wrong stars');
    assert(*reviews.at(0).review_cid.value == 'review', 'Wrong review cid');
    
    // Verify event emission
    let expected_event = dongle::ratings::ratings::Ratings::Event::ReviewAdded(
        ReviewAdded { review_id: 1, dapp_id, reviewer, stars, review_cid }
    );
    let expected_events = array![(dispatcher.contract_address, expected_event)];
    spy.assert_emitted(@expected_events);
    
    stop_cheat_caller_address(dispatcher.contract_address);
}

#[test]
fn test_add_second_review_same_user() {
    let dispatcher = deploy_ratings();
    let mut spy = spy_events();
    
    let reviewer: ContractAddress = contract_address_const::<'reviewer'>();
    start_cheat_caller_address(dispatcher.contract_address, reviewer);
    
    let dapp_id = 1_u32;
    let stars1 = 4_u8;
    let stars2 = 5_u8;
    let review_cid1 = cid_from_parts(array!['review1']);
    let review_cid2 = cid_from_parts(array!['review2']);
    
    // Add first review
    dispatcher.add_review(dapp_id, stars1, review_cid1);
    
    // Add second review from same user (should replace)
    let review_id2 = dispatcher.add_review(dapp_id, stars2, review_cid2);
    assert(review_id2 == 2, 'Wrong review id');
    
    // Check aggregates - should reflect new stars, same count
    let (avg_times_100, count) = dispatcher.get_average(dapp_id);
    assert(avg_times_100 == 500, 'Wrong average'); // 5.00 * 100
    assert(count == 1, 'Wrong count');
    
    // Check reviews - should have 2 reviews (append-only history)
    let reviews = dispatcher.list_reviews(dapp_id, 0, 10);
    assert(reviews.len() == 2, 'Wrong count');
    assert(*reviews.at(0).stars == stars1, 'Wrong first stars');
    assert(*reviews.at(1).stars == stars2, 'Wrong second stars');
    
    stop_cheat_caller_address(dispatcher.contract_address);
}

#[test]
fn test_add_reviews_different_users() {
    let dispatcher = deploy_ratings();
    
    let dapp_id = 1_u32;
    let review_cid = cid_from_parts(array!['review']);
    
    // First reviewer
    let reviewer1: ContractAddress = contract_address_const::<'reviewer1'>();
    start_cheat_caller_address(dispatcher.contract_address, reviewer1);
    dispatcher.add_review(dapp_id, 3_u8, review_cid);
    stop_cheat_caller_address(dispatcher.contract_address);
    
    // Second reviewer
    let reviewer2: ContractAddress = contract_address_const::<'reviewer2'>();
    start_cheat_caller_address(dispatcher.contract_address, reviewer2);
    dispatcher.add_review(dapp_id, 5_u8, review_cid);
    stop_cheat_caller_address(dispatcher.contract_address);
    
    // Check aggregates
    let (avg_times_100, count) = dispatcher.get_average(dapp_id);
    assert(avg_times_100 == 400, 'Wrong average'); // (3+5)/2 * 100 = 400
    assert(count == 2, 'Wrong count');
    
    // Check reviews
    let reviews = dispatcher.list_reviews(dapp_id, 0, 10);
    assert(reviews.len() == 2, 'Wrong count');
    assert(*reviews.at(0).reviewer == reviewer1, 'Wrong first reviewer');
    assert(*reviews.at(1).reviewer == reviewer2, 'Wrong second reviewer');
}

#[test]
fn test_update_review() {
    let dispatcher = deploy_ratings();
    let mut spy = spy_events();
    
    let reviewer: ContractAddress = contract_address_const::<'reviewer'>();
    start_cheat_caller_address(dispatcher.contract_address, reviewer);
    
    let dapp_id = 1_u32;
    let stars1 = 3_u8;
    let stars2 = 4_u8;
    let review_cid1 = cid_from_parts(array!['review1']);
    let review_cid2 = cid_from_parts(array!['review2']);
    
    // Add review
    dispatcher.add_review(dapp_id, stars1, review_cid1);
    
    // Update review
    dispatcher.update_review(1, stars2, review_cid2);
    
    // Check aggregates
    let (avg_times_100, count) = dispatcher.get_average(dapp_id);
    assert(avg_times_100 == 400, 'Wrong average'); // 4.00 * 100
    assert(count == 1, 'Wrong count');
    
    // Check review
    let reviews = dispatcher.list_reviews(dapp_id, 0, 10);
    assert(*reviews.at(0).stars == stars2, 'Review not updated');
    assert(*reviews.at(0).review_cid.value == 'review2', 'CID not updated');
    
    // Verify event emission
    let expected_event = dongle::ratings::ratings::Ratings::Event::ReviewUpdated(
        ReviewUpdated { review_id: 1, dapp_id, reviewer, stars: stars2, review_cid: review_cid2 }
    );
    let expected_events = array![(dispatcher.contract_address, expected_event)];
    spy.assert_emitted(@expected_events);
    
    stop_cheat_caller_address(dispatcher.contract_address);
}

#[test]
fn test_get_average_no_reviews() {
    let dispatcher = deploy_ratings();
    
    let dapp_id = 1_u32;
    let (avg_times_100, count) = dispatcher.get_average(dapp_id);
    
    assert(avg_times_100 == 0, 'Average should be 0');
    assert(count == 0, 'Count should be 0');
}

#[test]
fn test_list_reviews_pagination() {
    let dispatcher = deploy_ratings();
    
    let dapp_id = 1_u32;
    let review_cid = cid_from_parts(array!['review']);
    
    // Add multiple reviews
    let reviewer1: ContractAddress = contract_address_const::<'reviewer1'>();
    let reviewer2: ContractAddress = contract_address_const::<'reviewer2'>();
    let reviewer3: ContractAddress = contract_address_const::<'reviewer3'>();
    let reviewer4: ContractAddress = contract_address_const::<'reviewer4'>();
    let reviewer5: ContractAddress = contract_address_const::<'reviewer5'>();
    
    start_cheat_caller_address(dispatcher.contract_address, reviewer1);
    dispatcher.add_review(dapp_id, 1_u8, review_cid);
    stop_cheat_caller_address(dispatcher.contract_address);
    
    start_cheat_caller_address(dispatcher.contract_address, reviewer2);
    dispatcher.add_review(dapp_id, 2_u8, review_cid);
    stop_cheat_caller_address(dispatcher.contract_address);
    
    start_cheat_caller_address(dispatcher.contract_address, reviewer3);
    dispatcher.add_review(dapp_id, 3_u8, review_cid);
    stop_cheat_caller_address(dispatcher.contract_address);
    
    start_cheat_caller_address(dispatcher.contract_address, reviewer4);
    dispatcher.add_review(dapp_id, 4_u8, review_cid);
    stop_cheat_caller_address(dispatcher.contract_address);
    
    start_cheat_caller_address(dispatcher.contract_address, reviewer5);
    dispatcher.add_review(dapp_id, 5_u8, review_cid);
    stop_cheat_caller_address(dispatcher.contract_address);
    
    // Test pagination
    let reviews_page1 = dispatcher.list_reviews(dapp_id, 0, 2);
    assert(reviews_page1.len() == 2, 'Wrong page 1 size');
    
    let reviews_page2 = dispatcher.list_reviews(dapp_id, 2, 2);
    assert(reviews_page2.len() == 2, 'Wrong page 2 size');
    
    let reviews_page3 = dispatcher.list_reviews(dapp_id, 4, 2);
    assert(reviews_page3.len() == 1, 'Wrong page 3 size');
}

#[test]
#[should_panic(expected: 'ERR_INVALID_STARS')]
fn test_add_review_invalid_stars() {
    let dispatcher = deploy_ratings();
    
    let reviewer: ContractAddress = contract_address_const::<'reviewer'>();
    start_cheat_caller_address(dispatcher.contract_address, reviewer);
    
    let dapp_id = 1_u32;
    let invalid_stars = 6_u8; // Should be 1-5
    let review_cid = cid_from_parts(array!['review']);
    
    dispatcher.add_review(dapp_id, invalid_stars, review_cid);
    
    stop_cheat_caller_address(dispatcher.contract_address);
}

#[test]
fn test_review_replacement_logic() {
    let dispatcher = deploy_ratings();
    
    let reviewer: ContractAddress = contract_address_const::<'reviewer'>();
    start_cheat_caller_address(dispatcher.contract_address, reviewer);
    
    let dapp_id = 1_u32;
    let review_cid = cid_from_parts(array!['review']);
    
    // Add first review with 3 stars
    dispatcher.add_review(dapp_id, 3_u8, review_cid);
    let (avg1, count1) = dispatcher.get_average(dapp_id);
    assert(avg1 == 300, 'First avg wrong');
    assert(count1 == 1, 'First count wrong');
    
    // Add second review with 5 stars (should replace)
    dispatcher.add_review(dapp_id, 5_u8, review_cid);
    let (avg2, count2) = dispatcher.get_average(dapp_id);
    assert(avg2 == 500, 'Second avg wrong');
    assert(count2 == 1, 'Second count wrong');
    
    // Add third review with 1 star (should replace)
    dispatcher.add_review(dapp_id, 1_u8, review_cid);
    let (avg3, count3) = dispatcher.get_average(dapp_id);
    assert(avg3 == 100, 'Third avg wrong');
    assert(count3 == 1, 'Third count wrong');
    
    stop_cheat_caller_address(dispatcher.contract_address);
}

#[test]
fn test_multiple_dapps_reviews() {
    let dispatcher = deploy_ratings();
    
    let dapp1_id = 1_u32;
    let dapp2_id = 2_u32;
    let review_cid = cid_from_parts(array!['review']);
    
    // Add reviews for dapp 1
    let reviewer1: ContractAddress = contract_address_const::<'reviewer1'>();
    start_cheat_caller_address(dispatcher.contract_address, reviewer1);
    dispatcher.add_review(dapp1_id, 4_u8, review_cid);
    stop_cheat_caller_address(dispatcher.contract_address);
    
    // Add reviews for dapp 2
    let reviewer2: ContractAddress = contract_address_const::<'reviewer2'>();
    start_cheat_caller_address(dispatcher.contract_address, reviewer2);
    dispatcher.add_review(dapp2_id, 5_u8, review_cid);
    stop_cheat_caller_address(dispatcher.contract_address);
    
    // Check dapp 1
    let (avg1, count1) = dispatcher.get_average(dapp1_id);
    assert(avg1 == 400, 'Dapp 1 avg wrong');
    assert(count1 == 1, 'Dapp 1 count wrong');
    
    // Check dapp 2
    let (avg2, count2) = dispatcher.get_average(dapp2_id);
    assert(avg2 == 500, 'Dapp 2 avg wrong');
    assert(count2 == 1, 'Dapp 2 count wrong');
    
    // Check that reviews are separate
    let reviews1 = dispatcher.list_reviews(dapp1_id, 0, 10);
    let reviews2 = dispatcher.list_reviews(dapp2_id, 0, 10);
    
    assert(reviews1.len() == 1, 'Dapp 1 wrong count');
    assert(reviews2.len() == 1, 'Dapp 2 wrong count');
    assert(*reviews1.at(0).dapp_id == dapp1_id, 'Dapp 1 wrong id');
    assert(*reviews2.at(0).dapp_id == dapp2_id, 'Dapp 2 wrong id');
} 
