//! The strict-lint exemption for inherited modules, in one place.
//!
//! The workspace lints (`[workspace.lints]` in the root `Cargo.toml` and
//! `clippy.toml`) enforce PARITY_PLAN §3A everywhere. Code inherited from the
//! upstream app predates those rules, so `lib.rs` declares those modules
//! through [`legacy_modules!`], which allows exactly the lints they still
//! break, and nothing else: correctness lints and `unused_must_use` stay on.
//! New modules are declared normally and get no exemption.
//!
//! The list only shrinks. When a card ports a module, it moves the module's
//! declaration out of `legacy_modules!` in `lib.rs`; when no legacy module
//! breaks a lint any more, the lint leaves the list. Files over 300 lines are
//! tracked separately in `scripts/check-rust-file-size.sh`.
//!
//! One trap: a `#[macro_export]` macro inside a module declared here cannot be
//! imported by a `crate::` path. Declare it with `pub(crate) use name;` instead.

/// Declares inherited modules with the legacy lint exemption.
macro_rules! legacy_modules {
    ($($vis:vis mod $name:ident;)*) => {
        $(
            #[allow(
                // rustc
                dead_code,
                missing_docs,
                unsafe_code,
                unused_imports,
                unused_parens,
                unused_variables,
                // PARITY_PLAN §3A limits and safety rules
                clippy::allow_attributes_without_reason,
                clippy::cognitive_complexity,
                clippy::disallowed_methods,
                clippy::disallowed_types,
                clippy::expect_used,
                clippy::indexing_slicing,
                clippy::let_underscore_must_use,
                clippy::too_many_arguments,
                clippy::unwrap_used,
                // The pedantic group, including `too_many_lines`
                clippy::pedantic,
                // Clippy's default style, complexity and perf groups
                clippy::arc_with_non_send_sync,
                clippy::cmp_null,
                clippy::collapsible_if,
                clippy::derivable_impls,
                clippy::doc_lazy_continuation,
                clippy::empty_line_after_doc_comments,
                clippy::explicit_counter_loop,
                clippy::field_reassign_with_default,
                clippy::get_first,
                clippy::implicit_saturating_sub,
                clippy::int_plus_one,
                clippy::items_after_test_module,
                clippy::len_zero,
                clippy::manual_clamp,
                clippy::manual_div_ceil,
                clippy::manual_is_multiple_of,
                clippy::manual_range_contains,
                clippy::map_clone,
                clippy::module_inception,
                clippy::needless_borrow,
                clippy::needless_borrows_for_generic_args,
                clippy::needless_range_loop,
                clippy::needless_return,
                clippy::new_without_default,
                clippy::ptr_arg,
                clippy::redundant_closure,
                clippy::redundant_pattern_matching,
                clippy::should_implement_trait,
                clippy::to_string_in_format_args,
                clippy::type_complexity,
                clippy::unnecessary_cast,
                clippy::unnecessary_map_or,
                clippy::unwrap_or_default,
                clippy::useless_conversion,
                clippy::wildcard_in_or_patterns,
                reason = "inherited code that predates PARITY_PLAN §3A; see legacy_lints.rs"
            )]
            $vis mod $name;
        )*
    };
}
