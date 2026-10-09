//! ONNX Runtime bootstrap for Parakeet.
//!
//! On Windows the app bundles `onnxruntime.dll` as a resource and loads it at
//! startup ([`init_bundled`]). A failed load is remembered so that Parakeet
//! callers get a clear error from [`ensure_available`] instead of a crash.
//! On macOS and Linux ONNX Runtime is linked statically and both are no-ops.

#[cfg(target_os = "windows")]
static INIT_ERROR: std::sync::OnceLock<String> = std::sync::OnceLock::new();

/// Returns an error when the bundled ONNX Runtime failed to initialize.
#[cfg_attr(
    not(target_os = "windows"),
    allow(clippy::unnecessary_wraps, reason = "only the Windows build can fail")
)]
pub(crate) fn ensure_available() -> anyhow::Result<()> {
    #[cfg(target_os = "windows")]
    if let Some(error) = INIT_ERROR.get() {
        anyhow::bail!("{error}");
    }

    Ok(())
}

/// Loads the bundled ONNX Runtime (Windows only), recording any failure.
pub(crate) fn init_bundled<R: tauri::Runtime>(_app: &tauri::AppHandle<R>) {
    #[cfg(target_os = "windows")]
    windows::init(_app);
}

#[cfg(target_os = "windows")]
mod windows {
    use super::INIT_ERROR;
    use tauri::Manager;

    pub(super) fn init<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
        let runtime_path = match app
            .path()
            .resolve("onnxruntime.dll", tauri::path::BaseDirectory::Resource)
        {
            Ok(path) => path,
            Err(error) => {
                return record_failure(format!(
                    "Failed to resolve bundled ONNX Runtime resource: {error}"
                ));
            }
        };
        let result = catch_init(|| {
            ort::init_from(runtime_path.to_string_lossy().into_owned())
                .with_telemetry(false)
                .commit()
                .map(|_| ())
        });
        match result {
            Ok(()) => log::info!(
                "Initialized bundled ONNX Runtime from {}",
                runtime_path.display()
            ),
            Err(error) => record_failure(format!(
                "Failed to initialize bundled ONNX Runtime from {}: {error}",
                runtime_path.display()
            )),
        }
    }

    /// Runs `init`, converting both errors and panics into an error message.
    pub(super) fn catch_init<F, E>(init: F) -> Result<(), String>
    where
        F: FnOnce() -> Result<(), E>,
        E: std::fmt::Display,
    {
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(init)) {
            Ok(result) => result.map_err(|error| error.to_string()),
            Err(payload) => {
                let message = payload
                    .downcast_ref::<&str>()
                    .copied()
                    .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
                    .unwrap_or("unknown panic");
                Err(format!("ONNX Runtime initialization panicked: {message}"))
            }
        }
    }

    fn record_failure(error: String) {
        log::error!("{error}");
        let _ = INIT_ERROR.set(error);
    }

    #[cfg(test)]
    mod tests {
        use super::catch_init;

        #[test]
        fn catch_init_converts_errors_and_panics() {
            assert!(catch_init(|| Ok::<(), &str>(())).is_ok());
            assert_eq!(
                catch_init(|| Err::<(), _>("initializer error")),
                Err("initializer error".to_string())
            );

            let error = catch_init(|| -> Result<(), &str> {
                panic!("synthetic loader panic");
            })
            .unwrap_err();
            assert!(error.contains("synthetic loader panic"));
        }
    }
}
