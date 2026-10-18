//! # payment_request_registry
//!
//! SettleFlow SOW 1, Deliverable 1. A registry that records and verifies the lifecycle of a
//! payment a business owes a vendor: `pending → approved → paid`, plus a `cancelled` branch.
//!
//! ## This contract never moves a token
//!
//! There is no `token::Client`, no `transfer`, and no call to a Stellar Asset Contract anywhere in
//! this crate. Its custody risk surface is zero, which is what makes twenty working days to testnet
//! an honest estimate rather than a hopeful one. CI greps for those symbols on every push and fails
//! the build on a hit — it is the first item on the Day 4 review checklist.
//!
//! ## The write discipline
//!
//! Every mutating entry point follows the same order. Deviating from it is how a contract emits an
//! event for a change it then fails to make.
//!
//! ```text
//!  1  require_auth on the correct role
//!  2  validate parameters
//!  3  load state
//!  4  guards, cheapest first
//!  5  apply changes
//!  6  emit the event
//!  7  extend TTL                 always last, always bounded
//! ```
//!
//! ## Reading
//!
//! The specification is `../../docs/04-contract-interface.md`. Gate order is part of it: tests
//! assert exact error codes, so reordering two gates is a breaking change even when both still
//! reject.

#![no_std]

pub mod errors;
pub mod events;
pub mod invariant;
pub mod storage;
pub mod types;

#[cfg(test)]
mod test;

pub use errors::Error;
pub use types::{DataKey, PaymentRequest, ReqStatus, BUMP_AMOUNT, BUMP_THRESHOLD};

use soroban_sdk::{contract, contractimpl, Address, BytesN, Env};

#[contract]
pub struct Registry;

#[contractimpl]
impl Registry {
    /// The constructor. Runs inside the deployment transaction itself.
    ///
    /// **This is `__constructor`, not an `initialize` entry point, and the difference matters.**
    /// A contract deployed in one transaction and initialised in another has a window between the
    /// two in which anyone can call `initialize` and become its admin. The host invokes
    /// `__constructor` as part of `create_contract`, so that window does not exist: there is no
    /// ledger state in which this contract is deployed and unconfigured.
    ///
    /// It also means an uninitialised contract is unrepresentable rather than merely guarded
    /// against — which is why `AlreadyInitialized`, `NotInitialized` and `SettlerNotSet` are
    /// reserved-but-unreachable in `errors.rs` rather than live codes.
    ///
    /// `settler` is the only address that will ever be able to move a request past `Pending`.
    /// Which key that is — a dedicated backend key or the treasury account itself — is question 01
    /// in `../../docs/15-risks.md` and must be settled before Day 1.
    pub fn __constructor(env: Env, admin: Address, settler: Address) {
        admin.require_auth();

        storage::set_admin(&env, &admin);
        storage::set_settler(&env, &settler);
        storage::bump_instance(&env);
    }

    /// Record a payment owed to a vendor. Moves nothing.
    ///
    /// `id` is `sha256(business_id ‖ idempotency_key)`, derived rather than random. That is what
    /// makes gate 3 on-chain idempotency: a retry after a network timeout re-derives the same id,
    /// hits `RequestExists`, and cannot produce a second request. The backend treats that error as
    /// "already done" and reads the existing entry back — step S-11 of the flow.
    #[allow(clippy::too_many_arguments)]
    pub fn create_request(
        env: Env,
        creator: Address,
        payee: Address,
        id: BytesN<32>,
        asset: Address,
        amount: i128,
        reference: BytesN<32>,
        expiry: u64,
    ) -> Result<PaymentRequest, Error> {
        // 1 · a business creates only for itself
        creator.require_auth();

        // 2 · validate, in the order the specification names
        invariant::amount_positive(amount)?;
        invariant::expiry_in_future(&env, expiry)?;

        // 4 · on-chain idempotency
        if storage::has_request(&env, &id) {
            return Err(Error::RequestExists);
        }

        // 5 · apply
        let req = PaymentRequest {
            id: id.clone(),
            creator,
            payee,
            asset,
            amount,
            reference,
            expiry,
            status: ReqStatus::Pending,
            paid_tx: None,
        };
        storage::set_request(&env, &req);

        // 6 · emit
        events::RequestCreated {
            id: id.clone(),
            amount,
            expiry,
        }
        .publish(&env);

        // 7 · TTL last
        storage::bump_instance(&env);
        Ok(req)
    }

    /// `Pending → Approved`. The step the statement of work calls the budget approval gate.
    ///
    /// **The contract does not count signatures.** It trusts the settler, whose key is used only
    /// after the signature service has verified every signature against the pinned policy version
    /// and summed the weights. Putting weight arithmetic on chain would mean putting the signer
    /// roster on chain, which changes what a policy change costs and what it leaks.
    pub fn approve(env: Env, id: BytesN<32>) -> Result<(), Error> {
        // 1 · settler only
        let settler = storage::settler(&env)?;
        settler.require_auth();

        // 3 · load
        let mut req = storage::request(&env, &id)?;

        // 4 · guards, in specified order
        invariant::is_pending(&req)?;
        invariant::not_expired(&env, &req)?;

        // 5 · apply
        req.status = ReqStatus::Approved;
        storage::set_request(&env, &req);

        // 6 · emit
        events::RequestApproved { id: id.clone() }.publish(&env);

        // 7 · TTL last
        storage::bump_instance(&env);
        Ok(())
    }

    /// `Approved → Paid`, after the payment has settled on the network.
    ///
    /// **The backend calls this only after reading the settled transaction back from the ledger.**
    /// Calling it on the strength of a successful submit is the defect the whole design exists to
    /// prevent: a submit result means the network accepted an envelope, not that a payment
    /// happened.
    pub fn mark_paid(env: Env, id: BytesN<32>, amount: i128, tx: BytesN<32>) -> Result<(), Error> {
        // 1 · settler only — a business cannot mark its own request paid
        let settler = storage::settler(&env)?;
        settler.require_auth();

        // 3 · load
        let mut req = storage::request(&env, &id)?;

        // 4 · guards. Single-use FIRST, so a repeat reports AlreadyPaid rather than NotApproved.
        invariant::not_already_paid(&req)?;
        invariant::is_payable_status(&req)?;
        invariant::not_expired(&env, &req)?;
        invariant::amount_matches(&req, amount)?;

        // 5 · apply
        req.status = ReqStatus::Paid;
        req.paid_tx = Some(tx.clone());
        storage::set_request(&env, &req);

        // 6 · emit
        events::RequestPaid {
            id: id.clone(),
            amount,
            paid_tx: tx,
        }
        .publish(&env);

        // 7 · TTL last
        storage::bump_instance(&env);
        Ok(())
    }

    /// The reject branch. The business withdraws a payment nobody has made.
    ///
    /// Authority comes from STORED STATE, not from a parameter. Taking the creator as an argument
    /// would let anyone pass their own address, authorise as themselves, and cancel a request they
    /// do not own.
    pub fn cancel_request(env: Env, id: BytesN<32>) -> Result<(), Error> {
        // 3 · load first — the authority lives inside the record
        let mut req = storage::request(&env, &id)?;

        // 1 · authorise as the stored creator
        req.creator.require_auth();

        // 4 · cancellable from Pending OR Approved: an approved-but-unpaid payment is exactly the
        //     thing a business most needs to be able to withdraw.
        invariant::is_open(&req)?;

        // 5 · apply
        req.status = ReqStatus::Cancelled;
        storage::set_request(&env, &req);

        // 6 · emit
        events::RequestCancelled { id: id.clone() }.publish(&env);

        // 7 · TTL last
        storage::bump_instance(&env);
        Ok(())
    }

    /// Read a request.
    ///
    /// No authorisation: the data is not secret to anyone holding the 32-byte id, and the id is not
    /// guessable. Soroban has no view functions — this goes through `InvokeHostFunction` like
    /// everything else, and is free only when called off chain via `simulateTransaction` or
    /// `getLedgerEntries`. **Never submit it as a transaction.**
    pub fn get_request(env: Env, id: BytesN<32>) -> Result<PaymentRequest, Error> {
        storage::request(&env, &id)
    }

    /// The single question "can this still be paid", answered by the contract rather than inferred
    /// by the backend.
    ///
    /// Returns `false` for a request that never existed. It does **not** return `false` for an
    /// archived entry — that call aborts at apply time, before reaching this code. The backend has
    /// to tell the two cases apart.
    pub fn is_payable(env: Env, id: BytesN<32>) -> bool {
        match storage::try_request(&env, &id) {
            Some(req) => {
                matches!(req.status, ReqStatus::Pending | ReqStatus::Approved)
                    && env.ledger().timestamp() < req.expiry
                    && req.paid_tx.is_none()
            }
            None => false,
        }
    }

    /// The administrative address. Read-only.
    pub fn get_admin(env: Env) -> Result<Address, Error> {
        storage::admin(&env)
    }

    /// The settler address — the only party that may approve or mark paid. Read-only.
    pub fn get_settler(env: Env) -> Result<Address, Error> {
        storage::settler(&env)
    }
}
