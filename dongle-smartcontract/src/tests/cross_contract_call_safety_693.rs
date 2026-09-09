//! Tests for issue #693 — Cross-Contract Call Safety and State Consistency.
//!
//! `fee_manager.rs` is the only place this contract calls out to an
//! external contract — every call is a `soroban_sdk::token::Client`
//! transfer, at exactly three sites: `execute_fee_payment` (shared by
//! `pay_fee`/`pay_registration_fee`), `cancel_fee_payment`, and
//! `claim_fee_refund`. All three sites, their call ordering, the rollback
//! reasoning, and the external-contract trust assumption are catalogued in
//! `docs/REENTRANCY_ANALYSIS.md`'s "State consistency when the external
//! call itself fails (#693)" section — this file is the empirical check
//! for that document's central claim, since no prior test drove an actual
//! token-transfer failure through any of these three sites (existing
//! atomicity tests cover *validation* failures like insufficient
//! configured fee or invalid CID, not the external call itself failing).

#![cfg(test)]

use crate::storage_keys::{ExtensionKey, StorageKey};
use crate::tests::fixtures::{create_test_project, setup_contract};
use soroban_sdk::{testutils::Address as _, token, Address, Env, String};

const FEE: u128 = 100;

struct Fixture<'a> {
    client: crate::DongleContractClient<'a>,
    admin: Address,
    owner: Address,
    treasury: Address,
    token_address: Address,
    project_id: u64,
}

fn setup_project_with_fee(env: &Env) -> Fixture<'_> {
    let (client, admin) = setup_contract(env);
    let owner = Address::generate(env);
    let treasury = Address::generate(env);

    let token_admin = Address::generate(env);
    let token_address = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();

    client.set_fee(&admin, &Some(token_address.clone()), &FEE, &0u128, &treasury);
    let project_id = create_test_project(&client, &owner, "CrossContractSafetyProject");

    Fixture {
        client,
        admin,
        owner,
        treasury,
        token_address,
        project_id,
    }
}

/// `pay_fee`'s underlying token transfer fails (payer never minted enough)
/// — the whole call must fail, and it must leave no partial trace: no paid
/// flag, no payment record, no fee-paid event side effects observable via
/// the getters.
#[test]
fn pay_fee_leaves_no_partial_state_when_the_token_transfer_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let f = setup_project_with_fee(&env);

    // Owner has zero balance of the fee token — transfer must fail.
    let result = f.client.try_pay_fee(&f.owner, &f.project_id, &Some(f.token_address));
    assert!(
        result.is_err(),
        "pay_fee must fail when the underlying token transfer fails"
    );

    assert!(
        !f.client.is_fee_paid(&f.project_id),
        "paid flag must not be set when the transfer failed"
    );
    assert!(
        f.client.get_fee_payment_details(&f.project_id).is_none(),
        "no payment record must exist when the transfer failed"
    );
}

/// Same as above, but confirms it via a raw storage snapshot rather than
/// only the public getters, so a bug that set the flag under a different
/// key than the getter reads wouldn't hide behind the getter itself.
#[test]
fn pay_fee_storage_is_untouched_when_the_token_transfer_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let f = setup_project_with_fee(&env);
    let contract_id = f.client.address.clone();
    let project_id = f.project_id;

    let before = env.as_contract(&contract_id, || {
        (
            env.storage()
                .persistent()
                .get::<_, bool>(&StorageKey::FeePaidForProject(project_id)),
            env.storage()
                .persistent()
                .get::<_, crate::types::FeePaymentRecord>(&ExtensionKey::FeePaymentDetails(
                    project_id,
                )),
        )
    });

    let _ = f.client.try_pay_fee(&f.owner, &project_id, &Some(f.token_address));

    let after = env.as_contract(&contract_id, || {
        (
            env.storage()
                .persistent()
                .get::<_, bool>(&StorageKey::FeePaidForProject(project_id)),
            env.storage()
                .persistent()
                .get::<_, crate::types::FeePaymentRecord>(&ExtensionKey::FeePaymentDetails(
                    project_id,
                )),
        )
    });

    assert_eq!(before, after, "storage must be byte-for-byte unchanged after a failed transfer");
}

/// `claim_fee_refund`'s transfer fails because the treasury (the source of
/// the refund) never actually holds the tokens it claims to be able to pay
/// out. The refund record must remain unclaimed — not marked claimed with
/// no tokens moved — for the payer to retry once the treasury is funded.
#[test]
fn claim_fee_refund_does_not_mark_claimed_when_the_token_transfer_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let f = setup_project_with_fee(&env);

    // Fund the owner and pay the fee for real so a genuine refund exists.
    token::StellarAssetClient::new(&env, &f.token_address).mint(&f.owner, &1_000);
    f.client.pay_fee(&f.owner, &f.project_id, &Some(f.token_address.clone()));

    let cid = String::from_str(&env, "QmYwAPJhy5nTAQCj9g1s2bkss7jBlEd22bN2R4s5gR5PTa");
    f.client.request_verification(&f.project_id, &f.owner, &cid);
    f.client.reject_verification(&f.project_id, &f.admin);

    assert!(f.client.get_fee_refund(&f.project_id).is_some());

    // Drain the treasury back to zero before the payer claims, so the
    // refund's own transfer (treasury -> payer) fails for lack of funds —
    // the treasury received the fee via pay_fee above, then we sweep it out
    // to a throwaway address to simulate an underfunded treasury at claim
    // time.
    let sink = Address::generate(&env);
    let token_client = token::Client::new(&env, &f.token_address);
    let treasury_balance = token_client.balance(&f.treasury);
    token_client.transfer(&f.treasury, &sink, &treasury_balance);

    let result = f.client.try_claim_fee_refund(&f.owner, &f.project_id);
    assert!(
        result.is_err(),
        "claim_fee_refund must fail when the treasury can't cover the transfer"
    );

    let refund_after = f.client.get_fee_refund(&f.project_id).unwrap();
    assert!(
        refund_after.claimed_at.is_none(),
        "a refund whose payout transfer failed must remain unclaimed, not marked claimed"
    );
}

/// `cancel_fee_payment`'s refund transfer fails for the same reason —
/// confirms the payment record is not removed (i.e. the cancellation
/// itself is rolled back, not half-applied) when the treasury can't cover
/// the refund.
#[test]
fn cancel_fee_payment_does_not_remove_payment_record_when_the_refund_transfer_fails() {
    let env = Env::default();
    env.mock_all_auths();
    let f = setup_project_with_fee(&env);

    token::StellarAssetClient::new(&env, &f.token_address).mint(&f.owner, &1_000);
    f.client.pay_fee(&f.owner, &f.project_id, &Some(f.token_address.clone()));
    assert!(f.client.is_fee_paid(&f.project_id));

    // Drain the treasury so the refund-on-cancel transfer fails.
    let sink = Address::generate(&env);
    let token_client = token::Client::new(&env, &f.token_address);
    let treasury_balance = token_client.balance(&f.treasury);
    token_client.transfer(&f.treasury, &sink, &treasury_balance);

    let result = f.client.try_cancel_fee_payment(&f.owner, &f.project_id);
    assert!(
        result.is_err(),
        "cancel_fee_payment must fail when the treasury can't cover the refund"
    );

    assert!(
        f.client.is_fee_paid(&f.project_id),
        "a cancellation whose refund transfer failed must not remove the payment record — \
         the whole invocation (including the earlier storage.remove calls) must roll back atomically"
    );
    assert!(f.client.get_fee_payment_details(&f.project_id).is_some());
}

/// Documents which contract errors indicate a rejected cross-contract call
/// vs. an application-level validation failure, so a future reader doesn't
/// have to re-derive it: none of the three failures above surface as
/// ContractError::ArithmeticOverflow or any bespoke "transfer failed" code
/// — the token contract's panic propagates as a generic host error at the
/// client boundary, which is why the tests above assert `is_err()` rather
/// than matching a specific ContractError variant.
#[test]
fn token_transfer_failures_surface_as_a_generic_host_error_not_a_contract_error_variant() {
    let env = Env::default();
    env.mock_all_auths();
    let f = setup_project_with_fee(&env);

    let result = f.client.try_pay_fee(&f.owner, &f.project_id, &Some(f.token_address));
    match result {
        Err(Ok(_contract_error)) => {
            panic!("expected a host-level invoke error, not a typed ContractError, for a raw token transfer failure")
        }
        Err(Err(_invoke_error)) => {
            // Expected: the token contract's panic surfaces as an
            // Err(Err(_)) (host/invoke error) rather than Err(Ok(ContractError::_)).
        }
        Ok(_) => panic!("expected pay_fee to fail with zero balance"),
    }
}
