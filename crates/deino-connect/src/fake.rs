//! Fakes of this crate's ports, behind `test-support`.

use std::sync::{Mutex, MutexGuard, PoisonError};

use crate::oauth::BrowserOpener;

/// A browser that records the pages it was asked to open, and opens them
/// only while `works` is true.
#[derive(Debug)]
pub struct FakeBrowser {
    opened: Mutex<Vec<String>>,
    works: bool,
}

impl FakeBrowser {
    /// A browser that opens every page.
    pub fn new() -> Self {
        Self {
            opened: Mutex::new(Vec::new()),
            works: true,
        }
    }

    /// A Mac where no browser opens.
    pub fn broken() -> Self {
        Self {
            works: false,
            ..Self::new()
        }
    }

    /// Every page opened, in order.
    pub fn opened(&self) -> Vec<String> {
        self.lock().clone()
    }

    fn lock(&self) -> MutexGuard<'_, Vec<String>> {
        self.opened.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl Default for FakeBrowser {
    fn default() -> Self {
        Self::new()
    }
}

impl BrowserOpener for FakeBrowser {
    fn open(&self, url: &str) -> bool {
        if self.works {
            self.lock().push(url.to_owned());
        }
        self.works
    }
}
