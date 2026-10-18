//! The frozen error table.
//!
//! Numbered in banded decades so a code's range names the subsystem that refused. **A code number
//! never changes after the first public testnet deployment**, and a retired code leaves its number
//! permanently vacant — never recycled. A backend that maps codes to operator actions cannot
//! survive renumbering.
//!
//! `Debug` is not cosmetic. Without it, `.unwrap()` on a `Result<_, Error>` does not compile in
//! tests, and the exact-code assertion style the test suite depends on becomes unavailable.

use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    // ---- System · 1–9 --------------------------------------------------------------------
    //
    // All three below are RESERVED AND UNREACHABLE, and that is a property worth having rather
    // than a gap. Configuration happens in `__constructor`, which the host runs inside the
    // deployment transaction — so there is no ledger state in which this contract exists and is
    // unconfigured. An uninitialised contract is unrepresentable, not merely guarded against.
    //
    // Their numbers stay permanently occupied under the rule above: a backend that maps codes to
    // operator actions cannot survive renumbering, so a number is never reused even when the
    // condition it named becomes impossible.
    /// Reserved. Unreachable: the constructor runs exactly once, by construction.
    AlreadyInitialized = 1,
    /// Reserved. Unreachable: there is no deployed-but-unconfigured state.
    NotInitialized = 2,
    /// Reserved. Unreachable: the constructor always sets the settler.
    SettlerNotSet = 3,
    // 4–9 vacant, reserved.

    // ---- Request · 10–19 -----------------------------------------------------------------
    /// `create_request` with an id already in persistent storage. This is on-chain idempotency:
    /// the id is derived, so a retry re-derives it and cannot produce a second request.
    RequestExists = 10,
    /// No entry for this id — it never existed. NOT the same as archived; an archived entry makes
    /// the transaction fail at apply time, before it reaches this contract.
    RequestNotFound = 11,
    /// `amount <= 0` at create, or an amount mismatch at `mark_paid`.
    InvalidAmount = 12,
    /// `expiry <= env.ledger().timestamp()` at create.
    InvalidExpiry = 13,
    // 14–19 vacant, reserved.

    // ---- Lifecycle · 20–29 ---------------------------------------------------------------
    /// A transition requiring `Pending` found another status.
    NotPending = 20,
    /// `mark_paid` on a request that is neither `Pending` nor `Approved`.
    NotApproved = 21,
    /// `paid_tx` is already set.
    AlreadyPaid = 22,
    /// `env.ledger().timestamp() >= expiry`.
    Expired = 23,
    // 24–29 vacant, reserved.

    // ---- Authority · 30–39 ---------------------------------------------------------------
    /// Reserved for an authority check the host does not already panic on.
    ///
    /// Currently unreachable: every authority check routes through `require_auth`, which panics in
    /// the host rather than returning a code. Listed in `../../docs/11-testing.md` as deliberately
    /// untested — an untested code with a written reason is a decision; one without is a hole.
    Unauthorized = 30,
    // 31–39 vacant, reserved.
}
