//! Boundaries — cases 13 to 16 of the mandatory sixteen.

use soroban_sdk::testutils::storage::{Instance as _, Persistent as _};

use crate::test::harness::{units, Fixture, HOUR};
use crate::types::{DataKey, BUMP_AMOUNT};
use crate::Error;

/// Case 13.
#[test]
fn a_zero_or_negative_amount_is_refused() {
    let f = Fixture::new();
    f.env.mock_all_auths();

    for amount in [0i128, -1i128] {
        let attempt = f.client.try_create_request(
            &f.creator,
            &f.payee,
            &f.id(21),
            &f.asset,
            &amount,
            &f.hash(0xAA),
            &(f.now() + HOUR),
        );
        assert_err!(attempt, Error::InvalidAmount);
    }
}

/// Case 14.
#[test]
fn an_expiry_in_the_past_is_refused() {
    let f = Fixture::new();
    f.env.mock_all_auths();

    // Strictly in the past.
    let past = f.client.try_create_request(
        &f.creator,
        &f.payee,
        &f.id(22),
        &f.asset,
        &units(1),
        &f.hash(0xAA),
        &(f.now() - 1),
    );
    assert_err!(past, Error::InvalidExpiry);

    // Exactly now is also refused: an expiry equal to the current ledger time gives a payment no
    // window at all.
    let now = f.client.try_create_request(
        &f.creator,
        &f.payee,
        &f.id(23),
        &f.asset,
        &units(1),
        &f.hash(0xAA),
        &f.now(),
    );
    assert_err!(now, Error::InvalidExpiry);
}

/// Case 15 — the boundary that decides whether the comparison is `>=` or `>`.
///
/// The specification says a payment landing exactly on the expiry second is late. This test is
/// what keeps it that way through a refactor.
#[test]
fn a_payment_landing_on_the_expiry_second_is_refused() {
    let f = Fixture::new();
    let id = f.id(24);
    let expiry = f.now() + HOUR;

    f.create_expiring(&id, units(1_000), expiry);
    f.env.mock_all_auths();
    f.client.approve(&id);

    // One second before expiry: still fine.
    f.set_time(expiry - 1);
    assert!(f.client.is_payable(&id));

    // Exactly on expiry: late.
    f.set_time(expiry);
    assert!(!f.client.is_payable(&id));
    assert_err!(
        f.client.try_mark_paid(&id, &units(1_000), &f.hash(0xE1)),
        Error::Expired
    );
}

/// Case 16 — module A8.
///
/// The failure this covers is invisible until it is catastrophic. If instance TTL is not extended,
/// every function in the contract stops working at once, some weeks after deployment, and no test
/// that runs inside a single ledger will ever notice.
#[test]
fn an_entry_survives_across_ledgers_after_its_ttl_is_extended() {
    let f = Fixture::new();
    let id = f.id(25);

    f.create(&id, units(42));

    // Both the request entry and instance storage were extended by the write.
    let (request_ttl, instance_ttl) = f.env.as_contract(&f.contract_id, || {
        (
            f.env
                .storage()
                .persistent()
                .get_ttl(&DataKey::Req(id.clone())),
            f.env.storage().instance().get_ttl(),
        )
    });
    assert!(
        request_ttl >= BUMP_AMOUNT - 1,
        "request entry TTL was {request_ttl}, expected at least {}",
        BUMP_AMOUNT - 1
    );
    assert!(
        instance_ttl >= BUMP_AMOUNT - 1,
        "instance TTL was {instance_ttl}, expected at least {}",
        BUMP_AMOUNT - 1
    );

    // Move well beyond the default entry lifetime but inside the extension we asked for.
    f.advance_ledgers(BUMP_AMOUNT / 2);

    // Both the request and an instance-storage-dependent function still work.
    assert_eq!(f.client.get_request(&id).amount, units(42));
    assert_eq!(f.client.get_settler(), f.settler);

    // A read-only path does not extend anything, so a write is what keeps an entry alive.
    f.env.mock_all_auths();
    f.client.approve(&id);
    let after = f.env.as_contract(&f.contract_id, || {
        f.env
            .storage()
            .persistent()
            .get_ttl(&DataKey::Req(id.clone()))
    });
    assert!(
        after >= BUMP_AMOUNT - 1,
        "approve did not re-extend the TTL"
    );
}
