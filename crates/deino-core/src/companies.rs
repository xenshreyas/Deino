//! Companies: who the people the user meets work for.
//!
//! [`CompanyRepo`] stores them, keyed for upserts by [`CompanyDomain`];
//! [`SqliteCompanyRepo`] implements it and `FakeCompanyRepo` stands in for
//! tests. ORG-04 adds the rules (upsert from email domains, skipping public
//! ones, merging) and the pages.

#[cfg(any(test, feature = "test-support"))]
mod fake;
mod model;
mod repo;
#[cfg(test)]
mod repo_tests;

#[cfg(any(test, feature = "test-support"))]
pub use fake::FakeCompanyRepo;
pub use model::{Company, CompanyDomain};
pub use repo::{CompanyRepo, SqliteCompanyRepo};
