//! The automations section (AUT-01): what automations may do on the Mac.

use serde::{Deserialize, Serialize};

/// The automations section of `Settings`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase", default)]
pub struct AutomationSettings {
    /// Let "Run shell script" steps run commands. Off by default: a script
    /// runs with the user's own permissions, so the user turns it on.
    pub shell_scripts: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_scripts_are_off_unless_the_file_turns_them_on() {
        assert!(!AutomationSettings::default().shell_scripts);
        let missing: AutomationSettings = serde_json::from_str("{}").unwrap();
        assert!(!missing.shell_scripts);
        let on: AutomationSettings = serde_json::from_str(r#"{ "shellScripts": true }"#).unwrap();
        assert!(on.shell_scripts);
    }
}
