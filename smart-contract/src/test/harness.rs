//! The fixture every test builds on.
//!
//! One contract, four addresses, and a set of helpers that make the *intent* of a test visible
//! rather than its plumbing.

#![allow(dead_code)]

use soroban_sdk::testutils::{Address as _, Ledger, MockAuth, MockAuthInvoke};
use soroban_sdk::{Address, BytesN, Env, IntoVal};

use crate::{Registry, RegistryClient};

/// One hour, in seconds. A comfortable default expiry for a request under test.
pub const HOUR: u64 = 3_600;

/// Stroops per whole unit. Stellar uses seven decimal places for every asset.
pub const STROOPS: i128 = 10_000_000;

/// Whole units expressed in stroops.
///
/// Writing `units(4_500)` rather than `45_000_000_000` states the seven decimals once instead of
/// asking every reader to count zeros — and it keeps the arithmetic in integers, where all money
/// in this system lives.
pub const fn units(whole: i128) -> i128 {
    whole * STROOPS
}

/// Whole units plus a stroop offset, for the exact-amount boundary tests.
pub const fn units_plus(whole: i128, stroops: i128) -> i128 {
    whole * STROOPS + stroops
}

/// The ledger timestamp every fixture starts at. A round non-zero number, so a test that
/// accidentally compares against 0 fails visibly.
pub const START_TIME: u64 = 1_700_000_000;

pub struct Fixture<'a> {
    pub env: Env,
    pub contract_id: Address,
    pub client: RegistryClient<'a>,
    /// The administrative address.
    pub admin: Address,
    /// The only address permitted to approve or mark paid.
    pub settler: Address,
    /// The business creating requests.
    pub creator: Address,
    /// The vendor being paid.
    pub payee: Address,
    /// A Stellar Asset Contract address. The registry never calls it — it only records it.
    pub asset: Address,
    /// Somebody with no role at all, used to prove the authority matrix.
    pub stranger: Address,
}

impl<'a> Fixture<'a> {
    /// A deployed registry.
    ///
    /// Configuration happens in the constructor, inside the registration itself — the same thing
    /// `stellar contract deploy … -- --admin … --settler …` does on a real network. There is no
    /// `uninitialised()` counterpart because there is no such state to build.
    pub fn new() -> Fixture<'a> {
        let env = Env::default();
        env.ledger().with_mut(|li| li.timestamp = START_TIME);

        let admin = Address::generate(&env);
        let settler = Address::generate(&env);
        let creator = Address::generate(&env);
        let payee = Address::generate(&env);
        let asset = Address::generate(&env);
        let stranger = Address::generate(&env);

        env.mock_all_auths();
        let contract_id = env.register(Registry, (admin.clone(), settler.clone()));
        let client = RegistryClient::new(&env, &contract_id);

        Fixture {
            env,
            contract_id,
            client,
            admin,
            settler,
            creator,
            payee,
            asset,
            stranger,
        }
    }

    /// A deterministic 32-byte identifier, standing in for
    /// `sha256(business_id ‖ idempotency_key)`.
    pub fn id(&self, seed: u8) -> BytesN<32> {
        BytesN::from_array(&self.env, &[seed; 32])
    }

    /// A deterministic 32-byte value, standing in for a reference hash or a transaction hash.
    pub fn hash(&self, seed: u8) -> BytesN<32> {
        BytesN::from_array(&self.env, &[seed; 32])
    }

    /// The current ledger timestamp.
    pub fn now(&self) -> u64 {
        self.env.ledger().timestamp()
    }

    /// Move the ledger clock forward.
    pub fn advance_time(&self, seconds: u64) {
        self.set_time(self.now() + seconds);
    }

    /// Put the ledger clock at an exact timestamp.
    pub fn set_time(&self, timestamp: u64) {
        self.env.ledger().with_mut(|li| li.timestamp = timestamp);
    }

    /// Move the ledger sequence forward, for TTL behaviour.
    pub fn advance_ledgers(&self, ledgers: u32) {
        self.env
            .ledger()
            .with_mut(|li| li.sequence_number += ledgers);
    }

    /// Create a request with sensible defaults, expiring one hour from now.
    pub fn create(&self, id: &BytesN<32>, amount: i128) {
        self.env.mock_all_auths();
        self.client.create_request(
            &self.creator,
            &self.payee,
            id,
            &self.asset,
            &amount,
            &self.hash(0xAA),
            &(self.now() + HOUR),
        );
    }

    /// Create a request with an explicit expiry.
    pub fn create_expiring(&self, id: &BytesN<32>, amount: i128, expiry: u64) {
        self.env.mock_all_auths();
        self.client.create_request(
            &self.creator,
            &self.payee,
            id,
            &self.asset,
            &amount,
            &self.hash(0xAA),
            &expiry,
        );
    }

    /// Restrict authorisation to exactly one address for exactly one call.
    ///
    /// This is how the authority matrix is tested: mock the auth for the WRONG address, then make
    /// the call and assert it panics because the required `require_auth` had no entry.
    pub fn only_auth(
        &self,
        address: &'a Address,
        fn_name: &'a str,
        args: soroban_sdk::Vec<soroban_sdk::Val>,
    ) {
        self.env.mock_auths(&[MockAuth {
            address,
            invoke: &MockAuthInvoke {
                contract: &self.contract_id,
                fn_name,
                args,
                sub_invokes: &[],
            },
        }]);
    }

    /// Arguments for a one-parameter entry point taking a request id.
    pub fn id_args(&self, id: &BytesN<32>) -> soroban_sdk::Vec<soroban_sdk::Val> {
        (id.clone(),).into_val(&self.env)
    }
}

/// Assert that a client call failed with an exact contract error code.
///
/// The generated `try_` client returns `Result<Result<T, ConversionError>, Result<Error, InvokeError>>`.
/// The nested shape is noise at a call site, so it is unwrapped here once.
#[macro_export]
macro_rules! assert_err {
    ($result:expr, $expected:expr) => {{
        let outcome = $result;
        match outcome {
            Err(Ok(e)) => assert_eq!(e, $expected, "wrong error code"),
            Err(Err(e)) => panic!("expected {:?}, got an invoke error: {:?}", $expected, e),
            Ok(_) => panic!("expected {:?}, but the call succeeded", $expected),
        }
    }};
}
