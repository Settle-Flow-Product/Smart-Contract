//! Abuse resistance — cases 6 to 12 of the mandatory sixteen.
//!
//! Cases 8, 9 and 11 are the authority matrix. Without them an authorisation regression is silent:
//! the contract still works for the legitimate caller, which is the only path anyone tests by hand.

use soroban_sdk::IntoVal;

use crate::test::harness::{units, units_plus, Fixture};
use crate::Error;

/// Case 6 — the on-chain half of duplicate protection.
#[test]
fn the_same_request_id_cannot_be_created_twice() {
    let f = Fixture::new();
    let id = f.id(11);

    f.create(&id, units(1_000));

    f.env.mock_all_auths();
    let again = f.client.try_create_request(
        &f.creator,
        &f.payee,
        &id,
        &f.asset,
        &units(1_000),
        &f.hash(0xAA),
        &(f.now() + 3_600),
    );
    assert_err!(again, Error::RequestExists);
}

/// Case 7.
#[test]
fn a_request_cannot_be_paid_twice() {
    let f = Fixture::new();
    let id = f.id(12);

    f.create(&id, units(700));
    f.env.mock_all_auths();
    f.client.approve(&id);
    f.client.mark_paid(&id, &units(700), &f.hash(0xD1));

    let again = f.client.try_mark_paid(&id, &units(700), &f.hash(0xD2));
    // AlreadyPaid, not NotApproved: the single-use gate is checked first on purpose, because a
    // repeat needs a specific and actionable answer.
    assert_err!(again, Error::AlreadyPaid);
}

/// Case 8 — authority. `require_auth` panics in the host; it does not return a code.
#[test]
#[should_panic]
fn only_the_settler_can_mark_a_request_paid() {
    let f = Fixture::new();
    let id = f.id(13);

    f.create(&id, units(300));
    f.env.mock_all_auths();
    f.client.approve(&id);

    // Authorise the stranger and nobody else. The contract requires the settler, finds no entry,
    // and the host panics.
    let stranger = f.stranger.clone();
    let args = (id.clone(), units(300), f.hash(0xD3)).into_val(&f.env);
    f.only_auth(&stranger, "mark_paid", args);

    f.client.mark_paid(&id, &units(300), &f.hash(0xD3));
}

/// Case 9 — authority.
#[test]
#[should_panic]
fn only_the_settler_can_approve_a_request() {
    let f = Fixture::new();
    let id = f.id(14);

    f.create(&id, units(200));

    let stranger = f.stranger.clone();
    let args = f.id_args(&id);
    f.only_auth(&stranger, "approve", args);

    f.client.approve(&id);
}

/// Case 10.
#[test]
fn a_request_cannot_be_approved_twice() {
    let f = Fixture::new();
    let id = f.id(15);

    f.create(&id, units(150));
    f.env.mock_all_auths();
    f.client.approve(&id);

    assert_err!(f.client.try_approve(&id), Error::NotPending);
}

/// Case 11 — authority. The creator is read from stored state, never from a parameter, so passing
/// your own address does not make you the creator.
#[test]
#[should_panic]
fn only_the_creator_can_cancel_a_request() {
    let f = Fixture::new();
    let id = f.id(16);

    f.create(&id, units(250));

    let stranger = f.stranger.clone();
    let args = f.id_args(&id);
    f.only_auth(&stranger, "cancel_request", args);

    f.client.cancel_request(&id);
}

/// Case 12 — the exact-amount gate.
#[test]
fn paying_a_different_amount_is_refused() {
    let f = Fixture::new();
    let id = f.id(17);

    f.create(&id, units(4_500));
    f.env.mock_all_auths();
    f.client.approve(&id);

    // One stroop short.
    assert_err!(
        f.client
            .try_mark_paid(&id, &units_plus(4_499, 9_999_999), &f.hash(0xD4)),
        Error::InvalidAmount
    );
    // One stroop over. An overpayment is not "close enough" either.
    assert_err!(
        f.client
            .try_mark_paid(&id, &units_plus(4_500, 1), &f.hash(0xD5)),
        Error::InvalidAmount
    );

    assert_eq!(f.client.get_request(&id).paid_tx, None);
}

/// A cancelled request is terminal and cannot be resurrected by paying it.
#[test]
fn a_cancelled_request_cannot_be_paid() {
    let f = Fixture::new();
    let id = f.id(18);

    f.create(&id, units(90));
    f.env.mock_all_auths();
    f.client.cancel_request(&id);

    assert_err!(
        f.client.try_mark_paid(&id, &units(90), &f.hash(0xD6)),
        Error::NotApproved
    );
}

/// A cancelled request is terminal in the other direction too.
#[test]
fn a_cancelled_request_cannot_be_approved() {
    let f = Fixture::new();
    let id = f.id(19);

    f.create(&id, units(80));
    f.env.mock_all_auths();
    f.client.cancel_request(&id);

    assert_err!(f.client.try_approve(&id), Error::NotPending);
}

/// A paid request is terminal: cancelling it afterwards must not rewrite history.
#[test]
fn a_paid_request_cannot_be_cancelled() {
    let f = Fixture::new();
    let id = f.id(20);

    f.create(&id, units(60));
    f.env.mock_all_auths();
    f.client.approve(&id);
    f.client.mark_paid(&id, &units(60), &f.hash(0xD7));

    assert_err!(f.client.try_cancel_request(&id), Error::NotPending);
}
