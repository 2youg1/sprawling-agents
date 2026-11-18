// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::io::Read as _;

use serde_json::{Value, json};

use crate::error::{
    Code, Diagnostic, Error, Result, move_guide_to_backup_command, remove_file_instruction,
};

pub fn read_new_session() -> Result<Option<String>> {
    let directory = std::env::current_dir()
        .map_err(|err| workspace_error("resolve the workspace", err.to_string(), None))?;
    let path = directory.join("AGENTS.md");
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    platform_flags(&mut options);
    let mut file = match options.open("AGENTS.md") {
        Ok(file) => file,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            if let Ok(metadata) = std::fs::symlink_metadata("AGENTS.md")
                && metadata.is_dir()
            {
                return Err(non_regular_error(&path, metadata.file_type()));
            }
            return Err(workspace_error(
                "open the top-level AGENTS.md without following aliases",
                err.to_string(),
                Some(path.as_path()),
            ));
        }
    };
    let metadata = file.metadata().map_err(|err| {
        workspace_error(
            "inspect the opened top-level AGENTS.md",
            err.to_string(),
            Some(path.as_path()),
        )
    })?;
    if !metadata.is_file() {
        return Err(non_regular_error(&path, metadata.file_type()));
    }
    let mut guide = String::new();
    file.read_to_string(&mut guide).map_err(|err| {
        workspace_error(
            "read the top-level AGENTS.md as UTF-8",
            err.to_string(),
            Some(path.as_path()),
        )
    })?;
    Ok(Some(guide))
}

fn non_regular_error(path: &std::path::Path, kind: std::fs::FileType) -> Error {
    let recovery = if kind.is_dir() {
        match move_guide_to_backup_command(path) {
            Some(command) => {
                format!("preserve the directory by renaming it, then start a new session: {command}")
            }
            None => {
                "rename AGENTS.md to AGENTS.md.not-a-guide with your file manager, then start a new session"
                    .to_owned()
            }
        }
    } else {
        format!(
            "remove the non-regular AGENTS.md entry without following it, then start a new session: {}",
            remove_file_instruction(path)
        )
    };
    Error::Diagnostic(
        Diagnostic::new(
            Code::Workspace,
            "read the top-level AGENTS.md as a regular file",
            "the opened entry is not a regular file",
            recovery,
        )
        .with_path(path.display().to_string())
        .with_value(format!("{kind:?}")),
    )
}

#[cfg(unix)]
fn platform_flags(options: &mut std::fs::OpenOptions) {
    use std::os::unix::fs::OpenOptionsExt as _;
    // Published libc headers: Linux uses per-architecture values; Darwin uses 0x100/0x4.
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    const NO_FOLLOW: i32 = 0x20000;
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    const NO_FOLLOW: i32 = 0x10000;
    #[cfg(target_os = "linux")]
    const NON_BLOCK: i32 = 0x800;
    #[cfg(not(target_os = "linux"))]
    const NO_FOLLOW: i32 = 0x100;
    #[cfg(not(target_os = "linux"))]
    const NON_BLOCK: i32 = 0x4;
    options.custom_flags(NO_FOLLOW | NON_BLOCK);
}

#[cfg(windows)]
fn platform_flags(options: &mut std::fs::OpenOptions) {
    use std::os::windows::fs::OpenOptionsExt as _;
    options.custom_flags(0x0020_0000);
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
            remove_file_instruction(path)
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
    #![allow(clippy::unwrap_used, reason = "test code")]

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
        let command = crate::error::remove_file_command(path).unwrap();
        assert!(shown.contains(&command), "{shown}");
    }

    #[test]
    fn a_directory_is_preserved_by_the_repair() {
        let path = std::path::Path::new("workspace/AGENTS.md");
        let command = move_guide_to_backup_command(path).unwrap();
        assert!(command.contains("AGENTS.md.not-a-guide"));
        assert!(command.contains("AGENTS.md"));
    }
}
