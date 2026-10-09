//! Build-time provisioning of the FFmpeg sidecar.
//!
//! Tauri bundles `binaries/ffmpeg-<target-triple>[.exe]` as an external binary
//! (`bundle.externalBin` in `tauri.conf.json`). This module makes sure that file
//! exists before `tauri_build::build()` runs.
//!
//! Every download is pinned twice (PARITY_PLAN §3 rule 10):
//! - the **archive** by URL, byte size and SHA-256, and
//! - the **extracted executable** by byte size and SHA-256.
//!
//! A cached executable is reused only when it still matches its pin, so bumping
//! a pin forces a fresh download. The build never executes the downloaded
//! binary, which also keeps cross-compilation working.
//!
//! Sources (all FFmpeg 8.0.1):
//! - macOS and Linux: Martin Riedl's FFmpeg build server, whose release builds
//!   are kept under permanent, versioned URLs and published with `.sha256` files.
//! - Windows: Gyan Doshi's "essentials" build from the `GyanD/codexffmpeg`
//!   GitHub release, whose digest GitHub also reports for the asset.

use sha2::{Digest, Sha256};
use std::{
    env,
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    process,
    time::Duration,
};

/// The FFmpeg release every pinned build below was cut from.
const FFMPEG_VERSION: &str = "8.0.1";

/// Upper bound for a single archive download; the largest pinned archive is ~106 MB.
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(600);

/// Read buffer used while hashing files.
const HASH_BUFFER_BYTES: usize = 64 * 1024;

/// Expected byte size and lowercase hex SHA-256 of a file.
struct FilePin {
    size: u64,
    sha256: &'static str,
}

/// One pinned FFmpeg distribution for a single OS and CPU architecture.
struct FfmpegBuild {
    /// Immutable download URL of the archive.
    archive_url: &'static str,
    /// Pin for the archive as served.
    archive: FilePin,
    /// Path of the `ffmpeg` executable inside the zip archive.
    member: &'static str,
    /// Pin for the extracted executable.
    binary: FilePin,
}

const MACOS_ARM64: FfmpegBuild = FfmpegBuild {
    archive_url: "https://ffmpeg.martin-riedl.de/download/macos/arm64/1766430132_8.0.1/ffmpeg.zip",
    archive: FilePin {
        size: 26_953_718,
        sha256: "c56f4e2b2ce26a61becf890d8da3415347a1d7d4418cb514915f21612358b790",
    },
    member: "ffmpeg",
    binary: FilePin {
        size: 62_024_672,
        sha256: "3b586ff896c0339e8fd574c143aaccac23c80789341e22d4202f8013a133d3a4",
    },
};

const MACOS_X86_64: FfmpegBuild = FfmpegBuild {
    archive_url: "https://ffmpeg.martin-riedl.de/download/macos/amd64/1766437297_8.0.1/ffmpeg.zip",
    archive: FilePin {
        size: 32_877_318,
        sha256: "a6c41c69e829697e408308f1ecd6acdfd0d0a84973ff3a6bf782beba83885ed6",
    },
    member: "ffmpeg",
    binary: FilePin {
        size: 92_506_128,
        sha256: "0d9a45766a11804a598731e95b9bf1bc4306850150073e394bb63be8bed50add",
    },
};

const LINUX_X86_64: FfmpegBuild = FfmpegBuild {
    archive_url: "https://ffmpeg.martin-riedl.de/download/linux/amd64/1766430728_8.0.1/ffmpeg.zip",
    archive: FilePin {
        size: 32_323_774,
        sha256: "90cdb6066c3b11489df4b8bae1999d2e056e66e728fc482834db92769bf5d202",
    },
    member: "ffmpeg",
    binary: FilePin {
        size: 89_374_336,
        sha256: "aa4a1c02351ff62d3cb80173f7796b93cd540922af1577e075674fc222c4fc1f",
    },
};

const LINUX_ARM64: FfmpegBuild = FfmpegBuild {
    archive_url: "https://ffmpeg.martin-riedl.de/download/linux/arm64/1766429332_8.0.1/ffmpeg.zip",
    archive: FilePin {
        size: 27_730_206,
        sha256: "a032a323119086c2dd71f5e9e27f0cb55c010a3dcc0dd7d5591d06b0a2d87702",
    },
    member: "ffmpeg",
    binary: FilePin {
        size: 58_113_240,
        sha256: "8342efa7d256df6d46e10b4994e234d78498c9254a785b2c6d804b4b609b4e0b",
    },
};

const WINDOWS_X86_64: FfmpegBuild = FfmpegBuild {
    archive_url: "https://github.com/GyanD/codexffmpeg/releases/download/8.0.1/ffmpeg-8.0.1-essentials_build.zip",
    archive: FilePin {
        size: 106_259_850,
        sha256: "e2aaeaa0fdbc397d4794828086424d4aaa2102cef1fb6874f6ffd29c0b88b673",
    },
    member: "ffmpeg-8.0.1-essentials_build/bin/ffmpeg.exe",
    binary: FilePin {
        size: 99_264_000,
        sha256: "5af82a0d4fe2b9eae211b967332ea97edfc51c6b328ca35b827e73eac560dc0d",
    },
};

/// Ensures `binaries/ffmpeg-<target>[.exe]` exists and matches its pin.
///
/// Panics (failing the build) when the target is unsupported or a download
/// does not match its pin.
pub fn ensure_ffmpeg_binary() {
    let target = env_var("TARGET");
    let build = select_build().unwrap_or_else(|error| panic!("FFmpeg: {error}"));
    let output = sidecar_path(&target);

    if verify_file(&output, &build.binary).is_ok() {
        println!("cargo:warning=FFmpeg {FFMPEG_VERSION}: using verified cached sidecar");
        return;
    }

    println!(
        "cargo:warning=FFmpeg {FFMPEG_VERSION}: downloading {}",
        build.archive_url
    );
    install(build, &target, &output).unwrap_or_else(|error| panic!("FFmpeg: {error}"));
    println!(
        "cargo:warning=FFmpeg {FFMPEG_VERSION}: installed {}",
        output.display()
    );
}

/// Picks the pinned build for the target OS and CPU architecture.
fn select_build() -> Result<&'static FfmpegBuild, String> {
    let os = env_var("CARGO_CFG_TARGET_OS");
    let arch = env_var("CARGO_CFG_TARGET_ARCH");
    match (os.as_str(), arch.as_str()) {
        ("macos", "aarch64") => Ok(&MACOS_ARM64),
        ("macos", "x86_64") => Ok(&MACOS_X86_64),
        ("linux", "x86_64") => Ok(&LINUX_X86_64),
        ("linux", "aarch64") => Ok(&LINUX_ARM64),
        ("windows", "x86_64") => Ok(&WINDOWS_X86_64),
        _ => Err(format!("no pinned build for {os}/{arch}")),
    }
}

/// Path Tauri expects for the sidecar of `target`.
fn sidecar_path(target: &str) -> PathBuf {
    let extension = if target.contains("windows") {
        ".exe"
    } else {
        ""
    };
    PathBuf::from(env_var("CARGO_MANIFEST_DIR"))
        .join("binaries")
        .join(format!("ffmpeg-{target}{extension}"))
}

/// Downloads, verifies and atomically installs the executable at `output`.
fn install(build: &FfmpegBuild, target: &str, output: &Path) -> Result<(), String> {
    let parent = output
        .parent()
        .ok_or_else(|| format!("{} has no parent directory", output.display()))?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;

    // Process-unique scratch names, so parallel builds never share a file.
    let archive = env::temp_dir().join(format!("deino-ffmpeg-{}-{target}.zip", process::id()));
    let staged = parent.join(format!(".ffmpeg-{}-{target}.partial", process::id()));

    let result = download_and_stage(build, &archive, &staged).and_then(|()| {
        fs::rename(&staged, output)
            .map_err(|error| format!("failed to install {}: {error}", output.display()))
    });
    let _ = fs::remove_file(&archive);
    let _ = fs::remove_file(&staged);
    result
}

/// Fetches the archive, checks both pins and writes the executable to `staged`.
fn download_and_stage(build: &FfmpegBuild, archive: &Path, staged: &Path) -> Result<(), String> {
    download(build.archive_url, archive, &build.archive)?;
    verify_file(archive, &build.archive)?;
    extract_member(archive, build.member, staged)?;
    verify_file(staged, &build.binary)?;
    make_executable(staged)
}

/// Streams `url` into `destination`, refusing anything larger than the pin.
fn download(url: &str, destination: &Path, pin: &FilePin) -> Result<(), String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(DOWNLOAD_TIMEOUT)
        .build()
        .map_err(|error| format!("failed to create HTTP client: {error}"))?;
    let response = client
        .get(url)
        .send()
        .and_then(reqwest::blocking::Response::error_for_status)
        .map_err(|error| format!("download of {url} failed: {error}"))?;
    let mut file = File::create(destination)
        .map_err(|error| format!("failed to create {}: {error}", destination.display()))?;

    // Read one byte past the pin so an oversized body is detected by `verify_file`.
    std::io::copy(&mut response.take(pin.size + 1), &mut file)
        .map_err(|error| format!("download of {url} failed: {error}"))?;
    file.flush()
        .map_err(|error| format!("failed to write {}: {error}", destination.display()))
}

/// Copies the zip entry `member` out of `archive` into `destination`.
fn extract_member(archive: &Path, member: &str, destination: &Path) -> Result<(), String> {
    let file = File::open(archive)
        .map_err(|error| format!("failed to open {}: {error}", archive.display()))?;
    let mut zip =
        zip::ZipArchive::new(file).map_err(|error| format!("invalid FFmpeg archive: {error}"))?;
    let mut entry = zip
        .by_name(member)
        .map_err(|error| format!("{member} missing from FFmpeg archive: {error}"))?;
    let mut output = File::create(destination)
        .map_err(|error| format!("failed to create {}: {error}", destination.display()))?;
    std::io::copy(&mut entry, &mut output)
        .map_err(|error| format!("failed to extract {member}: {error}"))?;
    Ok(())
}

/// Marks the staged executable `rwxr-xr-x` (no-op on Windows).
fn make_executable(path: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o755))
            .map_err(|error| format!("failed to chmod {}: {error}", path.display()))?;
    }
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

/// Checks that `path` has exactly the pinned size and SHA-256.
fn verify_file(path: &Path, pin: &FilePin) -> Result<(), String> {
    let name = path.display();
    let size = fs::metadata(path)
        .map_err(|error| format!("failed to inspect {name}: {error}"))?
        .len();
    if size != pin.size {
        return Err(format!("{name} has size {size}, expected {}", pin.size));
    }

    let actual = sha256_hex(path)?;
    if actual != pin.sha256 {
        return Err(format!(
            "{name} has SHA-256 {actual}, expected {}",
            pin.sha256
        ));
    }
    Ok(())
}

/// Lowercase hex SHA-256 of the file at `path`.
fn sha256_hex(path: &Path) -> Result<String, String> {
    let mut file =
        File::open(path).map_err(|error| format!("failed to open {}: {error}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0_u8; HASH_BUFFER_BYTES];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| format!("failed to hash {}: {error}", path.display()))?;
        if read == 0 {
            return Ok(format!("{:x}", hasher.finalize()));
        }
        hasher.update(&buffer[..read]);
    }
}

/// Reads an environment variable Cargo always sets for build scripts.
fn env_var(name: &str) -> String {
    // Cargo sets TARGET, CARGO_MANIFEST_DIR and CARGO_CFG_* for every build script.
    env::var(name).unwrap_or_else(|_| panic!("Cargo did not set {name}"))
}
