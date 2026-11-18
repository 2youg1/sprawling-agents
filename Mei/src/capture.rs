// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::io::Read;
use std::path::{Path, PathBuf};

use crate::error::{Code, Diagnostic, Error, Result};

pub const INLINE_BYTES: u64 = 16 * 1024;

pub struct Store {
    root: PathBuf,
    inline: Vec<PathBuf>,
}

impl Store {
    pub fn create() -> Result<Store> {
        let root = match std::env::var_os("MEI_CAPTURE_DIR") {
            Some(path) if !path.is_empty() => absolute(PathBuf::from(path))?,
            Some(_) => {
                return Err(failure(
                    "create the capture directory",
                    "MEI_CAPTURE_DIR is empty",
                    "leave MEI_CAPTURE_DIR unset or provide a new run directory",
                    None,
                ));
            }
            None => unique_temporary_directory()?,
        };
        if let Some(parent) = root.parent() {
            std::fs::create_dir_all(parent).map_err(|err| {
                failure(
                    "create the capture parent",
                    err.to_string(),
                    "keep the run's parent directory writable",
                    Some(parent),
                )
            })?;
        }
        std::fs::create_dir(&root).map_err(|err| {
            failure(
                "create a new capture directory",
                err.to_string(),
                "provide a directory that does not already exist; Mei refuses to reuse or delete an existing tree",
                Some(&root),
            )
        })?;
        Ok(Store {
            root,
            inline: Vec::new(),
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn observe(&mut self, path: String) -> Result<String> {
        self.reclaim_inline()?;
        let path = PathBuf::from(path);
        match read(&path)? {
            Observation::Inline(text) => {
                self.inline.push(path);
                Ok(text)
            }
            Observation::Spilled(notice) => Ok(notice),
        }
    }

    pub fn complete(&mut self) -> Result<()> {
        self.reclaim_inline()?;
        match std::fs::remove_dir(&self.root) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(err) if err.kind() == std::io::ErrorKind::DirectoryNotEmpty => Ok(()),
            Err(err) => Err(failure(
                "remove an empty capture directory",
                err.to_string(),
                "keep the completed run directory removable; spilled files remain owned by the session or caller",
                Some(&self.root),
            )),
        }
    }

    pub fn discard(&mut self) -> Result<()> {
        match std::fs::remove_dir_all(&self.root) {
            Ok(()) => {
                self.inline.clear();
                Ok(())
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                self.inline.clear();
                Ok(())
            }
            Err(err) => Err(failure(
                "discard an uncommitted capture directory",
                err.to_string(),
                "stop processes holding the run's files, then remove the named directory",
                Some(&self.root),
            )),
        }
    }

    fn reclaim_inline(&mut self) -> Result<()> {
        while let Some(path) = self.inline.pop() {
            match std::fs::remove_file(&path) {
                Ok(()) => {}
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                Err(err) => {
                    return Err(failure(
                        "reclaim an inline capture",
                        err.to_string(),
                        "stop processes holding the capture, then retry the run",
                        Some(&path),
                    ));
                }
            }
        }
        Ok(())
    }
}

enum Observation {
    Inline(String),
    Spilled(String),
}

fn read(path: &Path) -> Result<Observation> {
    let file = std::fs::File::open(path).map_err(|err| {
        failure(
            "open python output",
            err.to_string(),
            "keep the capture file available until the tool result is read",
            Some(path),
        )
    })?;
    let bytes = file
        .metadata()
        .map_err(|err| {
            failure(
                "read python output metadata",
                err.to_string(),
                "keep the capture file readable until the tool result is read",
                Some(path),
            )
        })?
        .len();
    if bytes > INLINE_BYTES {
        return Ok(Observation::Spilled(too_large(path)));
    }
    let mut captured = Vec::new();
    file.take(INLINE_BYTES.saturating_add(1))
        .read_to_end(&mut captured)
        .map_err(|err| {
            failure(
                "read python output",
                err.to_string(),
                "keep the capture file readable until the tool result is read",
                Some(path),
            )
        })?;
    if u64::try_from(captured.len()).unwrap_or(u64::MAX) > INLINE_BYTES {
        return Ok(Observation::Spilled(too_large(path)));
    }
    match String::from_utf8(captured) {
        Ok(text) => Ok(Observation::Inline(text)),
        Err(_) => Ok(Observation::Spilled(format!(
            "Output is not UTF-8. Process it with python.\n{}",
            path.display()
        ))),
    }
}

fn too_large(path: &Path) -> String {
    format!(
        "Output exceeds {INLINE_BYTES} bytes. Use python to filter and print the needed result.\n{}",
        path.display()
    )
}

fn absolute(path: PathBuf) -> Result<PathBuf> {
    std::path::absolute(&path).map_err(|err| {
        failure(
            "resolve the capture directory",
            err.to_string(),
            "provide an absolute run directory or keep the current directory readable",
            Some(&path),
        )
    })
}

fn unique_temporary_directory() -> Result<PathBuf> {
    use std::hash::{BuildHasher, Hasher as _};
    let mut random = std::collections::hash_map::RandomState::new().build_hasher();
    random.write_u32(std::process::id());
    let nonce = random.finish();
    Ok(std::env::temp_dir().join(format!("mei-capture-{}-{nonce:x}", std::process::id())))
}

fn failure(
    action: impl Into<String>,
    subject: impl Into<String>,
    recovery: impl Into<String>,
    path: Option<&Path>,
) -> Error {
    let diagnostic = Diagnostic::new(Code::Capture, action, subject, recovery);
    Error::Diagnostic(match path {
        Some(path) => diagnostic.with_path(path.display().to_string()),
        None => diagnostic,
    })
}
