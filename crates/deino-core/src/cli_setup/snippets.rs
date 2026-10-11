//! The snippets that connect Claude to the CLI's MCP server (`deino mcp`).

use serde_json::json;

/// The name Claude knows the server by.
pub const SERVER_NAME: &str = "deino";

/// The Claude Code command that registers the server (for this project;
/// `--scope user` after `add` registers it for every project).
pub fn claude_code(command: &str) -> String {
    format!(
        "claude mcp add {SERVER_NAME} -- {} mcp",
        shell_quoted(command)
    )
}

/// The `mcpServers` entry for Claude Desktop's
/// `~/Library/Application Support/Claude/claude_desktop_config.json`.
/// Desktop apps do not read the shell's `PATH`, so it names the full path.
pub fn claude_desktop(command: &str) -> String {
    let config = json!({
        "mcpServers": {
            SERVER_NAME: { "command": command, "args": ["mcp"] }
        }
    });
    // A `json!` value always serializes.
    serde_json::to_string_pretty(&config).unwrap_or_default()
}

/// `text` as one shell word: as is when it is safe, else single-quoted.
fn shell_quoted(text: &str) -> String {
    let safe = text
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || "/._-+~".contains(character));
    if safe && !text.is_empty() {
        text.to_owned()
    } else {
        format!("'{}'", text.replace('\'', r"'\''"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_claude_code_command_quotes_only_when_needed() {
        assert_eq!(
            claude_code("/Users/me/.local/bin/deino"),
            "claude mcp add deino -- /Users/me/.local/bin/deino mcp"
        );
        assert_eq!(
            claude_code("/Volumes/My Apps/Deino.app/Contents/MacOS/cli"),
            "claude mcp add deino -- '/Volumes/My Apps/Deino.app/Contents/MacOS/cli' mcp"
        );
    }

    #[test]
    fn the_desktop_config_runs_the_server_by_full_path() {
        let config: serde_json::Value =
            serde_json::from_str(&claude_desktop("/Users/me/.local/bin/deino")).unwrap();

        assert_eq!(
            config,
            json!({ "mcpServers": { "deino": { "command": "/Users/me/.local/bin/deino", "args": ["mcp"] } } })
        );
    }
}
