//! The finished lines of a notes answer that is still being written
//! (streamed notes, ADR 2026-10-30-streamed-notes).
//!
//! [`complete_lines`] reads the JSON written so far, tolerating where it
//! stops, and keeps only what can no longer change: a section once its
//! heading is closed, a block once its `origin` is (the field after its
//! text, so who wrote the line is known), a detail once its text is. Fields
//! are looked up by name, so their order does not matter. Evidence is left
//! out: citations are hidden while the notes are written and the saved
//! version carries them. What it returns is always a prefix of the finished
//! answer (`partial_tests`), so lines appear in order and never change.

use serde::de::DeserializeOwned;

use super::wire::{EnhancedBlock, EnhancedDetail, EnhancedDoc, EnhancedSection};

/// A JSON value as far as it was written.
enum Node {
    /// A string: its text once the closing quote arrived.
    Str(Option<String>),
    /// An object's fields so far, each with a closed key.
    Obj(Vec<(String, Node)>),
    /// An array's items so far.
    Arr(Vec<Node>),
    /// A number, `true`, `false` or `null`: never needed here.
    Other,
}

impl Node {
    fn field(&self, name: &str) -> Option<&Node> {
        match self {
            Self::Obj(fields) => fields.iter().find(|(key, _)| key == name).map(|(_, v)| v),
            _ => None,
        }
    }

    fn text(&self, name: &str) -> Option<&str> {
        match self.field(name) {
            Some(Self::Str(Some(text))) => Some(text),
            _ => None,
        }
    }

    fn items(&self, name: &str) -> &[Node] {
        match self.field(name) {
            Some(Self::Arr(items)) => items,
            _ => &[],
        }
    }
}

/// The lines of `answer` that are final (see the module docs).
pub fn complete_lines(answer: &str) -> EnhancedDoc {
    let root = Reader::new(answer).value().unwrap_or(Node::Other);
    let mut sections = Vec::new();
    for section in root.items("sections") {
        let Some(heading) = section.text("heading") else {
            break;
        };
        sections.push(EnhancedSection {
            heading: heading.to_owned(),
            blocks: section.items("blocks").iter().map_while(block).collect(),
        });
    }
    EnhancedDoc { sections }
}

/// How many lines `doc` shows: headings, blocks and details.
pub fn line_count(doc: &EnhancedDoc) -> usize {
    doc.sections
        .iter()
        .map(|section| {
            1 + section
                .blocks
                .iter()
                .map(|block| 1 + block.details.len())
                .sum::<usize>()
        })
        .sum()
}

/// A block whose kind, text and origin are all closed.
fn block(node: &Node) -> Option<EnhancedBlock> {
    Some(EnhancedBlock {
        kind: word(node.text("kind")?)?,
        text: node.text("text")?.to_owned(),
        origin: word(node.text("origin")?)?,
        evidence: Vec::new(),
        details: node
            .items("details")
            .iter()
            .map_while(|detail| {
                Some(EnhancedDetail {
                    text: detail.text("text")?.to_owned(),
                    evidence: Vec::new(),
                })
            })
            .collect(),
    })
}

/// An enum written as a JSON string (`"bullet"`, `"user"`).
fn word<T: DeserializeOwned>(text: &str) -> Option<T> {
    serde_json::from_value(serde_json::Value::String(text.to_owned())).ok()
}

/// A forgiving reader: every value it cannot finish is cut where the
/// text ends.
struct Reader<'a> {
    text: &'a str,
    at: usize,
}

impl<'a> Reader<'a> {
    fn new(text: &'a str) -> Self {
        Self { text, at: 0 }
    }

    fn peek(&self) -> Option<u8> {
        self.text.as_bytes().get(self.at).copied()
    }

    fn skip_space(&mut self) {
        while self.peek().is_some_and(|byte| byte.is_ascii_whitespace()) {
            self.at += 1;
        }
    }

    fn value(&mut self) -> Option<Node> {
        self.skip_space();
        match self.peek()? {
            b'{' => Some(self.object()),
            b'[' => Some(self.array()),
            b'"' => Some(Node::Str(self.string())),
            _ => self.scalar(),
        }
    }

    fn object(&mut self) -> Node {
        self.at += 1;
        let mut fields = Vec::new();
        loop {
            self.skip_space();
            match self.peek() {
                Some(b'}') | None => break,
                Some(b',') => self.at += 1,
                Some(b'"') => match self.field() {
                    Some(field) => fields.push(field),
                    None => break,
                },
                Some(_) => self.at = self.text.len(),
            }
        }
        if self.peek() == Some(b'}') {
            self.at += 1;
        }
        Node::Obj(fields)
    }

    /// `"key": value`, or `None` when the text ends before the value starts.
    fn field(&mut self) -> Option<(String, Node)> {
        let key = self.string()?;
        self.skip_space();
        if self.peek()? != b':' {
            self.at = self.text.len();
            return None;
        }
        self.at += 1;
        Some((key, self.value()?))
    }

    fn array(&mut self) -> Node {
        self.at += 1;
        let mut items = Vec::new();
        loop {
            self.skip_space();
            match self.peek() {
                Some(b']') | None => break,
                Some(b',') => self.at += 1,
                Some(_) => match self.value() {
                    Some(item) => items.push(item),
                    None => break,
                },
            }
        }
        if self.peek() == Some(b']') {
            self.at += 1;
        }
        Node::Arr(items)
    }

    /// A string's text once it is closed (escapes decoded), else `None`
    /// with the reader at the end.
    fn string(&mut self) -> Option<String> {
        let start = self.at;
        self.at += 1;
        while let Some(byte) = self.peek() {
            self.at += if byte == b'\\' { 2 } else { 1 };
            if byte == b'"' {
                return self
                    .text
                    .get(start..self.at)
                    .and_then(|quoted| serde_json::from_str(quoted).ok());
            }
        }
        self.at = self.text.len();
        None
    }

    fn scalar(&mut self) -> Option<Node> {
        let start = self.at;
        while self
            .peek()
            .is_some_and(|byte| !matches!(byte, b',' | b'}' | b']') && !byte.is_ascii_whitespace())
        {
            self.at += 1;
        }
        (self.at > start).then_some(Node::Other)
    }
}

#[cfg(test)]
mod partial_tests;
