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
    handed_off: bool,
}

impl Store {
    pub fn create() -> Result<Store> {
        let (root, adopt) = match std::env::var_os("MEI_CAPTURE_DIR") {
            Some(path) if !path.is_empty() => (
                absolute(PathBuf::from(path))?,
                std::env::var_os("MEI_CAPTURE_CREATED").is_some_and(|value| value == "1"),
            ),
            Some(_) => {
                let recovery = if cfg!(windows) {
                    "remove the empty override with `Remove-Item Env:MEI_CAPTURE_DIR`, then start Mei again"
                } else {
                    "remove the empty override with `unset MEI_CAPTURE_DIR`, then start Mei again"
                };
                return Err(Error::Diagnostic(
                    Diagnostic::new(
                        Code::Capture,
                        "create the capture directory",
                        "MEI_CAPTURE_DIR must name a new run directory or be unset",
                        recovery,
                    )
                    .with_value("\"\""),
                ));
            }
            None => (unique_temporary_directory()?, false),
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
        match std::fs::create_dir(&root) {
            Ok(()) => {}
            Err(err) if adopt && err.kind() == std::io::ErrorKind::AlreadyExists => {
                adopt_created_directory(&root)?;
            }
            Err(err) => {
                return Err(failure(
                    "create a new capture directory",
                    err.to_string(),
                    "provide a directory that does not already exist; Mei refuses to reuse or delete an existing tree",
                    Some(&root),
                ));
            }
        }
        Ok(Store {
            root,
            inline: Vec::new(),
            handed_off: false,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn observe(&mut self, identifier: String) -> Result<String> {
        self.reclaim_inline()?;
        let path = self.capture_path(&identifier)?;
        match read(&path)? {
            Observation::Inline(text) => {
                self.inline.push(path);
                Ok(text)
            }
            Observation::Spilled(notice) => Ok(notice),
        }
    }

    pub fn complete(&mut self) -> Result<()> {
        self.handed_off = true;
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
        if self.handed_off {
            return Ok(());
        }
        match std::fs::remove_dir_all(&self.root) {
            Ok(()) => {
                self.handed_off = true;
                self.inline.clear();
                Ok(())
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                self.handed_off = true;
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

    pub fn abandon(&mut self) -> Result<()> {
        if self.handed_off {
            return Ok(());
        }
        self.discard()
    }

    fn capture_path(&self, identifier: &str) -> Result<PathBuf> {
        let valid = !identifier.is_empty()
            && identifier
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
            && identifier != "."
            && identifier != "..";
        if !valid {
            return Err(failure(
                "resolve a Python capture identifier",
                identifier,
                "keep the private channel limited to one ASCII capture filename",
                Some(&self.root),
            ));
        }
        Ok(self.root.join(identifier))
    }

    fn reclaim_inline(&mut self) -> Result<()> {
        while let Some(path) = self.inline.last() {
            match std::fs::remove_file(path) {
                Ok(()) => {}
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                Err(err) => {
                    return Err(failure(
                        "reclaim an inline capture",
                        err.to_string(),
                        "stop processes holding the capture, then retry the cleanup",
                        Some(path),
                    ));
                }
            }
            self.inline.pop();
        }
        Ok(())
    }
}

fn adopt_created_directory(root: &Path) -> Result<()> {
    let metadata = std::fs::symlink_metadata(root).map_err(|err| {
        failure(
            "inspect the caller-created capture directory",
            err.to_string(),
            "keep the run directory a real directory owned by this caller",
            Some(root),
        )
    })?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(failure(
            "adopt the caller-created capture directory",
            "the directory is not an empty real directory",
            "provide a newly created empty run directory and preserve MEI_CAPTURE_CREATED=1",
            Some(root),
        ));
    }
    let mut entries = std::fs::read_dir(root).map_err(|err| {
        failure(
            "inspect the caller-created capture directory",
            err.to_string(),
            "keep the run directory empty and readable",
            Some(root),
        )
    })?;
    let empty = match entries.next() {
        None => true,
        Some(Ok(_)) => false,
        Some(Err(err)) => {
            return Err(failure(
                "read the caller-created capture directory",
                err.to_string(),
                "keep the run directory empty and readable",
                Some(root),
            ));
        }
    };
    if empty {
        return Ok(());
    }
    Err(failure(
        "adopt the caller-created capture directory",
        "the directory is not empty",
        "provide a newly created empty run directory and preserve MEI_CAPTURE_CREATED=1",
        Some(root),
    ))
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
            path_instruction(path)
        ))),
    }
}

fn too_large(path: &Path) -> String {
    format!(
        "Output exceeds {INLINE_BYTES} bytes. Use python to filter and print the needed result.\n{}",
        path_instruction(path)
    )
}

fn path_instruction(path: &Path) -> String {
    if let Some(shown) = path.to_str() {
        return shown.to_owned();
    }
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt as _;
        format!(
            "__import__('os').fsdecode(bytes.fromhex('{}'))",
            hex(path.as_os_str().as_bytes())
        )
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt as _;
        let bytes = path
            .as_os_str()
            .encode_wide()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>();
        format!(
            "bytes.fromhex('{}').decode('utf-16-le', errors='surrogatepass')",
            hex(&bytes)
        )
    }
}

fn hex(bytes: &[u8]) -> String {
    let mut encoded = String::with_capacity(bytes.len().saturating_mul(2));
    for byte in bytes {
        encoded.push_str(&format!("{byte:02x}"));
    }
    encoded
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

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, reason = "test code")]

    use super::*;

    #[test]
    fn inline_cleanup_retries_the_same_path() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("blocked");
        std::fs::create_dir(&path).unwrap();
        let mut store = Store {
            root: directory.path().to_path_buf(),
            inline: vec![path.clone()],
            handed_off: false,
        };
        assert!(store.reclaim_inline().is_err());
        assert_eq!(store.inline, vec![path.clone()]);
        std::fs::remove_dir(&path).unwrap();
        std::fs::write(&path, []).unwrap();
        store.reclaim_inline().unwrap();
        assert!(store.inline.is_empty());
    }

    #[test]
    fn completion_hands_off_even_when_inline_cleanup_fails() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join("run");
        std::fs::create_dir(&root).unwrap();
        let blocked = root.join("blocked");
        std::fs::create_dir(&blocked).unwrap();
        let spilled = root.join("spilled");
        std::fs::write(&spilled, "keep").unwrap();
        let mut store = Store {
            root: root.clone(),
            inline: vec![blocked],
            handed_off: false,
        };
        assert!(store.complete().is_err());
        store.discard().unwrap();
        assert_eq!(std::fs::read_to_string(spilled).unwrap(), "keep");
    }

    #[cfg(unix)]
    #[test]
    fn a_non_unicode_path_has_a_lossless_python_representation() {
        use std::os::unix::ffi::OsStringExt as _;
        let path = PathBuf::from(std::ffi::OsString::from_vec(vec![b'p', 0xff]));
        let shown = path_instruction(&path);
        assert!(shown.starts_with("__import__('os').fsdecode"));
        assert!(shown.contains("70ff"));
        assert!(!shown.contains('�'));
    }
}
