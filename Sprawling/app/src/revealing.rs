// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use kernel::{Address, AxCode, AxError};

use crate::doctor::Platform;

pub(crate) fn reveal(city_root: &Path, at: &Address) -> Result<(), AxError> {
    let path = city_root.join(at.as_str());
    if !path.exists() {
        return Err(AxError::failure(
            AxCode::PathNotFound,
            "reveal a path",
            at.as_str().to_owned(),
        )
        .with_recovery("this city holds no such file; the page may be older than the tree"));
    }

    manager(&path)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(drop)
        .map_err(|err| {
            AxError::failure(AxCode::ToolUnavailable, "reveal a path", err.to_string())
                .with_recovery("this desktop has no file manager to hand a path to")
        })
}

fn manager(path: &Path) -> std::process::Command {
    match Platform::current() {
        Some(Platform::Windows) => {
            let mut command = child::command("explorer");

            command.arg(format!("/select,{}", path.display()));
            command
        }
        Some(Platform::MacOs) => {
            let mut command = child::command("open");
            command.args(["-R", &path.display().to_string()]);
            command
        }
        Some(Platform::Linux) | None => {
            let mut command = child::command("xdg-open");
            command.arg(path.parent().unwrap_or(path).display().to_string());
            command
        }
    }
}

#[cfg(test)]
#[expect(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::*;

    #[test]
    fn a_path_this_city_does_not_hold_is_refused_rather_than_opened() {
        let root = tempfile::tempdir().unwrap();
        let at = Address::parse("hall/nothing.md").unwrap();
        let refused = reveal(root.path(), &at).unwrap_err();
        assert_eq!(refused.code(), &AxCode::PathNotFound);
        assert!(
            refused.recovery().contains("no such file"),
            "{}",
            refused.recovery()
        );
    }

    #[test]
    fn the_selected_path_travels_as_one_argument() {
        let path = std::path::PathBuf::from("C:/city/hall/JOB.md");
        let command = manager(&path);
        let written: Vec<String> = command
            .get_args()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();

        let named = if cfg!(target_os = "linux") {
            "hall"
        } else {
            "JOB.md"
        };
        assert!(
            written.iter().any(|arg| arg.ends_with(named)),
            "the file manager is told which file: {written:?}"
        );
        if cfg!(target_os = "windows") {
            assert_eq!(written.len(), 1, "one argument, not two: {written:?}");
            assert!(
                written
                    .first()
                    .is_some_and(|arg| arg.starts_with("/select,")),
                "{written:?}"
            );
        }
    }
}
