//! Constants, storage keys and data structures. **No logic.**
//!
//! Nine fields on `PaymentRequest`, and no more. Every additional field on chain is storage rent
//! nobody budgeted and a migration nobody planned. See `../../docs/04-contract-interface.md`.

use soroban_sdk::{contracttype, Address, BytesN};

/// TTL bump threshold, counted in **ledgers** — about 30 days IF a ledger closes every 5 seconds.
///
/// Ledger close time is a network parameter and it can fall. At 4 seconds this is roughly 24 days,
/// not 30. Recompute at build time rather than copying the number forward.
pub const BUMP_THRESHOLD: u32 = 518_400;

/// TTL extension, counted in **ledgers** — about 42 days at a 5 second close time.
pub const BUMP_AMOUNT: u32 = 725_760;

/// Storage keys, with their durability class.
///
/// `Admin` and `Settler` live in instance storage, which shares its TTL with the Wasm code: if it
/// expires, *every* function fails until a `RestoreFootprint` operation runs. That is why every
/// entry point extends the instance TTL, not only the ones that write.
#[contracttype]
pub enum DataKey {
    /// Instance · the administrative address.
    Admin,
    /// Instance · the ONLY address permitted to call `approve` or `mark_paid`.
    Settler,
    /// Persistent · one entry per request.
    Req(BytesN<32>),
}

/// The lifecycle the statement of work names directly: pending → approved → paid, plus the
/// cancelled branch.
#[contracttype]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u32)]
pub enum ReqStatus {
    /// Created, not yet approved.
    Pending = 0,
    /// Threshold met off chain; a payment transaction may now be built from it.
    Approved = 1,
    /// Terminal.
    Paid = 2,
    /// Terminal. The reject branch.
    Cancelled = 3,
}

/// A payment a business owes a vendor.
///
/// The contract records and verifies this. It never holds the asset named in `asset`, and there is
/// no function in this crate that can move one.
#[contracttype]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PaymentRequest {
    /// `sha256(business_id ‖ idempotency_key)`. The SAME value the backend stores in Postgres,
    /// which is what makes a retry after a timeout land on this entry instead of creating a second.
    pub id: BytesN<32>,

    /// The business that created it — the only party that may cancel it.
    pub creator: Address,

    /// THE VENDOR ADDRESS, per SOW Deliverable 1. The party being paid.
    pub payee: Address,

    /// The Stellar Asset Contract of USDC, or of native XLM.
    pub asset: Address,

    /// Stroops. 7 decimals. Never a float, at any layer, ever.
    pub amount: i128,

    /// A HASH of the reference. The plaintext — a purchase order or invoice number — stays in
    /// Postgres. Written to the ledger it would be permanently public and permanently linked to a
    /// vendor and an amount, which publishes the business's supplier relationships.
    pub reference: BytesN<32>,

    /// A LEDGER timestamp. Compared against `env.ledger().timestamp()` and nothing else, ever.
    pub expiry: u64,

    pub status: ReqStatus,

    /// Set exactly once, by `mark_paid`, after the backend has read the payment back from the
    /// ledger. Its presence is the single-use gate.
    pub paid_tx: Option<BytesN<32>>,
}
