//! The gates, as named functions.
//!
//! **Gate order is part of the specification, not an implementation detail.** Tests assert the
//! exact error code, so reordering two gates is a breaking change even when both still reject.
//! Naming each one keeps the order visible at the call site in `lib.rs`.
//!
//! Every time comparison here uses the LEDGER clock. `env.ledger().timestamp()` is the only clock
//! every validator agrees on; two machines with skewed system clocks reaching different conclusions
//! about whether a payment was on time is a dispute with no resolution procedure.

use soroban_sdk::Env;

use crate::errors::Error;
use crate::types::{PaymentRequest, ReqStatus};

/// An amount must be positive. Zero and negative have no business meaning.
pub fn amount_positive(amount: i128) -> Result<(), Error> {
    if amount <= 0 {
        return Err(Error::InvalidAmount);
    }
    Ok(())
}

/// An expiry must be in the future at creation, measured against the ledger.
pub fn expiry_in_future(env: &Env, expiry: u64) -> Result<(), Error> {
    if expiry <= env.ledger().timestamp() {
        return Err(Error::InvalidExpiry);
    }
    Ok(())
}

/// A request must not have lapsed.
///
/// `>=` rather than `>`: a payment landing exactly on the expiry second is late. Test case 15 in
/// `../../docs/11-testing.md` is what keeps it that way through a refactor.
pub fn not_expired(env: &Env, req: &PaymentRequest) -> Result<(), Error> {
    if env.ledger().timestamp() >= req.expiry {
        return Err(Error::Expired);
    }
    Ok(())
}

/// A transition that requires the request to still be awaiting approval.
pub fn is_pending(req: &PaymentRequest) -> Result<(), Error> {
    if req.status != ReqStatus::Pending {
        return Err(Error::NotPending);
    }
    Ok(())
}

/// A transition that is valid from either open status.
///
/// Used by `cancel_request`, so an approved-but-unpaid request can still be withdrawn — which is
/// exactly the thing a business most needs to be able to do.
pub fn is_open(req: &PaymentRequest) -> Result<(), Error> {
    match req.status {
        ReqStatus::Pending | ReqStatus::Approved => Ok(()),
        _ => Err(Error::NotPending),
    }
}

/// The single-use gate.
///
/// Checked BEFORE the status gate, deliberately. A second `mark_paid` on an already-paid request
/// must report `AlreadyPaid` — specific and actionable — rather than `NotApproved`, which would be
/// technically true and diagnostically useless.
pub fn not_already_paid(req: &PaymentRequest) -> Result<(), Error> {
    if req.paid_tx.is_some() {
        return Err(Error::AlreadyPaid);
    }
    Ok(())
}

/// `mark_paid` accepts `Pending` as well as `Approved`, so a single-authority configuration can
/// complete without a no-op `approve`. In the configuration being demonstrated, `approve` always
/// runs first.
pub fn is_payable_status(req: &PaymentRequest) -> Result<(), Error> {
    match req.status {
        ReqStatus::Pending | ReqStatus::Approved => Ok(()),
        _ => Err(Error::NotApproved),
    }
}

/// The exact-amount gate. Integer comparison on stroops; there is no tolerance parameter, because
/// a tolerance parameter is a value somebody eventually sets to something other than zero.
pub fn amount_matches(req: &PaymentRequest, amount: i128) -> Result<(), Error> {
    if amount != req.amount {
        return Err(Error::InvalidAmount);
    }
    Ok(())
}
