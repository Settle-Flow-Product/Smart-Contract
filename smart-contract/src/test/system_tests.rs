//! System-level cases: initialisation, and the codes the mandatory sixteen do not reach.
//!
//! Every code in `../../../docs/04-contract-interface.md` is reached by a test except
//! `Unauthorized`, which is deliberately unreachable — see the note at the bottom of this file.

use crate::test::harness::{units, Fixture, HOUR};
use crate::Error;

/// The constructor ran as part of the deployment, so the contract is configured from the moment
/// it exists.
#[test]
fn the_constructor_records_the_admin_and_the_settler() {
    let f = Fixture::new();
    assert_eq!(f.client.get_admin(), f.admin);
    assert_eq!(f.client.get_settler(), f.settler);
}

/// The contract is usable immediately, with no second setup call.
///
/// This is the property that closes the deployment window: a contract deployed in one transaction
/// and configured in another has a gap in which anyone may configure it. There is no such gap
/// here, and this test is what says so.
#[test]
fn the_contract_is_usable_the_moment_it_exists() {
    let f = Fixture::new();
    f.env.mock_all_auths();
    let id = f.id(31);

    f.client.create_request(
        &f.creator,
        &f.payee,
        &id,
        &f.asset,
        &units(5),
        &f.hash(0xAA),
        &(f.now() + HOUR),
    );
    f.client.approve(&id);

    assert_eq!(f.client.get_request(&id).amount, units(5));
}

#[test]
fn reading_a_request_that_never_existed_is_refused() {
    let f = Fixture::new();
    assert_err!(f.client.try_get_request(&f.id(33)), Error::RequestNotFound);
}

#[test]
fn a_request_that_never_existed_is_not_payable() {
    let f = Fixture::new();
    // `false`, not an error. The two are different questions and the backend asks both.
    assert!(!f.client.is_payable(&f.id(34)));
}

#[test]
fn approving_a_request_that_never_existed_is_refused() {
    let f = Fixture::new();
    f.env.mock_all_auths();
    assert_err!(f.client.try_approve(&f.id(35)), Error::RequestNotFound);
}

#[test]
fn cancelling_a_request_that_never_existed_is_refused() {
    let f = Fixture::new();
    f.env.mock_all_auths();
    assert_err!(
        f.client.try_cancel_request(&f.id(36)),
        Error::RequestNotFound
    );
}

#[test]
fn approving_after_expiry_is_refused() {
    let f = Fixture::new();
    let id = f.id(37);

    f.create(&id, units(10));
    f.advance_time(HOUR + 1);

    f.env.mock_all_auths();
    assert_err!(f.client.try_approve(&id), Error::Expired);
}

/// Four codes have no test, and each is a decision rather than a gap.
///
/// `AlreadyInitialized`, `NotInitialized` and `SettlerNotSet` describe a contract that exists
/// without being configured. The constructor runs inside the deployment transaction, so that state
/// is unrepresentable — the codes are unreachable **because the design improved**, not because
/// anybody forgot them.
///
/// `Unauthorized` is reserved for an authority check the host cannot express; today every one of
/// them routes through `require_auth`, which panics rather than returning a code.
///
/// Their numbers stay occupied. A backend that maps codes to operator actions cannot survive
/// renumbering, so a number is never reused even once the condition it named becomes impossible.
#[test]
fn the_reserved_codes_keep_their_numbers() {
    assert_eq!(Error::AlreadyInitialized as u32, 1);
    assert_eq!(Error::NotInitialized as u32, 2);
    assert_eq!(Error::SettlerNotSet as u32, 3);
    assert_eq!(Error::Unauthorized as u32, 30);
}
