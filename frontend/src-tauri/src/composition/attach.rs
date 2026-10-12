//! Attach hooks: the wiring of leaf features, registered from each feature's
//! own `composition/<feature>.rs` instead of a line in `composition.rs`.
//!
//! A feature whose services nothing else is built from (stats, briefs, the
//! player and clips, imports, speaker tags, ...) registers its wiring with
//! [`attach!`]. The composition root runs every hook of a [`Phase`] at that
//! phase's fixed point in startup, sorted by `order` and then by name:
//!
//! ```ignore
//! // composition/stats.rs
//! super::attach::attach!("stats", 10, Phase::ReadModel(|cx, ai| attach_stats(cx, ai)));
//! ```
//!
//! The shared services (settings, the network client, the database, the LLM
//! gateway and search, the pipeline and its stage services) stay explicit in
//! `composition.rs`, because later steps are built from them.

use std::error::Error;
use std::sync::Arc;

use deino_db::Database;
use deino_domain::Clock;
use linkme::distributed_slice;
use tauri::AppHandle;

use super::action_items::ActionItemWiring;

/// What every database-backed hook is wired over.
pub(crate) struct Context<'a> {
    /// The app, where services are managed as state.
    pub(crate) app: &'a AppHandle,
    /// The app's database.
    pub(crate) database: &'a Database,
    /// The time.
    pub(crate) clock: &'a Arc<dyn Clock>,
}

/// When a hook runs, with what it needs at that point. Phases run in this
/// order; [`Phase::Startup`] runs from `setup.rs`, the others from
/// `composition::attach_database`.
pub(crate) enum Phase {
    /// Right after the database opens and the core services are managed:
    /// needs only the database and the clock.
    Local(fn(&Context<'_>)),
    /// After the LLM gateway and the pipeline's stage services: read models
    /// over the AI wiring.
    ReadModel(fn(&Context<'_>, &ActionItemWiring)),
    /// Once the pipeline runs: services that work on their own.
    Background(fn(&Context<'_>)),
    /// At the end of startup, after the database: a feature with its own store.
    Startup(fn(&AppHandle) -> Result<(), Box<dyn Error>>),
}

/// One registered hook.
pub(crate) struct Attachment {
    /// The feature it wires, for ordering ties and logs.
    pub(crate) name: &'static str,
    /// Its place within its phase, lowest first (like pipeline stage orders).
    pub(crate) order: u16,
    /// When it runs, and the wiring itself.
    pub(crate) phase: Phase,
}

/// Every attach hook in this build, in no particular order.
#[allow(
    unsafe_code,
    reason = "linkme declares the slice over a linker section; no unsafe code of ours"
)]
#[distributed_slice]
pub(crate) static ATTACHMENTS: [Attachment];

/// Registers an [`Attachment`]: `attach!(name, order, phase)`.
macro_rules! attach {
    ($name:literal, $order:literal, $phase:expr) => {
        const _: () = {
            #[allow(
                unsafe_code,
                reason = "linkme places the entry in a linker section; no unsafe code of ours"
            )]
            #[linkme::distributed_slice($crate::composition::attach::ATTACHMENTS)]
            static ATTACH: $crate::composition::attach::Attachment =
                $crate::composition::attach::Attachment {
                    name: $name,
                    order: $order,
                    phase: $phase,
                };
        };
    };
}
pub(crate) use attach;

/// Every hook, by order and then by name.
fn sorted() -> Vec<&'static Attachment> {
    let mut hooks: Vec<&'static Attachment> = ATTACHMENTS.iter().collect();
    hooks.sort_by_key(|hook| (hook.order, hook.name));
    hooks
}

/// Runs every [`Phase::Local`] hook.
pub(crate) fn run_local(cx: &Context<'_>) {
    for hook in sorted() {
        if let Phase::Local(attach) = hook.phase {
            attach(cx);
        }
    }
}

/// Runs every [`Phase::ReadModel`] hook.
pub(crate) fn run_read_models(cx: &Context<'_>, ai: &ActionItemWiring) {
    for hook in sorted() {
        if let Phase::ReadModel(attach) = hook.phase {
            attach(cx, ai);
        }
    }
}

/// Runs every [`Phase::Background`] hook.
pub(crate) fn run_background(cx: &Context<'_>) {
    for hook in sorted() {
        if let Phase::Background(attach) = hook.phase {
            attach(cx);
        }
    }
}

/// Runs every [`Phase::Startup`] hook, stopping at the first error.
pub(crate) fn run_startup(app: &AppHandle) -> Result<(), Box<dyn Error>> {
    for hook in sorted() {
        if let Phase::Startup(attach) = hook.phase {
            attach(app)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The order every hook runs in, as `phase/order name`.
    fn schedule() -> Vec<String> {
        sorted()
            .iter()
            .map(|hook| {
                let phase = match hook.phase {
                    Phase::Local(_) => "local",
                    Phase::ReadModel(_) => "read-model",
                    Phase::Background(_) => "background",
                    Phase::Startup(_) => "startup",
                };
                format!("{phase}/{} {}", hook.order, hook.name)
            })
            .collect()
    }

    /// The hooks reach the slice from their own files (linkme), sorted by order.
    #[test]
    fn hooks_registered_in_their_own_files_are_scheduled_by_phase_and_order() {
        let schedule = schedule();
        let position = |hook: &str| schedule.iter().position(|entry| entry == hook);
        let speakers = position("local/10 speakers").expect("speakers registers a hook");
        let player = position("local/50 player").expect("player registers a hook");
        assert!(speakers < player, "{schedule:?}");
        assert!(position("read-model/20 briefs").is_some(), "{schedule:?}");
        assert!(
            position("background/60 integrations").is_some(),
            "{schedule:?}"
        );
    }

    #[test]
    fn hook_names_are_unique() {
        let mut names: Vec<&str> = sorted().iter().map(|hook| hook.name).collect();
        names.sort_unstable();
        let count = names.len();
        names.dedup();
        assert_eq!(
            names.len(),
            count,
            "a hook is registered twice: {:?}",
            schedule()
        );
    }
}
