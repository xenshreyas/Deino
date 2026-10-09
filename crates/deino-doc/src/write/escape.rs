//! Backslash escapes: just enough that text never turns into syntax.
//!
//! Escaping everything (as anarlog's printer does before un-escaping it all
//! again) makes notes unreadable for people and LLMs; escaping nothing makes
//! `* item` or `snake_case_name` change meaning on the way back. Each rule
//! below exists for a construct that would otherwise start.

/// Escapes `text` for a Markdown paragraph. `line_start` is true when the
/// text begins a line, where `#`, `>`, `-`, `+`, `=` and `1.` start blocks;
/// leading whitespace there is dropped, as the parser would drop it.
pub(super) fn escape_text(text: &str, line_start: bool) -> String {
    let text = if line_start { text.trim_start() } else { text };
    let chars: Vec<char> = text.chars().collect();
    let mut escaped = String::with_capacity(text.len() + 8);
    for (index, &character) in chars.iter().enumerate() {
        let before = index.checked_sub(1).and_then(|i| chars.get(i)).copied();
        let after = chars.get(index + 1).copied();
        let rest = chars.get(index..).unwrap_or_default();
        if needs_escape(character, before, after, rest)
            || (line_start && starts_block(&chars, index))
        {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    escaped
}

/// Characters that start inline syntax wherever they are.
fn needs_escape(character: char, before: Option<char>, after: Option<char>, rest: &[char]) -> bool {
    let alphanumeric = |c: Option<char>| c.is_some_and(char::is_alphanumeric);
    match character {
        '\\' | '*' | '`' | '[' | ']' => true,
        // `snake_case` is safe: `_` only emphasises at word boundaries.
        '_' => !(alphanumeric(before) && alphanumeric(after)),
        // A single `~` is plain text; `~~` is strikethrough.
        '~' => before == Some('~') || after == Some('~'),
        // `<a`, `</`, `<!`, `<?` open HTML or an autolink; `< 7` and `<3` do not.
        '<' => after.is_some_and(|c| c.is_ascii_alphabetic() || matches!(c, '/' | '!' | '?')),
        '&' => is_entity(rest),
        _ => false,
    }
}

/// True for `&name;` and `&#123;`, which the parser would decode.
fn is_entity(rest: &[char]) -> bool {
    let body: Vec<char> = rest
        .iter()
        .skip(1)
        .take_while(|c| c.is_ascii_alphanumeric() || **c == '#')
        .copied()
        .collect();
    !body.is_empty() && rest.get(body.len() + 1) == Some(&';')
}

/// True when the character at `index` would start a block at the beginning
/// of a line: a heading, quote, list, rule or setext underline, or the `.`
/// or `)` of an ordered-list number.
fn starts_block(chars: &[char], index: usize) -> bool {
    let Some(&character) = chars.get(index) else {
        return false;
    };
    if index == 0 {
        return matches!(character, '#' | '>' | '-' | '+' | '=');
    }
    let number = chars.get(..index).unwrap_or_default();
    matches!(character, '.' | ')')
        && number.len() <= 9
        && number.iter().all(char::is_ascii_digit)
        && chars.get(index + 1).is_none_or(|c| c.is_whitespace())
}

/// Escapes link or image text, where only brackets and backslashes matter.
pub(super) fn escape_link_text(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        if matches!(character, '\\' | '[' | ']') {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    escaped
}

/// Escapes a double-quoted link or image title.
pub(super) fn escape_title(title: &str) -> String {
    title.replace('\\', "\\\\").replace('"', "\\\"")
}
