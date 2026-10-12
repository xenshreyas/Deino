//! One [`Detector`](super::detector::Detector) per [`SecretKind`](super::SecretKind).
//!
//! | File | Finds | Confidence |
//! |---|---|---|
//! | `password.rs` | The words after "password is", "passphrase:" … | likely |
//! | `api_key.rs` | Provider prefixes (`sk-ant-`, `ghp_`, `AKIA` …); long mixed-case tokens | certain; likely |
//! | `card.rs` | 13–19 digits on a card network's prefix that pass Luhn | certain |
//! | `ssn.rs` | `123-45-6789` with valid parts; nine digits after "social security" | certain; likely |
//! | `one_time_code.rs` | 4–8 digits after "code", "PIN", "OTP" … | likely |

mod api_key;
mod card;
mod one_time_code;
mod password;
mod ssn;

pub(super) use api_key::ApiKeyDetector;
pub(super) use card::CardDetector;
#[cfg(test)]
pub(super) use card::luhn;
pub(super) use one_time_code::OneTimeCodeDetector;
pub(super) use password::PasswordDetector;
pub(super) use ssn::SsnDetector;
