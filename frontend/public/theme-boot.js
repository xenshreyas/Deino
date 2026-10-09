// Applies the saved theme before the first paint, so a dark-mode user never
// sees a light flash while React loads. Keep in step with
// src/features/appearance/{theme,theme-store}.ts (key `deino.theme`; values
// `system`, `light`, `dark`; only an explicit choice sets `data-theme`).
(function () {
  try {
    var saved = window.localStorage.getItem('deino.theme');
    if (saved === 'light' || saved === 'dark') {
      document.documentElement.dataset.theme = saved;
    }
  } catch {
    // Storage unavailable: follow the system theme.
  }
})();
