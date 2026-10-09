//! Injected worker faults: crash or hang at a chosen stage, the way a
//! `GGML_ASSERT` abort or a wedged GPU driver would. Ported from Handy's
//! `HANDY_WORKER_FAULT` hooks (MIT).

use std::path::{Path, PathBuf};

/// Where a fault can be injected.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FaultStage {
    /// Backend registration, before the hello is answered.
    Init,
    /// Model load.
    Load,
    /// A one-shot run.
    Run,
    /// A stream feed.
    Feed,
    /// A stream finalize.
    Finalize,
}

/// What the fault does to the worker.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FaultKind {
    /// Abort the process, as `GGML_ASSERT` does.
    Crash,
    /// Stop answering forever, as a wedged GPU driver does.
    Hang,
}

/// One injected fault.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FaultPlan {
    /// What happens.
    pub kind: FaultKind,
    /// Where.
    pub stage: FaultStage,
    /// Fire on the n-th time the stage is reached in this worker (1-based).
    pub nth: u32,
    /// Fire only while running on the (pretend) GPU, so a CPU retry succeeds.
    pub gpu_only: bool,
    /// Fire only in the first worker to get here: the marker file records it.
    pub once_marker: Option<PathBuf>,
}

impl FaultPlan {
    /// Fires the fault when `stage` is reached for the `reached`-th time and
    /// the plan applies to this worker (GPU-only plans skip CPU workers).
    pub fn fire_if_due(&self, stage: FaultStage, reached: u32, cpu_only: bool) {
        let due = self.stage == stage && self.nth == reached && !(self.gpu_only && cpu_only);
        if due && claim_marker(self.once_marker.as_deref()) {
            fire(self.kind);
        }
    }

    /// Environment variable: `<crash|hang>@<init|load|run|feed|finalize>[#n]`.
    pub const ENV: &'static str = "DEINO_STT_FAULT";
    /// Environment variable: set to fire only on the GPU.
    pub const GPU_ONLY_ENV: &'static str = "DEINO_STT_FAULT_GPU_ONLY";
    /// Environment variable: a marker path, to fire in one worker only.
    pub const ONCE_ENV: &'static str = "DEINO_STT_FAULT_ONCE";

    /// Parses a plan from `DEINO_STT_FAULT=<kind>@<stage>[#n]`.
    pub fn parse(spec: &str) -> Option<Self> {
        let (kind, at) = spec.split_once('@')?;
        let (stage, nth) = at.split_once('#').unwrap_or((at, "1"));
        let kind = match kind {
            "crash" => FaultKind::Crash,
            "hang" => FaultKind::Hang,
            _ => return None,
        };
        Some(Self {
            kind,
            stage: parse_stage(stage)?,
            nth: nth.parse().ok()?,
            gpu_only: false,
            once_marker: None,
        })
    }

    /// The plan in this process's environment, if any.
    pub fn from_env(env: impl Fn(&str) -> Option<String>) -> Option<Self> {
        let mut plan = Self::parse(&env(Self::ENV)?)?;
        plan.gpu_only = env(Self::GPU_ONLY_ENV).is_some();
        plan.once_marker = env(Self::ONCE_ENV).map(PathBuf::from);
        Some(plan)
    }
}

fn parse_stage(stage: &str) -> Option<FaultStage> {
    Some(match stage {
        "init" => FaultStage::Init,
        "load" => FaultStage::Load,
        "run" => FaultStage::Run,
        "feed" => FaultStage::Feed,
        "finalize" => FaultStage::Finalize,
        _ => return None,
    })
}

/// True when this worker may fire: no marker, or it created the marker first.
fn claim_marker(marker: Option<&Path>) -> bool {
    marker.is_none_or(|path| {
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .is_ok()
    })
}

fn fire(kind: FaultKind) {
    log::warn!("injected fault: {kind:?}");
    match kind {
        FaultKind::Crash => std::process::abort(),
        FaultKind::Hang => loop {
            std::thread::park();
        },
    }
}
