# deino-export

## Role

Deino's meeting exports as pure renderers (EXP-01, EXP-02, ADR 0025): a
typed `MeetingExport` and `RenderOptions` in, a file's bytes out.

## Owns

- `MeetingExport` (the DTO `deino-core::export` assembles from the
  repositories), `ExportFormat`, `ExportSections`/`Section`, `LinkStyle`,
  `RenderOptions`, `render()`.
- The formats: `markdown` (YAML frontmatter Obsidian reads as properties,
  Obsidian wiki links and tags, transcript sidecar and people pages, `owner()`
  to recognise Deino's own files), `text`, `json` (`deino.meeting` v1),
  `subtitles` (SRT, WebVTT), `html` (the rich clipboard fragment and the
  preview), `docx` (`docx-rs`, re-deflated), `pdf` (Typst with
  `template.typ` and the bundled Inter), `csv` (meetings, action items,
  people), `mailto`.
- `doc`: notes Markdown parsed once into typed `Block`s (pulldown-cmark).
- `naming`: `NameTemplate` (`{date} {title}.md`, `{yyyy}/{mm}`) and
  `sanitize`, so no title can escape the export folder.
- `fonts/`: Inter Regular, SemiBold, Italic and SemiBold Italic (static
  instances of `@fontsource-variable/inter` 5.3, latin + latin-ext; SIL OFL
  1.1 in `fonts/OFL.txt`).

## Does Not Own

- Reading the database, notes and settings, and writing files: `deino-core`
  (`export`).
- The pipeline stage: `deino-pipeline` (`auto_export`).
- Dialogs, the clipboard and opening the mail app: the app crate.

## Invariants

- Pure: no I/O, no clock, no time zone, no system fonts. The same input
  gives the same bytes (the PDF test checks it).
- Meeting text is always escaped for the target format, and never reaches
  the PDF as Typst markup (it travels as JSON data).
- Renderer errors never quote meeting content.
- No `tauri`, no `deino-net`.
