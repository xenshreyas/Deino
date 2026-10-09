//! Companies: who the people the user meets work for.
//!
//! [`CompanyRepo`] stores them, keyed for upserts by [`CompanyDomain`];
//! [`SqliteCompanyRepo`] implements it and `FakeCompanyRepo` stands in for
//! tests. [`is_public_email_domain`] names the personal mail providers whose
//! domain is never a company. ORG-01 upserts companies from calendar
//! attendees; ORG-04 adds the rest of the rules (merging) and the pages.

#[cfg(any(test, feature = "test-support"))]
mod fake;
mod model;
mod public_domains;
mod repo;
#[cfg(test)]
mod repo_tests;

#[cfg(any(test, feature = "test-support"))]
pub use fake::FakeCompanyRepo;
pub use model::{Company, CompanyDomain};
pub use public_domains::{PUBLIC_EMAIL_DOMAINS, is_public_email_domain};
pub use repo::{CompanyRepo, SqliteCompanyRepo};
