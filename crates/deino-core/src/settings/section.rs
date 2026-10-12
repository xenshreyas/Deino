//! [`SettingsSection`] and [`register_section!`]: how a section joins
//! [`Settings`](super::Settings) from its own file (REG-02).
//!
//! Every file in `settings/sections/` is re-exported by a glob, and every
//! `register_section!` in them becomes one field of `Settings`: `build.rs`
//! reads the invocations and generates the struct, its `Default`, its
//! `validate` and the patch's dialog-only list
//! (`docs/adr/2026-10-12-settings-self-registration.md`).

use super::SettingsError;

/// One section of [`Settings`](super::Settings): a `#[serde(default)]`
/// struct whose `Default` is the section's defaults.
pub(crate) trait SettingsSection: Default {
    /// Checks the rules this section enforces; a section without rules
    /// accepts every value its type can hold.
    fn check(&self) -> Result<(), SettingsError> {
        Ok(())
    }
}

/// Registers a section of `Settings` from its own file:
///
/// ```ignore
/// crate::settings::register_section! {
///     /// The field's doc, also on the generated TypeScript type.
///     ai: AiSettings,
///     order: 10,
///     validate: AiSettings::validate,
///     dialog_only: ["folder"],
/// }
/// ```
///
/// - the doc and `key: Type` become the field `pub key: Type` of
///   `Settings`, serialized as the camelCase `key`;
/// - `order` places the field (and the `settings.json` key): sections sort
///   by `order`, then key. Pick an unused number; existing sections use
///   multiples of 10;
/// - `validate` (optional) names the section's rule check;
/// - `dialog_only` (optional) lists the section's fields the UI may clear
///   but never set, by their JSON names: paths only a dialog Rust opens may
///   choose.
///
/// The macro implements [`SettingsSection`]; `build.rs` reads the rest from
/// the source text, so keep this exact layout (one entry per line).
macro_rules! register_section {
    (
        $(#[doc = $doc:literal])+
        $key:ident: $section:ty,
        order: $order:literal,
        $(validate: $validate:path,)?
        $(dialog_only: [$($field:literal),+ $(,)?],)?
    ) => {
        impl $crate::settings::SettingsSection for $section {
            $(
                fn check(&self) -> Result<(), $crate::settings::SettingsError> {
                    $validate(self)
                }
            )?
        }
    };
}
pub(crate) use register_section;
