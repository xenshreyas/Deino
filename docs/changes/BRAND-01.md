# BRAND-01 · Brand assets: mark, wordmark, app icon, menu bar, favicon

## Progress log (PARITY_PLAN §7.1)
| BRAND-01 Brand assets | ☑️ done on branch `feat/brand-assets` (the owner's wordmark and mark replace FND-01's placeholder "D". `scripts/brand/make-brand-assets.py` (`python3 -I`, Pillow only) turns `assets/brand/source/{wordmark,mark}.png` into alpha masks by unmixing each pixel along the white→ink axis (anti-aliased, no white fringe), samples the inks (mark blue `#6B89BC`, wordmark navy `#232F40`) and writes `assets/brand/{mark,mark-mask,wordmark-mask,app-icon}.png`, `frontend/src/ui/brand-masks/`, the tray template icons and the favicons. The app icon is a light continuous-corner body (824 px on 1024, 185 px corner) with a hairline edge and shadow, the blue mark at 62% of the body; `pnpm tauri icon assets/brand/app-icon.png` regenerated `src-tauri/icons/*` (desktop only; the android/ios output is dropped as before). Menu bar: `tray.rs` uses `icons/tray/tray-icon@2x.png` through `tauri::include_image!` as a template image on macOS (tinted for light and dark bars), the app icon elsewhere. UI: `--brand` token (`bg-brand`), `ui/brand.tsx` (`BrandMark`, `Wordmark`: CSS masks painted with `--brand` / `--fg`), used in the sidebar header, onboarding Welcome and Settings → About, and on `/design`. Favicon: `favicon-32.png` and `favicon-64.png` in every HTML entry.) | |

## Follow-ups (PARITY_PLAN §7.2)
- Owner QA: look at the icon in the Dock, Finder and Launchpad, and the menu-bar template on a light and a dark bar, after the next real build (not built on this branch).
- Owner QA: the icon body is white with a faint cool gradient; a tinted body (pale blue) is a one-line change in `app_icon()` if preferred.
- BRAND: the menu bar has no recording-state icon variant (none existed); a "mark with a dot" template could follow if the title text ("Xm left") is not enough.

## Architecture (ARCHITECTURE.md)
- Frontend, `ui/`: `brand.tsx` (`BrandMark`, `Wordmark`) paints the masks in `ui/brand-masks/` with tokens; the masks are generated, never edited by hand.
- App crate, `tray.rs`: the menu-bar icon is a macOS template image (`icons/tray/`), the app icon on other platforms.
- Repository: `assets/brand/` holds the owner's originals (`source/`) and the generated masters; `scripts/brand/make-brand-assets.py` regenerates every derived file, then `pnpm tauri icon ../assets/brand/app-icon.png` (from `frontend/`) regenerates the icon set.

## CLAUDE.md
- Patterns worth knowing: "**Brand:** the mark and wordmark come from `ui/brand.tsx` (`BrandMark`, `Wordmark`, coloured by `--brand` and `--fg`); never an `<img>` of a logo. Regenerate assets with `python3 -I scripts/brand/make-brand-assets.py`."

## Plan updates
- FND-01's placeholder "D" icons (`src-tauri/icons/svg/`, `public/{favicon.ico,icon_*.png,logo*.png}`) are removed.
