//! Build script: stages the pinned FFmpeg and ONNX Runtime binaries, reports
//! the GPU backend, and generates the ACL of every inline feature plugin.

// Inherited downloaders that predate PARITY_PLAN §3A. FND-05 removes FFmpeg (D11).
#[allow(
    clippy::disallowed_types,
    clippy::doc_markdown,
    clippy::duration_suboptimal_units,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::large_stack_arrays,
    clippy::let_underscore_must_use,
    clippy::manual_assert,
    clippy::panic,
    clippy::too_many_lines,
    reason = "inherited build-time downloaders; see legacy_lints.rs for the policy"
)]
#[path = "build"]
mod downloads {
    pub mod ffmpeg;
    pub mod onnxruntime;
}
use downloads::{ffmpeg, onnxruntime};

// `FEATURES`: the inline feature plugins and their commands (ADR 0002).
include!("src/features/manifest.rs");
// `ORGANIZE_FEATURES`: the organization feature plugins (the ORG cards).
include!("src/features/organize_manifest.rs");
// `DEV_FEATURES`: the feature plugins of debug builds.
include!("src/features/dev_manifest.rs");

#[allow(
    clippy::expect_used,
    reason = "a build script reports failure by panicking"
)]
fn main() {
    // GPU Acceleration Detection and Build Guidance
    detect_and_report_gpu_capabilities();

    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-link-lib=framework=AVFoundation");
        println!("cargo:rustc-link-lib=framework=Cocoa");
        println!("cargo:rustc-link-lib=framework=Foundation");

        // Let the enhanced_macos crate handle its own Swift compilation
        // The swift-rs crate build will be handled in the enhanced_macos crate's build.rs
    }

    // Download and bundle FFmpeg binary at build-time to eliminate runtime download delays
    ffmpeg::ensure_ffmpeg_binary();
    onnxruntime::ensure_onnxruntime_runtime();

    tauri_build::try_build(tauri_attributes()).expect("tauri build failed");
}

/// Capability files for release builds: `capabilities/dev/` (debug-only
/// feature plugins) is left out, because its plugins do not exist there.
const RELEASE_CAPABILITIES: &str = "./capabilities/*.json";

/// Registers every feature in `src/features/manifest.rs` and
/// `organize_manifest.rs` (and, in debug builds, `dev_manifest.rs`) as an
/// inline plugin,
/// so tauri-build generates its `allow-*`/`deny-*` permissions and a
/// `<name>:default` set that allows all of its commands. `DEV_FEATURES` and
/// their capabilities are only registered when debug assertions are on.
fn tauri_attributes() -> tauri_build::Attributes {
    println!("cargo:rerun-if-changed=src/features/manifest.rs");
    println!("cargo:rerun-if-changed=src/features/manifest_commands.rs");
    println!("cargo:rerun-if-changed=src/features/organize_manifest.rs");
    println!("cargo:rerun-if-changed=src/features/dev_manifest.rs");
    println!("cargo:rerun-if-changed=src/features/recording_manifest.rs");
    let debug = std::env::var_os("CARGO_CFG_DEBUG_ASSERTIONS").is_some();
    let dev_features: &[(&str, &[&str])] = if debug { DEV_FEATURES } else { &[] };
    let attributes = FEATURES
        .iter()
        .chain(ORGANIZE_FEATURES)
        .chain(dev_features)
        .fold(
            tauri_build::Attributes::new(),
            |attributes, &(name, commands)| {
                attributes.plugin(
                    name,
                    tauri_build::InlinedPlugin::new()
                        .commands(commands)
                        .default_permission(tauri_build::DefaultPermissionRule::AllowAllCommands),
                )
            },
        );
    if debug {
        attributes
    } else {
        attributes.capabilities_path_pattern(RELEASE_CAPABILITIES)
    }
}

/// Detects GPU acceleration capabilities and provides build guidance
#[allow(
    clippy::too_many_lines,
    clippy::uninlined_format_args,
    reason = "inherited build guidance that predates PARITY_PLAN §3A"
)]
fn detect_and_report_gpu_capabilities() {
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    println!("cargo:warning=🚀 Building Deino for: {}", target_os);

    match target_os.as_str() {
        "macos" => {
            println!("cargo:warning=✅ macOS: Metal GPU acceleration ENABLED by default");
            #[cfg(feature = "coreml")]
            println!("cargo:warning=✅ CoreML acceleration ENABLED");
        }
        "windows" => {
            if cfg!(feature = "cuda") {
                println!("cargo:warning=✅ Windows: CUDA GPU acceleration ENABLED");
            } else if cfg!(feature = "vulkan") {
                println!("cargo:warning=✅ Windows: Vulkan GPU acceleration ENABLED");
            } else if cfg!(feature = "openblas") {
                println!("cargo:warning=✅ Windows: OpenBLAS CPU optimization ENABLED");
            } else {
                println!(
                    "cargo:warning=⚠️  Windows: Using CPU-only mode (no GPU or BLAS acceleration)"
                );
                println!("cargo:warning=💡 For NVIDIA GPU: cargo build --release --features cuda");
                println!(
                    "cargo:warning=💡 For AMD/Intel GPU: cargo build --release --features vulkan"
                );
                println!(
                    "cargo:warning=💡 For CPU optimization: cargo build --release --features openblas"
                );

                // Try to detect NVIDIA GPU
                if which::which("nvidia-smi").is_ok() {
                    println!(
                        "cargo:warning=🎯 NVIDIA GPU detected! Consider rebuilding with --features cuda"
                    );
                }
            }
        }
        "linux" => {
            if cfg!(feature = "cuda") {
                println!("cargo:warning=✅ Linux: CUDA GPU acceleration ENABLED");
            } else if cfg!(feature = "vulkan") {
                println!("cargo:warning=✅ Linux: Vulkan GPU acceleration ENABLED");
            } else if cfg!(feature = "hipblas") {
                println!("cargo:warning=✅ Linux: AMD ROCm (HIP) acceleration ENABLED");
            } else if cfg!(feature = "openblas") {
                println!("cargo:warning=✅ Linux: OpenBLAS CPU optimization ENABLED");
            } else {
                println!(
                    "cargo:warning=⚠️  Linux: Using CPU-only mode (no GPU or BLAS acceleration)"
                );
                println!("cargo:warning=💡 For NVIDIA GPU: cargo build --release --features cuda");
                println!("cargo:warning=💡 For AMD GPU: cargo build --release --features hipblas");
                println!(
                    "cargo:warning=💡 For other GPUs: cargo build --release --features vulkan"
                );
                println!(
                    "cargo:warning=💡 For CPU optimization: cargo build --release --features openblas"
                );

                // Try to detect NVIDIA GPU
                if which::which("nvidia-smi").is_ok() {
                    println!(
                        "cargo:warning=🎯 NVIDIA GPU detected! Consider rebuilding with --features cuda"
                    );
                }

                // Try to detect AMD GPU
                if which::which("rocm-smi").is_ok() {
                    println!(
                        "cargo:warning=🎯 AMD GPU detected! Consider rebuilding with --features hipblas"
                    );
                }
            }
        }
        _ => {
            println!("cargo:warning=ℹ️  Unknown platform: {}", target_os);
        }
    }

    // Performance guidance
    if !cfg!(feature = "cuda")
        && !cfg!(feature = "vulkan")
        && !cfg!(feature = "hipblas")
        && !cfg!(feature = "openblas")
        && target_os != "macos"
    {
        println!(
            "cargo:warning=📊 Performance: CPU-only builds are significantly slower than GPU/BLAS builds"
        );
        println!("cargo:warning=📚 See README.md for GPU/BLAS setup instructions");
    }
}
