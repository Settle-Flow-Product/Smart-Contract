//! The event catalogue. Four events; the first topic is always `"settle"`.
//!
//! These are the sole input to chain synchronisation (`../../docs/07-chain-sync.md`), so three
//! properties hold and keep holding:
//!
//! * **Every event names its request as a topic**, so an indexer filters without decoding data.
//! * **Fields never change after deployment.** An indexer in production is parsing them.
//! * **Emission happens after the state write**, never before — step 6 of the write discipline.
//!
//! Topic symbols are abbreviated to fit within nine characters, the `Symbol` short-form limit.
//! That limit surprises people at exactly the wrong moment, so the names are short by construction
//! here rather than truncated later.

use soroban_sdk::{contractevent, BytesN};

/// `create_request` succeeded. Topics: `settle`, `req_new`, id.
#[contractevent(topics = ["settle", "req_new"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RequestCreated {
    #[topic]
    pub id: BytesN<32>,
    pub amount: i128,
    pub expiry: u64,
}

/// `approve` succeeded. Topics: `settle`, `req_appr`, id.
#[contractevent(topics = ["settle", "req_appr"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RequestApproved {
    #[topic]
    pub id: BytesN<32>,
}

/// `mark_paid` succeeded. Topics: `settle`, `req_paid`, id.
///
/// Carries the settled transaction hash, which is what the evidence page links to.
#[contractevent(topics = ["settle", "req_paid"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RequestPaid {
    #[topic]
    pub id: BytesN<32>,
    pub amount: i128,
    pub paid_tx: BytesN<32>,
}

/// `cancel_request` succeeded — the reject branch. Topics: `settle`, `req_canc`, id.
#[contractevent(topics = ["settle", "req_canc"])]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RequestCancelled {
    #[topic]
    pub id: BytesN<32>,
}
