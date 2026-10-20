//! Tests for premium "featured plus" placement tiers (#928).

use crate::constants::PREMIUM_FEATURED_DAY_SECONDS;
use crate::errors::ContractError;
use crate::tests::fixtures::{create_test_project, setup_contract};
use crate::types::FeaturedTier;
use crate::DongleContractClient;
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::{token, Address, Env};

const DAY: u64 = PREMIUM_FEATURED_DAY_SECONDS;

fn setup(env: &Env) -> (DongleContractClient<'_>, Address, Address, Address) {
    env.mock_all_auths();
    let (client, admin) = setup_contract(env);
    let token = env
        .register_stellar_asset_contract_v2(Address::generate(env))
        .address();
    let treasury = Address::generate(env);
    client.set_fee(&admin, &Some(token.clone()), &0u128, &0u128, &treasury);
    client.set_featured_tier_config(&admin, &FeaturedTier::Standard, &10u128, &2u32);
    client.set_featured_tier_config(&admin, &FeaturedTier::Premium, &50u128, &2u32);
    client.set_featured_tier_config(&admin, &FeaturedTier::Platinum, &100u128, &1u32);
    (client, admin, token, treasury)
}

fn funded_project(
    env: &Env,
    client: &DongleContractClient<'_>,
    token: &Address,
    name: &str,
) -> (Address, u64) {
    let owner = Address::generate(env);
    token::StellarAssetClient::new(env, token).mint(&owner, &1_000_000);
    let id = create_test_project(client, &owner, name);
    (owner, id)
}

#[test]
fn test_tier_config_admin_only() {
    let env = Env::default();
    let (client, _admin, _token, _treasury) = setup(&env);
    let stranger = Address::generate(&env);
    assert_eq!(
        client.try_set_featured_tier_config(&stranger, &FeaturedTier::Premium, &1u128, &1u32),
        Err(Ok(ContractError::AdminOnly))
    );
}

#[test]
fn test_quote_charges_per_started_day() {
    let env = Env::default();
    let (client, _admin, _token, _treasury) = setup(&env);
    assert_eq!(client.quote_premium_featured(&FeaturedTier::Premium, &DAY), 50);
    assert_eq!(
        client.quote_premium_featured(&FeaturedTier::Premium, &(DAY + 1)),
        100
    );
    assert_eq!(
        client.try_quote_premium_featured(&FeaturedTier::Premium, &(DAY - 1)),
        Err(Ok(ContractError::InvalidPremiumDuration))
    );
}

#[test]
fn test_bid_transfers_to_treasury_and_tracks_stats() {
    let env = Env::default();
    let (client, _admin, token, treasury) = setup(&env);
    let (owner, id) = funded_project(&env, &client, &token, "Alpha");

    let campaign_id =
        client.bid_for_premium_featured(&owner, &id, &FeaturedTier::Premium, &(2 * DAY), &150u128);

    let campaign = client.get_premium_campaign(&campaign_id).unwrap();
    assert_eq!(campaign.project_id, id);
    assert_eq!(campaign.bid_amount, 150);
    assert_eq!(campaign.ends_at - campaign.starts_at, 2 * DAY);
    assert_eq!(token::Client::new(&env, &token).balance(&treasury), 150);

    let stats = client.get_premium_featured_stats();
    assert_eq!(stats.successful_campaigns, 1);
    assert_eq!(stats.total_bid_amount, 150);
    assert_eq!(stats.highest_bid, 150);
    assert_eq!(client.get_project_premium_campaigns(&id).len(), 1);
}

#[test]
fn test_bid_below_fee_rejected() {
    let env = Env::default();
    let (client, _admin, token, _treasury) = setup(&env);
    let (owner, id) = funded_project(&env, &client, &token, "Alpha");
    assert_eq!(
        client.try_bid_for_premium_featured(&owner, &id, &FeaturedTier::Premium, &(2 * DAY), &99u128),
        Err(Ok(ContractError::PremiumBidTooLow))
    );
}

#[test]
fn test_only_owner_can_bid() {
    let env = Env::default();
    let (client, _admin, token, _treasury) = setup(&env);
    let (_owner, id) = funded_project(&env, &client, &token, "Alpha");
    let stranger = Address::generate(&env);
    assert_eq!(
        client.try_bid_for_premium_featured(&stranger, &id, &FeaturedTier::Standard, &DAY, &10u128),
        Err(Ok(ContractError::Unauthorized))
    );
}

#[test]
fn test_full_tier_rejects_until_slot_expires() {
    let env = Env::default();
    let (client, _admin, token, _treasury) = setup(&env);
    let (a_owner, a) = funded_project(&env, &client, &token, "Alpha");
    let (b_owner, b) = funded_project(&env, &client, &token, "Beta");

    client.bid_for_premium_featured(&a_owner, &a, &FeaturedTier::Platinum, &DAY, &100u128);

    // A higher bid cannot displace an active campaign: exposure is guaranteed.
    assert_eq!(
        client.try_bid_for_premium_featured(&b_owner, &b, &FeaturedTier::Platinum, &DAY, &10_000u128),
        Err(Ok(ContractError::PremiumTierFull))
    );

    env.ledger().with_mut(|l| l.timestamp += DAY);
    client.bid_for_premium_featured(&b_owner, &b, &FeaturedTier::Platinum, &DAY, &100u128);

    let listed = client.get_premium_featured_projects(&Some(FeaturedTier::Platinum), &0, &10);
    assert_eq!(listed.len(), 1);
    assert_eq!(listed.get(0).unwrap().project_id, b);
}

#[test]
fn test_project_cannot_hold_two_active_campaigns() {
    let env = Env::default();
    let (client, _admin, token, _treasury) = setup(&env);
    let (owner, id) = funded_project(&env, &client, &token, "Alpha");
    client.bid_for_premium_featured(&owner, &id, &FeaturedTier::Standard, &DAY, &10u128);
    assert_eq!(
        client.try_bid_for_premium_featured(&owner, &id, &FeaturedTier::Premium, &DAY, &50u128),
        Err(Ok(ContractError::ProjectAlreadyPremiumFeatured))
    );
}

#[test]
fn test_listing_orders_by_tier_then_bid_and_filters() {
    let env = Env::default();
    let (client, _admin, token, _treasury) = setup(&env);
    let (s_owner, s) = funded_project(&env, &client, &token, "Standard One");
    let (p1_owner, p1) = funded_project(&env, &client, &token, "Premium One");
    let (p2_owner, p2) = funded_project(&env, &client, &token, "Premium Two");
    let (x_owner, x) = funded_project(&env, &client, &token, "Platinum One");

    client.bid_for_premium_featured(&s_owner, &s, &FeaturedTier::Standard, &DAY, &10u128);
    client.bid_for_premium_featured(&p1_owner, &p1, &FeaturedTier::Premium, &DAY, &50u128);
    client.bid_for_premium_featured(&p2_owner, &p2, &FeaturedTier::Premium, &DAY, &80u128);
    client.bid_for_premium_featured(&x_owner, &x, &FeaturedTier::Platinum, &DAY, &100u128);

    let all = client.get_premium_featured_projects(&None, &0, &10);
    assert_eq!(all.len(), 4);
    assert_eq!(all.get(0).unwrap().project_id, x);
    assert_eq!(all.get(1).unwrap().project_id, p2);
    assert_eq!(all.get(2).unwrap().project_id, p1);
    assert_eq!(all.get(3).unwrap().project_id, s);

    let premium = client.get_premium_featured_projects(&Some(FeaturedTier::Premium), &0, &10);
    assert_eq!(premium.len(), 2);

    env.ledger().with_mut(|l| l.timestamp += DAY);
    assert_eq!(client.get_premium_featured_projects(&None, &0, &10).len(), 0);
    // Expired campaigns stay in the project's history.
    assert_eq!(client.get_project_premium_campaigns(&p2).len(), 1);
}
