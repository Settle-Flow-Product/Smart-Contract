//! Typed storage accessors, split by durability class.
//!
//! Nothing outside this module touches `env.storage()` directly. Two reasons: every per-request
//! accessor takes the id, so there is no way to reach an entry without naming which one; and the
//! TTL extension lives beside the read, so it cannot be forgotten in one call site and remembered
//! in another.

use soroban_sdk::{Address, BytesN, Env};

use crate::errors::Error;
use crate::types::{DataKey, PaymentRequest, BUMP_AMOUNT, BUMP_THRESHOLD};

// ---------------------------------------------------------------------------
// Instance storage — market-wide, shares its TTL with the Wasm code
// ---------------------------------------------------------------------------

pub fn is_initialized(env: &Env) -> bool {
    env.storage().instance().has(&DataKey::Admin)
}

pub fn set_admin(env: &Env, admin: &Address) {
    env.storage().instance().set(&DataKey::Admin, admin);
}

pub fn admin(env: &Env) -> Result<Address, Error> {
    env.storage()
        .instance()
        .get(&DataKey::Admin)
        .ok_or(Error::NotInitialized)
}

pub fn set_settler(env: &Env, settler: &Address) {
    env.storage().instance().set(&DataKey::Settler, settler);
}

/// The only address permitted to call `approve` or `mark_paid`.
pub fn settler(env: &Env) -> Result<Address, Error> {
    env.storage()
        .instance()
        .get(&DataKey::Settler)
        .ok_or(Error::SettlerNotSet)
}

/// Extend the instance TTL.
///
/// Called from **every** entry point, including the ones that only touch a persistent entry. If
/// instance storage expires, every function in the contract fails at once — some weeks after
/// deployment, with no warning — until a `RestoreFootprint` operation runs.
pub fn bump_instance(env: &Env) {
    env.storage()
        .instance()
        .extend_ttl(BUMP_THRESHOLD, BUMP_AMOUNT);
}

// ---------------------------------------------------------------------------
// Persistent storage — one entry per request
// ---------------------------------------------------------------------------

pub fn has_request(env: &Env, id: &BytesN<32>) -> bool {
    env.storage().persistent().has(&DataKey::Req(id.clone()))
}

/// Read a request, or `RequestNotFound`.
///
/// `RequestNotFound` means the entry NEVER EXISTED. An archived entry does not reach this code
/// path at all: a transaction touching an archived key fails at apply time, before the contract
/// runs. The backend has to tell the two apart, because only the second is recoverable.
pub fn request(env: &Env, id: &BytesN<32>) -> Result<PaymentRequest, Error> {
    env.storage()
        .persistent()
        .get(&DataKey::Req(id.clone()))
        .ok_or(Error::RequestNotFound)
}

pub fn try_request(env: &Env, id: &BytesN<32>) -> Option<PaymentRequest> {
    env.storage().persistent().get(&DataKey::Req(id.clone()))
}

pub fn set_request(env: &Env, req: &PaymentRequest) {
    let key = DataKey::Req(req.id.clone());
    env.storage().persistent().set(&key, req);
    env.storage()
        .persistent()
        .extend_ttl(&key, BUMP_THRESHOLD, BUMP_AMOUNT);
}

/// Extend a request entry's TTL without writing it.
pub fn bump_request(env: &Env, id: &BytesN<32>) {
    let key = DataKey::Req(id.clone());
    if env.storage().persistent().has(&key) {
        env.storage()
            .persistent()
            .extend_ttl(&key, BUMP_THRESHOLD, BUMP_AMOUNT);
    }
}
