//! The one attribute cidre has no safe accessor for: a list of strings
//! (Chromium's `AXDOMClassList`, a `CFArray` of `CFString`s). The crate's
//! only `unsafe` is here, behind a type check.

#![allow(
    unsafe_code,
    reason = "reinterpreting a CFArray (checked by type id) as cidre's typed array view; cidre's own casts do the same"
)]

use cidre::cf;

/// The strings in `value` when it is an array; non-string items are skipped.
pub(super) fn strings(value: &cf::Type) -> Vec<String> {
    if value.get_type_id() != cf::Array::type_id() {
        return Vec::new();
    }
    // SAFETY: the type id says `value` is a CFArray, and `ArrayOf<cf::Type>`
    // is cidre's view of a CFArray whose items are any CF objects. Each item
    // is checked to be a string before it is read as one.
    let array = unsafe { &*std::ptr::from_ref(value).cast::<cf::ArrayOf<cf::Type>>() };
    array
        .iter()
        .filter_map(|item| item.try_as_string().map(ToString::to_string))
        .collect()
}
