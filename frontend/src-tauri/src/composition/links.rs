//! `deino://` links (DEV-05): the link service over the recorder, the app
//! lock and the window, and `tauri-plugin-deep-link`, whose URLs it follows.
//!
//! The plugin is registered here, at the end of startup, rather than in
//! `lib.rs`: macOS hands a launching URL to the app after setup, so none
//! is missed, and the feature stays in its own files.

use std::error::Error;
use std::sync::Arc;

use deino_core::links::LinkService;
use tauri::{AppHandle, Manager};
use tauri_plugin_deep_link::DeepLinkExt;

use super::attach::Phase;
use crate::features::links::adapters::{AppLock, AppRecorder, AppShell, open_urls};

super::attach::attach!("links", 70, Phase::Startup(attach_links));

/// Manages the [`LinkService`] and follows every `deino://` URL.
fn attach_links(app: &AppHandle) -> Result<(), Box<dyn Error>> {
    app.manage(Arc::new(LinkService::new(
        Arc::new(AppRecorder(app.clone())),
        Arc::new(AppLock(app.clone())),
        Arc::new(AppShell(app.clone())),
    )));
    app.plugin(tauri_plugin_deep_link::init())?;
    let handle = app.clone();
    app.deep_link().on_open_url(move |event| {
        let urls = event.urls().into_iter().map(String::from).collect();
        open_urls(&handle, urls);
    });
    // Windows and Linux pass a launching URL as an argument instead.
    if let Ok(Some(urls)) = app.deep_link().get_current() {
        open_urls(app, urls.into_iter().map(String::from).collect());
    }
    Ok(())
}
