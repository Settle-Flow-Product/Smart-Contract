//! The registry test suite.
//!
//! **Assert the exact error code.** `assert_eq!(err, Error::AlreadyPaid)`, never
//! `assert!(result.is_err())`. A test that only checks *that* a call failed passes when the
//! contract fails for the wrong reason, which is precisely the bug worth catching.
//!
//! `require_auth` failures panic in the host rather than returning a code. Those cases assert a
//! panic, and their names say so, so nobody later "fixes" them into code assertions.
//!
//! Test names are full sentences. The name is what a reviewer reads in the `cargo test` output.
//!
//! The mandatory matrix is in `../../../docs/11-testing.md`.

#[macro_use]
mod harness;

mod abuse_tests;
mod boundary_tests;
mod lifecycle_tests;
mod system_tests;
