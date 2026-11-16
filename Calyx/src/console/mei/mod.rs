// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Mei interactive entry: resolve the Zig `calyx` face and replace this
//! process with it (`Calyx/Spec.lean` D2, D6, D7).

use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Hands the interactive Mei face to the Zig `calyx` binary. The wall clock
/// is sampled inside that binary; `clock` is accepted so callers keep a
/// stable signature.
pub fn verb(replay: Option<&str>, _clock: super::terminal::Clock) -> ExitCode {
    match calyx_program() {
        Ok(program) => exec_calyx(&program, replay),
        Err(diagnostic) => {
            eprintln!("{diagnostic}");
            ExitCode::FAILURE
        }
    }
}

fn calyx_program() -> Result<PathBuf, String> {
    if let Some(forced) = std::env::var_os("CALYX_BIN") {
        let path = PathBuf::from(forced);
        if path.is_file() {
            return Ok(path);
        }
        return Err(face_missing(
            "open CALYX_BIN",
            path.display().to_string(),
            "set CALYX_BIN to the Zig `calyx` binary from `zig build` / cargo target dir",
        ));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let sibling = dir.join(if cfg!(windows) {
                "calyx.exe"
            } else {
                "calyx"
            });
            if sibling.is_file() && sibling != exe {
                return Ok(sibling);
            }
        }
    }
    which("calyx").ok_or_else(|| {
        face_missing(
            "find the Zig calyx face",
            "calyx",
            "put `calyx` beside this binary, on PATH, or set CALYX_BIN (built by Calyx/build.zig via sprawling-calyx build.rs)",
        )
    })
}

fn which(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn exec_calyx(program: &Path, replay: Option<&str>) -> ExitCode {
    let mut command = child::command(program);
    if let Some(path) = replay {
        command.arg("--replay").arg(path);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt as _;
        let error = command.exec();
        eprintln!(
            "{}",
            face_missing(
                "exec the Zig calyx face",
                format!("{program}: {error}", program = program.display()),
                "rebuild Calyx (`cargo build -p sprawling-calyx` or `zig build` in Calyx/) so `calyx` is present and executable",
            )
        );
        ExitCode::FAILURE
    }
    #[cfg(not(unix))]
    {
        match command.status() {
            Ok(status) => {
                let code = status.code().unwrap_or(1);
                let clipped = u8::try_from(code).unwrap_or(1);
                ExitCode::from(clipped)
            }
            Err(error) => {
                eprintln!(
                    "{}",
                    face_missing(
                        "start the Zig calyx face",
                        format!("{program}: {error}", program = program.display()),
                        "rebuild Calyx (`cargo build -p sprawling-calyx` or `zig build` in Calyx/) so `calyx` is present and executable",
                    )
                );
                ExitCode::FAILURE
            }
        }
    }
}

fn face_missing(action: &str, subject: impl Into<String>, recovery: &str) -> String {
    let subject = subject.into();
    format!(
        "```toml\n\
         code = \"E_CALYX_FACE\"\n\
         action = \"{action}\"\n\
         subject = \"{subject}\"\n\
         path = \"{subject}\"\n\
         value = \"{subject}\"\n\
         rule = \"Mei interactive entry runs the Zig calyx binary, never a Rust shell\"\n\
         recovery = \"{recovery}\"\n\
         ```"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn which_finds_nothing_for_a_missing_name() {
        assert!(which("calyx-face-not-on-path-xyz-9f2a").is_none());
    }

    #[test]
    fn face_missing_is_a_structured_block() {
        let text = face_missing("find", "calyx", "rebuild");
        assert!(text.contains("E_CALYX_FACE"), "{text}");
        assert!(text.contains("```toml"), "{text}");
    }

    #[test]
    fn missing_calyx_bin_path_uses_the_face_diagnostic() {
        let err = face_missing(
            "open CALYX_BIN",
            "/no/such/calyx",
            "set CALYX_BIN to the Zig `calyx` binary",
        );
        assert!(err.contains("E_CALYX_FACE"), "{err}");
        assert!(err.contains("open CALYX_BIN"), "{err}");
    }
}
