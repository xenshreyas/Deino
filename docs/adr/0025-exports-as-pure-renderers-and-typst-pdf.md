# 0025. Export through pure renderers over one DTO, and typeset PDFs with Typst

- **Status:** Accepted
- **Date:** 2026-10-09
- **Card:** EXP-01 (exports), EXP-02 (auto-export and the account export)

## Context

EXP-01 asks for every meeting in seven formats (Markdown with frontmatter,
plain text, JSON, SRT, WebVTT, DOCX, PDF), a rich-text copy, a `mailto:`
draft and a preview; EXP-02 adds an auto-export into a notes vault after
every meeting (with Obsidian links) and a zip of the whole account. The same
content is shaped six ways, written from three places (a command, a pipeline
stage, a background job), and has to be golden-tested on the FND-04
fixtures. Notes are stored as Markdown today and as TipTap documents from
NOT-01 on (`deino-doc`).

The PDF is the hard part: it must look as good as the app (Inter, the
design tokens), paginate, run without a window (the pipeline and the account
export have none), and be testable in CI.

## Decision

**One DTO, pure renderers, one assembler.**

- `crates/deino-export` (no Tauri, no network, no I/O) owns
  `MeetingExport` (title, local start time, duration, attendees, tags,
  folders, notes and "my notes" as Markdown, action items, a speaker-labelled
  transcript) and a renderer per format: `render(format, &export,
  &RenderOptions)`. Options are the chosen sections and the link style
  (plain or Obsidian). Notes Markdown is parsed once (pulldown-cmark) into
  typed blocks that the HTML, DOCX, PDF, text and Markdown writers walk.
  Every piece of meeting text is escaped for its target.
- `deino-core::export::ExportAssembler` reads a meeting through the
  repositories' ports into the DTO, resolving names (person, "Me", label)
  and converting times with a `LocalZone` port. Notes come through the
  `NoteRenderer` port: `DocNoteRenderer` writes the notepad's TipTap
  document as Readable Markdown with `deino-doc`, and the AI notes are the
  legacy summary's Markdown until NOT-02.
- Writes go through ports the app implements: the save and folder dialogs
  (`tauri-plugin-dialog`, opened in Rust so the webview never names a path;
  `export.folder` cannot be set by a settings patch), the clipboard
  (`arboard`, HTML plus plain text), the URL opener (`mailto:`), and a
  `Vault` (atomic writes) for files.
- Auto-export is the `auto_export` pipeline stage (order 99). It only ever
  overwrites a file whose frontmatter carries the meeting's `deino-id`; a
  stranger's file of the same name pushes the note to `… (2).md`. The
  account export is one tracked task streaming meetings to a blocking zip
  writer through a bounded channel.

**PDF: Typst, in-process.** `typst` + `typst-pdf` 0.15.1 (Apache-2.0)
compile a fixed template (`pdf/template.typ`) against the meeting as JSON
data in a private `World` that holds only the template, `data.json` and four
bundled Inter TTFs (static instances of `@fontsource-variable/inter`, OFL).
Meeting text never enters the markup: the template reads strings from JSON
and places them as text, so nothing a meeting says can change the document.

## Alternatives

- **WKWebView `createPDF` of an HTML template.** Same HTML as the preview
  and the clipboard, and no new dependency. Rejected: `createPDF` returns
  one tall page (real pagination needs `NSPrintOperation` driven off the
  main thread), it needs a live (hidden) webview, so the pipeline and the
  account export cannot make PDFs, it is macOS-only, and it cannot be tested
  on the Linux CI runner. Output would depend on WebKit's version.
- **`printpdf` (or `genpdf` on it).** Pure Rust and small, but a PDF drawing
  API: line breaking, lists, tables, page breaks and font shaping would all
  be ours to write. `genpdf` does some of that but is unmaintained (2021)
  and typographically basic.
- **Shelling out to a renderer** (Chrome headless, `wkhtmltopdf`, LaTeX):
  a runtime binary, larger than Typst, GPL in some cases, and against D11's
  direction of fewer sidecars.

Typst costs about 450 crates in `deino-export`'s tree (all permissive:
`cargo deny check licenses` passes) and two minutes on a cold debug build,
cached by sccache afterwards; the release binary grows by roughly 10 MB.
In exchange the PDF is deterministic (a test checks equal bytes), tagged,
paginated with running footers, and identical on every machine.

**DOCX:** `docx-rs` (MIT). It stores parts uncompressed, so the package is
re-deflated with `zip` (a 1.4 MB transcript document becomes 65 KB). Fonts
are named on every run (Helvetica Neue): Quick Look ignores document
defaults on unstyled paragraphs, and Inter is not installed on most Macs.

## Consequences

- A format is one module and one match arm in `render`; golden files
  (insta) pin Markdown, TXT, JSON, SRT, VTT, HTML and CSV; DOCX and PDF are
  read back in tests (zip parts; PDF text extraction).
- The template is the PDF's only design surface: change `template.typ` and
  look at `DEINO_EXPORT_SAMPLES=… cargo test -p deino-export -- --ignored
  write_samples`.
- Typst's API moves between minor versions; it is pinned and confined to
  `pdf/world.rs` and `pdf.rs`.
- Glyphs outside Inter's latin and latin-ext subsets (emoji, CJK) render as
  missing glyphs in PDFs; adding a fallback font is a follow-up.
- When NOT-02 stores enhanced notes as documents, only `DocNoteRenderer`
  changes.
