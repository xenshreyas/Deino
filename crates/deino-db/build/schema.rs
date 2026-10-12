//! Reads the tables a set of migrations leaves behind, from their SQL text.
//!
//! The migrations are replayed statement by statement: `CREATE TABLE` adds a
//! table, `DROP TABLE` removes it and `ALTER TABLE .. RENAME TO` renames it.
//! Temporary tables (a migration's scratch space) and virtual tables (FTS5
//! indexes, whose content table reports their changes) are not tracked. A
//! table's doc is the `--` comment block directly above its first `CREATE
//! TABLE`, and its position is the order in which tables first appear.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

/// A table the migrations create and keep.
pub struct TableDef {
    /// The SQL name, for example `meeting_notes`.
    pub name: String,
    /// The comment lines above its `CREATE TABLE`, without the `--`.
    pub doc: Vec<String>,
}

/// One schema-changing statement.
enum Change {
    Create { name: String, doc: Vec<String> },
    Drop(String),
    Rename { from: String, to: String },
}

/// Every table that `dirs`' migrations leave behind, in order of first appearance.
pub fn tables(dirs: &[&str]) -> Result<Vec<TableDef>, String> {
    let mut migrations = Vec::new();
    for dir in dirs {
        for file in migration_files(Path::new(dir))? {
            let sql = fs::read_to_string(&file).map_err(|e| format!("{}: {e}", file.display()))?;
            migrations.push(sql);
        }
    }
    Ok(replay(migrations.iter().map(String::as_str)))
}

/// Every table that `migrations`, applied in order, leave behind.
pub fn replay<'a>(migrations: impl IntoIterator<Item = &'a str>) -> Vec<TableDef> {
    let mut history: Vec<TableDef> = Vec::new();
    let mut live: BTreeSet<String> = BTreeSet::new();
    for sql in migrations {
        for change in statements(sql).into_iter().filter_map(classify) {
            apply(change, &mut history, &mut live);
        }
    }
    history.retain(|table| live.contains(&table.name));
    history
}

/// The `.sql` files in `dir`, in the order they are applied (by name).
fn migration_files(dir: &Path) -> Result<Vec<std::path::PathBuf>, String> {
    let entries = fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut files: Vec<_> = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "sql"))
        .collect();
    files.sort();
    Ok(files)
}

fn apply(change: Change, history: &mut Vec<TableDef>, live: &mut BTreeSet<String>) {
    let seen = |history: &[TableDef], name: &str| history.iter().any(|t| t.name == name);
    match change {
        Change::Create { name, doc } => {
            if !seen(history, &name) {
                history.push(TableDef {
                    name: name.clone(),
                    doc,
                });
            }
            live.insert(name);
        }
        Change::Drop(name) => {
            live.remove(&name);
        }
        Change::Rename { from, to } => {
            live.remove(&from);
            if !seen(history, &to) {
                let doc = history
                    .iter()
                    .find(|t| t.name == from)
                    .map(|t| t.doc.clone())
                    .unwrap_or_default();
                history.push(TableDef {
                    name: to.clone(),
                    doc,
                });
            }
            live.insert(to);
        }
    }
}

/// A statement's text and the comment block directly above it.
struct Statement {
    text: String,
    doc: Vec<String>,
}

/// Splits `sql` into statements at `;`, skipping comments and quoted text.
/// A trigger body splits into fragments too, which [`classify`] ignores.
fn statements(sql: &str) -> Vec<Statement> {
    let mut lexer = Lexer::default();
    for line in sql.lines() {
        lexer.line(line);
    }
    lexer.finish()
}

/// The state of [`statements`] between lines.
#[derive(Default)]
struct Lexer {
    done: Vec<Statement>,
    text: String,
    pending_doc: Vec<String>,
    doc: Vec<String>,
    in_quote: bool,
}

impl Lexer {
    fn line(&mut self, line: &str) {
        if !self.in_quote && self.text.trim().is_empty() {
            if let Some(comment) = line.trim_start().strip_prefix("--") {
                self.pending_doc
                    .push(comment.strip_prefix(' ').unwrap_or(comment).to_owned());
                return;
            }
            if line.trim().is_empty() {
                self.pending_doc.clear();
                return;
            }
        }
        self.code(line);
        self.text.push('\n');
    }

    /// Adds the code of `line`, ending statements at unquoted `;`.
    fn code(&mut self, line: &str) {
        let mut chars = line.chars().peekable();
        while let Some(c) = chars.next() {
            if !self.in_quote && c == '-' && chars.peek() == Some(&'-') {
                return;
            }
            if c == '\'' {
                self.in_quote = !self.in_quote;
            }
            if self.text.trim().is_empty() && !c.is_whitespace() {
                self.doc = std::mem::take(&mut self.pending_doc);
            }
            if c == ';' && !self.in_quote {
                self.end();
            } else {
                self.text.push(c);
            }
        }
    }

    fn end(&mut self) {
        let text = std::mem::take(&mut self.text);
        let doc = std::mem::take(&mut self.doc);
        self.done.push(Statement { text, doc });
    }

    fn finish(mut self) -> Vec<Statement> {
        if !self.text.trim().is_empty() {
            self.end();
        }
        self.done
    }
}

/// The schema change `statement` makes, if it makes one this module tracks.
fn classify(statement: Statement) -> Option<Change> {
    let words: Vec<String> = statement
        .text
        .split(|c: char| c.is_whitespace() || c == '(')
        .filter(|word| !word.is_empty())
        .map(str::to_owned)
        .collect();
    let upper: Vec<String> = words.iter().map(|w| w.to_ascii_uppercase()).collect();
    let keywords: Vec<&str> = upper.iter().map(String::as_str).collect();
    let name_at = |index: usize| words.get(index).map(|word| unquote(word));
    match keywords.as_slice() {
        ["CREATE", "TABLE", "IF", "NOT", "EXISTS", ..] => Some(Change::Create {
            name: name_at(5)?,
            doc: statement.doc,
        }),
        ["CREATE", "TABLE", ..] => Some(Change::Create {
            name: name_at(2)?,
            doc: statement.doc,
        }),
        ["DROP", "TABLE", "IF", "EXISTS", ..] => Some(Change::Drop(name_at(4)?)),
        ["DROP", "TABLE", ..] => Some(Change::Drop(name_at(2)?)),
        ["ALTER", "TABLE", _, "RENAME", "TO", ..] => Some(Change::Rename {
            from: name_at(2)?,
            to: name_at(5)?,
        }),
        _ => None,
    }
}

/// `name` without SQL quotes.
fn unquote(name: &str) -> String {
    name.trim_matches(|c| matches!(c, '"' | '`' | '[' | ']' | '\''))
        .to_owned()
}
