//! Opening links and folders with the macOS `open` command, for commands the
//! webview can call.
//!
//! The webview never chooses freely what `open` launches. A link must be an
//! `https:`, `http:` or `mailto:` URL ([`parse_external_url`]); a folder must
//! sit inside one of the app's own folders ([`ensure_inside`]); a System
//! Settings pane must be a plain pane name ([`settings_pane_url`]). Anything
//! else (`file:` URLs, custom app schemes, bare paths, `..` or symlink escapes)
//! is refused with an [`OpenError`] before a process starts. Deino ships on
//! macOS only, so elsewhere [`launch`] is a stub that always fails.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use deino_domain::{ErrorClass, ErrorCode};
use url::Url;

/// The only URL schemes the webview may open.
const ALLOWED_SCHEMES: [&str; 3] = ["https", "http", "mailto"];

/// The System Settings deep link that pane names are appended to.
const SETTINGS_SECURITY_URL: &str = "x-apple.systempreferences:com.apple.preference.security";

/// Why a link or folder was not opened.
#[derive(Debug, thiserror::Error)]
pub(crate) enum OpenError {
    /// The text is not an absolute URL (a bare path, for example).
    #[error("the link is not a valid URL")]
    Malformed(#[source] url::ParseError),
    /// The URL's scheme is not `https:`, `http:` or `mailto:`.
    #[error("links with the {0}: scheme cannot be opened")]
    Scheme(String),
    /// The System Settings pane name has characters a pane name never has.
    #[error("the System Settings pane name is not valid")]
    Pane,
    /// The folder does not exist (or cannot be resolved).
    #[error("the folder does not exist")]
    Missing(#[source] std::io::Error),
    /// The path resolves to something that is not a folder.
    #[error("the path is not a folder")]
    NotAFolder,
    /// The folder resolves outside every folder the app owns.
    #[error("the folder is outside Deino's folders")]
    OutsideAppFolders,
    /// `open` could not be started, failed, or does not exist (not macOS).
    #[error("could not open it with the system opener")]
    Launch(#[source] std::io::Error),
}

impl ErrorClass for OpenError {
    fn code(&self) -> ErrorCode {
        match self {
            Self::Malformed(_)
            | Self::Scheme(_)
            | Self::Pane
            | Self::NotAFolder
            | Self::OutsideAppFolders => ErrorCode::Invalid,
            Self::Missing(_) => ErrorCode::NotFound,
            Self::Launch(_) => ErrorCode::Internal,
        }
    }
}

/// Parses `raw` as a link the webview may open: an absolute `https:`,
/// `http:` or `mailto:` URL. The scheme is checked on the parsed URL, so
/// case, whitespace and encoding tricks cannot slip another scheme through.
pub(crate) fn parse_external_url(raw: &str) -> Result<Url, OpenError> {
    let url = Url::parse(raw).map_err(OpenError::Malformed)?;
    if ALLOWED_SCHEMES.contains(&url.scheme()) {
        Ok(url)
    } else {
        Err(OpenError::Scheme(url.scheme().to_owned()))
    }
}

/// Resolves `folder` (following `..` and symlinks) and returns it if it is a
/// folder inside one of `roots`. Roots that do not exist are skipped.
pub(crate) fn ensure_inside(folder: &Path, roots: &[PathBuf]) -> Result<PathBuf, OpenError> {
    let folder = folder.canonicalize().map_err(OpenError::Missing)?;
    if !folder.is_dir() {
        return Err(OpenError::NotAFolder);
    }
    let inside = roots
        .iter()
        .filter_map(|root| root.canonicalize().ok())
        .any(|root| folder.starts_with(root));
    if inside {
        Ok(folder)
    } else {
        Err(OpenError::OutsideAppFolders)
    }
}

/// The deep link for a Privacy & Security pane such as `Privacy_Microphone`.
pub(crate) fn settings_pane_url(pane: &str) -> Result<String, OpenError> {
    let valid = !pane.is_empty()
        && pane
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_');
    if valid {
        Ok(format!("{SETTINGS_SECURITY_URL}?{pane}"))
    } else {
        Err(OpenError::Pane)
    }
}

/// Opens a link from the webview in the default browser or mail app.
pub(crate) async fn open_external_url(raw: &str) -> Result<(), OpenError> {
    let url = parse_external_url(raw)?;
    launch(OsStr::new(url.as_str())).await
}

/// Runs `open <target>` and waits for it. Callers validate `target` first.
#[cfg(target_os = "macos")]
pub(crate) async fn launch(target: &OsStr) -> Result<(), OpenError> {
    let status = tokio::process::Command::new("open")
        .arg(target)
        .status()
        .await
        .map_err(OpenError::Launch)?;
    if status.success() {
        Ok(())
    } else {
        Err(OpenError::Launch(std::io::Error::other(format!(
            "open exited with {status}"
        ))))
    }
}

/// Deino opens links and folders on macOS only.
#[cfg(not(target_os = "macos"))]
#[allow(
    clippy::unused_async,
    reason = "keeps the macOS signature so callers compile on every platform"
)]
pub(crate) async fn launch(_target: &OsStr) -> Result<(), OpenError> {
    Err(OpenError::Launch(std::io::ErrorKind::Unsupported.into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn web_and_mail_links_are_allowed() {
        let cases = [
            ("https://ollama.com/download", "https"),
            ("http://localhost:11434/", "http"),
            ("HTTPS://Example.com/path?q=1#top", "https"),
            ("  https://example.com", "https"),
            ("mailto:someone@example.com?subject=Notes", "mailto"),
            ("mailto:", "mailto"),
        ];
        for (raw, scheme) in cases {
            let url = parse_external_url(raw).unwrap_or_else(|error| panic!("{raw}: {error}"));
            assert_eq!(url.scheme(), scheme, "{raw}");
        }
    }

    #[test]
    fn other_schemes_are_refused() {
        let cases = [
            ("file:///Applications/Calculator.app", "file"),
            ("FILE:///etc/passwd", "file"),
            (
                "x-apple.systempreferences:com.apple.preference.security",
                "x-apple.systempreferences",
            ),
            ("javascript:alert(1)", "javascript"),
            ("data:text/html,<script>alert(1)</script>", "data"),
            ("vscode://file/Users/me/.ssh/config", "vscode"),
            ("smb://server/share", "smb"),
            ("ftp://example.com/file", "ftp"),
            ("tel:+15555550123", "tel"),
        ];
        for (raw, scheme) in cases {
            match parse_external_url(raw) {
                Err(OpenError::Scheme(found)) => assert_eq!(found, scheme, "{raw}"),
                other => panic!("{raw}: expected a scheme error, got {other:?}"),
            }
        }
    }

    #[test]
    fn bare_paths_and_malformed_links_are_refused() {
        let cases = [
            "",
            "   ",
            "/Applications/Calculator.app",
            "~/Documents",
            "../secrets",
            "Calculator.app",
            "example.com",
            "https://",
            "http://exa mple.com",
            "://missing-scheme",
        ];
        for raw in cases {
            assert!(
                matches!(parse_external_url(raw), Err(OpenError::Malformed(_))),
                "{raw:?} should be malformed"
            );
        }
    }

    #[test]
    fn refusals_are_invalid_input() {
        for error in [
            OpenError::Scheme("file".to_owned()),
            OpenError::Pane,
            OpenError::OutsideAppFolders,
            OpenError::NotAFolder,
        ] {
            assert_eq!(error.code(), ErrorCode::Invalid, "{error}");
        }
    }

    #[test]
    fn folders_inside_an_app_folder_open() {
        let data = tempfile::tempdir().unwrap();
        let recordings = tempfile::tempdir().unwrap();
        let meeting = recordings.path().join("Standup 2026-10-11");
        std::fs::create_dir(&meeting).unwrap();
        let roots = [data.path().to_path_buf(), recordings.path().to_path_buf()];

        for folder in [meeting.as_path(), data.path(), recordings.path()] {
            let opened = ensure_inside(folder, &roots).unwrap();
            assert_eq!(opened, folder.canonicalize().unwrap());
        }
    }

    #[test]
    fn folders_outside_the_app_folders_are_refused() {
        let data = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let roots = [data.path().join("recordings"), data.path().join("missing")];
        std::fs::create_dir(&roots[0]).unwrap();
        let escape = roots[0].join("..");
        let sibling = data.path().join("recordings-other");
        std::fs::create_dir(&sibling).unwrap();

        for folder in [outside.path(), escape.as_path(), sibling.as_path()] {
            assert!(
                matches!(
                    ensure_inside(folder, &roots),
                    Err(OpenError::OutsideAppFolders)
                ),
                "{} should be refused",
                folder.display()
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn a_symlink_out_of_an_app_folder_is_refused() {
        let data = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let link = data.path().join("link");
        std::os::unix::fs::symlink(outside.path(), &link).unwrap();

        let result = ensure_inside(&link, &[data.path().to_path_buf()]);

        assert!(matches!(result, Err(OpenError::OutsideAppFolders)));
    }

    #[test]
    fn missing_folders_and_files_are_refused() {
        let data = tempfile::tempdir().unwrap();
        let file = data.path().join("audio.mp4");
        std::fs::write(&file, b"").unwrap();
        let roots = [data.path().to_path_buf()];

        let missing = ensure_inside(&data.path().join("gone"), &roots);
        let not_folder = ensure_inside(&file, &roots);

        assert!(matches!(missing, Err(OpenError::Missing(_))));
        assert!(matches!(not_folder, Err(OpenError::NotAFolder)));
    }

    #[test]
    fn settings_panes_must_be_plain_names() {
        assert_eq!(
            settings_pane_url("Privacy_Microphone").unwrap(),
            "x-apple.systempreferences:com.apple.preference.security?Privacy_Microphone"
        );
        for pane in [
            "",
            "Privacy Microphone",
            "a&b",
            "../x",
            "x\n",
            "Privacy_Mic?file:///",
        ] {
            assert!(
                matches!(settings_pane_url(pane), Err(OpenError::Pane)),
                "{pane:?}"
            );
        }
    }
}
