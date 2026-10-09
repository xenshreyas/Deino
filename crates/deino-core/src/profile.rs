//! The user's profile and global AI context (NOT-05).
//!
//! - [`ProfileService`] saves Settings → Profile (normalising a
//!   [`ProfileInput`] into `settings::ProfileSettings`) and keeps the "Me"
//!   person (`people.is_me`) named and addressed as the profile says.
//! - [`render_profile_context`] turns the profile into the snippet every LLM
//!   prompt starts with; [`ProfileContext`] gives it to the gateway as its
//!   `deino_llm::SystemContext`.
//! - [`InternalDomains`] tells internal meetings from external ones by email
//!   domain, for template auto rules and later briefs, follow-ups and
//!   calendar sync.

mod audience;
mod context;
mod error;
mod input;
mod me;
mod service;
#[cfg(test)]
mod service_tests;

pub use audience::InternalDomains;
pub use context::{ProfileContext, render_profile_context};
pub use error::ProfileError;
pub use input::ProfileInput;
pub use me::DEFAULT_ME_NAME;
pub use service::ProfileService;
