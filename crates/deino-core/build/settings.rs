//! Reads the settings sections from `src/settings/sections/*.rs`.
//!
//! Each file is one module of `settings::sections`, re-exported by a glob.
//! Each `register_section! { .. }` in it is one field of `Settings`: its
//! `///` doc, `key: Type`, `order` and optional `dialog_only` list, one per
//! line in that order (`validate` is the macro's business).

use std::fs;
use std::path::{Path, PathBuf};

/// One registered section.
#[derive(Debug)]
pub struct Section {
    /// The field name, for example `ai`.
    pub key: String,
    /// The section's type, re-exported by `settings`, for example `AiSettings`.
    pub ty: String,
    /// The `///` lines above the key, without the `///`.
    pub doc: Vec<String>,
    /// Where the field sits: sections sort by order, then key.
    pub order: u32,
    /// The JSON names of the fields only a dialog may set.
    pub dialog_only: Vec<String>,
}

/// The modules in `dir` (file stems, by name) and every section they
/// register, sorted by order and then key.
pub fn read(dir: &Path) -> Result<(Vec<String>, Vec<Section>), String> {
    let mut modules = Vec::new();
    let mut sections = Vec::new();
    for file in rust_files(dir)? {
        let text = fs::read_to_string(&file).map_err(|e| format!("{}: {e}", file.display()))?;
        let found = parse_file(&text).map_err(|e| format!("{}: {e}", file.display()))?;
        sections.extend(found);
        modules.push(stem(&file)?);
    }
    sections.sort_by(|a, b| (a.order, &a.key).cmp(&(b.order, &b.key)));
    let mut keys = std::collections::BTreeSet::new();
    if let Some(twice) = sections
        .iter()
        .find(|section| !keys.insert(section.key.as_str()))
    {
        return Err(format!("two sections are named `{}`", twice.key));
    }
    Ok((modules, sections))
}

/// The `.rs` files directly in `dir`, by name.
pub fn rust_files(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let entries = fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut files: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "rs"))
        .collect();
    files.sort();
    Ok(files)
}

/// A file's module name.
pub fn stem(file: &Path) -> Result<String, String> {
    file.file_stem()
        .and_then(|stem| stem.to_str())
        .map(str::to_owned)
        .ok_or_else(|| format!("{}: not a module name", file.display()))
}

/// Every `register_section!` in one file.
pub fn parse_file(text: &str) -> Result<Vec<Section>, String> {
    let mut sections = Vec::new();
    let mut lines = text.lines();
    while let Some(line) = lines.next() {
        if !line.trim_end().ends_with("register_section! {") || line.trim_start().starts_with("//")
        {
            continue;
        }
        let body: Vec<&str> = lines
            .by_ref()
            .take_while(|line| line.trim() != "}")
            .collect();
        sections.push(parse_body(&body)?);
    }
    Ok(sections)
}

/// One invocation's lines between its braces.
fn parse_body(body: &[&str]) -> Result<Section, String> {
    let mut lines = body.iter().map(|line| line.trim()).peekable();
    let mut doc = Vec::new();
    while let Some(text) = lines.peek().and_then(|line| line.strip_prefix("///")) {
        doc.push(text.strip_prefix(' ').unwrap_or(text).to_owned());
        lines.next();
    }
    let (key, ty) = entry(lines.next())?;
    if doc.is_empty() {
        return Err(format!("section `{key}` has no `///` doc"));
    }
    let (name, order) = entry(lines.next())?;
    if name != "order" {
        return Err(format!(
            "section `{key}`: `order: <number>,` must follow `{key}: {ty},`"
        ));
    }
    let order = order
        .parse()
        .map_err(|_| format!("section `{key}`: order `{order}` is not a number"))?;
    let mut dialog_only = Vec::new();
    for line in lines {
        match entry(Some(line))? {
            (name, _) if name == "validate" => {}
            (name, list) if name == "dialog_only" => dialog_only = quoted(&list),
            (name, _) => return Err(format!("section `{key}`: unknown entry `{name}`")),
        }
    }
    check_key(&key)?;
    Ok(Section {
        key,
        ty,
        doc,
        order,
        dialog_only,
    })
}

/// `name: value,` as `(name, value)`.
fn entry(line: Option<&str>) -> Result<(String, String), String> {
    let line = line.ok_or("a register_section! entry is missing")?;
    let (name, value) = line
        .strip_suffix(',')
        .and_then(|line| line.split_once(':'))
        .ok_or_else(|| format!("expected `name: value,`, found `{line}`"))?;
    Ok((name.trim().to_owned(), value.trim().to_owned()))
}

/// The string literals of `["a", "b"]`.
fn quoted(list: &str) -> Vec<String> {
    list.split('"')
        .skip(1)
        .step_by(2)
        .map(str::to_owned)
        .collect()
}

/// A key must be a `snake_case` field name.
fn check_key(key: &str) -> Result<(), String> {
    let valid = key.starts_with(|c: char| c.is_ascii_lowercase())
        && key
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
    if valid {
        Ok(())
    } else {
        Err(format!(
            "section key `{key}` is not a snake_case field name"
        ))
    }
}

/// `auto_export` → `autoExport`, the field's JSON name under `rename_all = "camelCase"`.
pub fn camel(key: &str) -> String {
    let mut words = key.split('_');
    let first = words.next().unwrap_or_default().to_owned();
    words.fold(first, |mut out, word| {
        let mut chars = word.chars();
        if let Some(head) = chars.next() {
            out.push(head.to_ascii_uppercase());
            out.push_str(chars.as_str());
        }
        out
    })
}
