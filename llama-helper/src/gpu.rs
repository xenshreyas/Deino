//! How many model layers to offload to the GPU.
//!
//! The sidecar estimates the GPU memory it may use ([`detect_vram_gb`]) and
//! how much each layer costs (weights plus KV cache, from the file size and
//! the context size), then offloads as many layers as fit
//! ([`plan_gpu_layers`]). Without a GPU feature it assumes a conservative
//! amount, which only matters to a GPU build.

use std::path::Path;

use crate::diag::diag;

/// Bytes in a GiB.
const BYTES_PER_GB: f64 = 1024.0 * 1024.0 * 1024.0;
/// VRAM assumed when it cannot be detected.
const FALLBACK_VRAM_GB: f64 = 4.0;
/// Memory kept free for the OS and the display.
const SAFETY_BUFFER_GB: f64 = 0.5;
/// Files above this size are treated as 7B-class models, below as 1B-class.
const LARGE_MODEL_GB: f64 = 2.5;
/// KV cache per 1000 context tokens for a 7B-class model (4096 hidden dim).
const LARGE_MODEL_KV_PER_1K_GB: f64 = 0.25;
/// KV cache per 1000 context tokens for a 1B-class model (2048 hidden dim).
const SMALL_MODEL_KV_PER_1K_GB: f64 = 0.12;
/// Estimated layers of a 7B-class model (Q4 files are about 4.1 GB).
const LARGE_MODEL_LAYERS: u32 = 33;
/// Estimated layers of a 1B-class model (Q4 files are about 1.1 GB).
const SMALL_MODEL_LAYERS: u32 = 28;

/// The inputs to the layer heuristic.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayerBudget {
    /// Size of the model file in GiB.
    pub file_size_gb: f64,
    /// Layers in the model.
    pub model_layers: u32,
    /// GPU memory available in GiB.
    pub vram_gb: f64,
    /// Context window in tokens.
    pub context_size: u32,
}

/// The GPU layer count for the model at `model_path` and `context_size`.
pub fn default_gpu_layers(model_path: &Path, context_size: u32) -> u32 {
    let vram_gb = detect_vram_gb();
    let file_size_gb = std::fs::metadata(model_path).map_or(0.0, |meta| bytes_to_gb(meta.len()));
    let model_layers = if file_size_gb > LARGE_MODEL_GB {
        LARGE_MODEL_LAYERS
    } else {
        SMALL_MODEL_LAYERS
    };
    plan_gpu_layers(LayerBudget {
        file_size_gb,
        model_layers,
        vram_gb,
        context_size,
    })
}

/// How many layers fit in the budget's VRAM, at most all of them.
pub fn plan_gpu_layers(budget: LayerBudget) -> u32 {
    if budget.file_size_gb <= 0.0 || budget.model_layers == 0 {
        diag!("Could not determine model file size, using conservative default");
        return 0;
    }
    let kv_per_1k_gb = if budget.file_size_gb > LARGE_MODEL_GB {
        LARGE_MODEL_KV_PER_1K_GB
    } else {
        SMALL_MODEL_KV_PER_1K_GB
    };
    let total_kv_gb = f64::from(budget.context_size) / 1000.0 * kv_per_1k_gb;
    let safe_vram = budget.vram_gb - SAFETY_BUFFER_GB;
    diag!(
        "VRAM: available {:.2} GB, safe {safe_vram:.2} GB, weights {:.2} GB, KV cache ({} ctx) {total_kv_gb:.2} GB",
        budget.vram_gb,
        budget.file_size_gb,
        budget.context_size
    );
    if safe_vram <= 0.0 {
        diag!("No safe VRAM available, using CPU only");
        return 0;
    }
    let layers = f64::from(budget.model_layers);
    let per_layer_gb = (budget.file_size_gb + total_kv_gb) / layers;
    let fitting = whole_layers((safe_vram / per_layer_gb).floor().min(layers));
    diag!(
        "Offloading {fitting}/{} layers ({:.2} MB per layer)",
        budget.model_layers,
        per_layer_gb * 1024.0
    );
    fitting
}

/// Converts a byte count to GiB.
#[allow(
    clippy::cast_precision_loss,
    reason = "model sizes are far below 2^53 bytes, and the heuristic needs only two decimals"
)]
fn bytes_to_gb(bytes: u64) -> f64 {
    bytes as f64 / BYTES_PER_GB
}

/// A whole, non-negative layer count from a value already floored and capped.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "the value is clamped to 0..=u32::MAX and already whole"
)]
fn whole_layers(value: f64) -> u32 {
    value.clamp(0.0, f64::from(u32::MAX)) as u32
}

/// GPU memory the sidecar may use, in GiB.
pub fn detect_vram_gb() -> f64 {
    #[cfg(feature = "metal")]
    {
        if let Some(vram) = detect_metal_vram() {
            diag!("Metal VRAM detected: {vram:.2} GB");
            return vram;
        }
    }
    #[cfg(feature = "cuda")]
    {
        if let Some(vram) = detect_cuda_vram() {
            diag!("CUDA VRAM detected: {vram:.2} GB");
            return vram;
        }
    }
    diag!("VRAM detection not available, using conservative estimate");
    FALLBACK_VRAM_GB
}

/// Share of system memory the GPU can use on Apple Silicon.
#[cfg(feature = "metal")]
const METAL_MEMORY_SHARE: f64 = 0.6;

/// Unified memory the Metal GPU may use: a share of `hw.memsize`.
#[cfg(feature = "metal")]
fn detect_metal_vram() -> Option<f64> {
    let output = std::process::Command::new("sysctl")
        .arg("hw.memsize")
        .output()
        .ok()?;
    let stdout = String::from_utf8(output.stdout).ok()?;
    let bytes = stdout.split(':').nth(1)?.trim().parse::<u64>().ok()?;
    Some(bytes_to_gb(bytes) * METAL_MEMORY_SHARE)
}

/// Free memory on the first CUDA device, from `nvidia-smi`.
#[cfg(feature = "cuda")]
fn detect_cuda_vram() -> Option<f64> {
    let output = std::process::Command::new("nvidia-smi")
        .args(["--query-gpu=memory.free", "--format=csv,noheader,nounits"])
        .output()
        .ok()?;
    let stdout = String::from_utf8(output.stdout).ok()?;
    let megabytes = stdout.trim().parse::<f64>().ok()?;
    Some(megabytes / 1024.0)
}

#[cfg(test)]
mod tests {
    use super::{LayerBudget, plan_gpu_layers};

    fn budget(file_size_gb: f64, vram_gb: f64) -> LayerBudget {
        LayerBudget {
            file_size_gb,
            model_layers: 33,
            vram_gb,
            context_size: 4096,
        }
    }

    #[test]
    fn offloads_every_layer_when_the_model_fits() {
        assert_eq!(plan_gpu_layers(budget(4.1, 16.0)), 33);
    }

    #[test]
    fn offloads_what_fits_when_memory_is_tight() {
        // 4.1 GB weights + 1.024 GB KV over 33 layers is 0.155 GB a layer;
        // 2.5 GB of safe VRAM holds 16 of them.
        assert_eq!(plan_gpu_layers(budget(4.1, 3.0)), 16);
    }

    #[test]
    fn uses_the_cpu_without_safe_vram_or_a_known_size() {
        assert_eq!(plan_gpu_layers(budget(4.1, 0.5)), 0);
        assert_eq!(plan_gpu_layers(budget(0.0, 16.0)), 0);
    }
}
