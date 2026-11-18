// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use serde_json::{Value, json};

use crate::error::{Code, Diagnostic, Error, Result, remove_file_command};

pub fn read_new_session() -> Result<Option<String>> {
    let directory = std::env::current_dir()
        .map_err(|err| workspace_error("resolve the workspace", err.to_string(), None))?;
    let path = directory.join("AGENTS.md");
    match std::fs::read_to_string(&path) {
        Ok(guide) => Ok(Some(guide)),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(workspace_error(
            "read the top-level AGENTS.md",
            err.to_string(),
            Some(path.as_path()),
        )),
    }
}

pub fn opening_content(guide: Option<&str>, prompt: &str) -> Value {
    match guide {
        Some(body) => json!([
            {"type": "text", "text": format!("<workspace-instructions source=\"AGENTS.md\">\n{body}\n</workspace-instructions>")},
            {"type": "text", "text": prompt}
        ]),
        None => Value::String(prompt.to_owned()),
    }
}

fn workspace_error(action: &str, subject: String, path: Option<&std::path::Path>) -> Error {
    let recovery = match path {
        Some(path) => format!(
            "rewrite the guide as UTF-8, or delete it and start a new session: {}",
            remove_file_command(path)
        ),
        None => "keep the workspace readable and AGENTS.md as UTF-8".to_owned(),
    };
    let diagnostic = Diagnostic::new(Code::Workspace, action, subject, recovery);
    Error::Diagnostic(match path {
        Some(path) => diagnostic.with_path(path.display().to_string()),
        None => diagnostic,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opening_has_one_optional_guide_and_one_prompt() {
        assert_eq!(
            opening_content(Some("rule"), "task"),
            json!([
                {"type": "text", "text": "<workspace-instructions source=\"AGENTS.md\">\nrule\n</workspace-instructions>"},
                {"type": "text", "text": "task"}
            ])
        );
        assert_eq!(opening_content(None, "task"), json!("task"));
    }

    #[test]
    fn an_unreadable_guide_names_a_runnable_repair() {
        let path = std::path::Path::new("work space/AGENTS.md");
        let shown = workspace_error(
            "read the top-level AGENTS.md",
            "invalid UTF-8".to_owned(),
            Some(path),
        )
        .to_string();
        assert!(shown.contains("path = work space/AGENTS.md"), "{shown}");
        assert!(shown.contains("start a new session"), "{shown}");
        let command = remove_file_command(path).replace('\\', "\\\\");
        assert!(shown.contains(&command), "{shown}");
    }
}
