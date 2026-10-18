//! Lifecycle — cases 1 to 5 of the mandatory sixteen.
//!
//! `pending → approved → paid`, plus the cancelled branch, plus expiry.

use crate::test::harness::{units, Fixture, HOUR};
use crate::ReqStatus;

/// Case 1.
#[test]
fn a_created_request_is_pending() {
    let f = Fixture::new();
    let id = f.id(1);

    f.create(&id, units(4_500));

    let req = f.client.get_request(&id);
    assert_eq!(req.status, ReqStatus::Pending);
    assert_eq!(req.amount, units(4_500));
    assert_eq!(req.payee, f.payee);
    assert_eq!(req.creator, f.creator);
    assert_eq!(req.paid_tx, None);
    assert!(f.client.is_payable(&id));
}

/// Case 2.
#[test]
fn the_settler_moves_a_pending_request_to_approved() {
    let f = Fixture::new();
    let id = f.id(2);

    f.create(&id, units(1_200));
    f.env.mock_all_auths();
    f.client.approve(&id);

    assert_eq!(f.client.get_request(&id).status, ReqStatus::Approved);
    // An approved request is still payable — that is the whole point of approving it.
    assert!(f.client.is_payable(&id));
}

/// Case 3.
#[test]
fn an_approved_request_becomes_paid_with_its_transaction_hash() {
    let f = Fixture::new();
    let id = f.id(3);
    let tx = f.hash(0xC1);

    f.create(&id, units(890));
    f.env.mock_all_auths();
    f.client.approve(&id);
    f.client.mark_paid(&id, &units(890), &tx);

    let req = f.client.get_request(&id);
    assert_eq!(req.status, ReqStatus::Paid);
    assert_eq!(req.paid_tx, Some(tx));
    // Paid is terminal, so the link is spent.
    assert!(!f.client.is_payable(&id));
}

/// Case 4.
#[test]
fn the_creator_can_cancel_a_request_nobody_has_paid() {
    let f = Fixture::new();
    let id = f.id(4);

    f.create(&id, units(3_000));
    f.env.mock_all_auths();
    f.client.cancel_request(&id);

    assert_eq!(f.client.get_request(&id).status, ReqStatus::Cancelled);
    assert!(!f.client.is_payable(&id));
}

/// Case 5.
#[test]
fn an_expired_request_is_not_payable() {
    let f = Fixture::new();
    let id = f.id(5);

    f.create(&id, units(100));
    assert!(f.client.is_payable(&id));

    f.advance_time(HOUR + 1);

    assert!(!f.client.is_payable(&id));
    // The stored status is untouched: the contract has no Expired status, it compares the ledger
    // clock on every call. Materialising `expired` is the backend's job.
    assert_eq!(f.client.get_request(&id).status, ReqStatus::Pending);
}

/// An approved-but-unpaid request is exactly the thing a business most needs to withdraw.
#[test]
fn an_approved_request_can_still_be_cancelled() {
    let f = Fixture::new();
    let id = f.id(6);

    f.create(&id, units(500));
    f.env.mock_all_auths();
    f.client.approve(&id);
    f.client.cancel_request(&id);

    assert_eq!(f.client.get_request(&id).status, ReqStatus::Cancelled);
}
