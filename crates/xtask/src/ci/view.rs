// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! One owned input view: the same bytes feed its files and content identity
//! (xtask `spec/Ci.lean`, D43). Like library precheck, capture refuses links
//! and snapshots regular hard-linked sources as independent ordinary files.

use std::collections::BTreeMap;
use std::io::Write as _;
use std::path::Path;

use crate::report::XtaskError;
use crate::walk;

/// The source view whose lifetime covers planning and all selected actions.
/// Its directory is private before any source bytes are written to it.
#[derive(Debug)]
pub(crate) struct Snapshot {
    files: BTreeMap<String, blake3::Hash>,
    owned: tempfile::TempDir,
}

impl Snapshot {
    pub(crate) fn capture(root: &Path) -> Result<Self, XtaskError> {
        let owned = tempfile::Builder::new()
            .prefix("xtask-ci-")
            .tempdir()
            .map_err(|source| XtaskError::Io {
                path: "create private verification input (capture-directory)".to_owned(),
                source,
            })?;
        let result = make_private(owned.path()).and_then(|()| capture(root, owned.path()));
        match result {
            Ok(files) => Ok(Self { files, owned }),
            Err(error) => close(owned, Err(error)),
        }
    }

    pub(crate) fn root(&self) -> &Path {
        self.owned.path()
    }

    pub(crate) fn files(&self) -> &BTreeMap<String, blake3::Hash> {
        &self.files
    }

    /// End the lifetime even when the action failed; neither failure is erased.
    pub(crate) fn finish<T>(self, result: Result<T, XtaskError>) -> Result<T, XtaskError> {
        close(self.owned, result)
    }

    #[cfg(test)]
    pub(crate) fn close(self) -> Result<(), XtaskError> {
        self.finish(Ok(()))
    }
}

fn capture(root: &Path, owned: &Path) -> Result<BTreeMap<String, blake3::Hash>, XtaskError> {
    let mut files = BTreeMap::new();
    for source in walk::files_without_aliases(root)? {
        let relative = source.strip_prefix(root).map_err(|error| XtaskError::Doc {
            file: source.display().to_string(),
            msg: format!("capture source path (capture-outside-root): {error}; keep inputs inside the checkout"),
        })?;
        let name = relative
            .to_str()
            .ok_or_else(|| XtaskError::Doc {
                file: source.display().to_string(),
                msg: "capture source path (capture-non-utf8): rename the input to a UTF-8 path"
                    .to_owned(),
            })?
            .to_owned();
        #[cfg(windows)]
        let name = name.replace('\\', "/");
        let bytes = std::fs::read(&source).map_err(|source_error| XtaskError::Io {
            path: source.display().to_string(),
            source: source_error,
        })?;
        let target = owned.join(relative);
        let parent = target.parent().ok_or_else(|| XtaskError::Doc {
            file: target.display().to_string(),
            msg: "capture source path (capture-no-parent): keep inputs below the checkout root"
                .to_owned(),
        })?;
        std::fs::create_dir_all(parent).map_err(|source| XtaskError::Io {
            path: parent.display().to_string(),
            source,
        })?;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&target)
            .map_err(|source| XtaskError::Io {
                path: target.display().to_string(),
                source,
            })?;
        file.write_all(&bytes).map_err(|source| XtaskError::Io {
            path: target.display().to_string(),
            source,
        })?;
        if files.insert(name, blake3::hash(&bytes)).is_some() {
            return Err(XtaskError::Doc {
                file: source.display().to_string(),
                msg: "capture source path (capture-name-collision): give every input a distinct UTF-8 relative path".to_owned(),
            });
        }
    }
    Ok(files)
}

fn close<T>(owned: tempfile::TempDir, result: Result<T, XtaskError>) -> Result<T, XtaskError> {
    let path = owned.path().display().to_string();
    match (result, owned.close()) {
        (outcome, Ok(())) => outcome,
        (Ok(_), Err(source)) => Err(XtaskError::Io { path, source }),
        (Err(error), Err(source)) => Err(XtaskError::Doc {
            file: path,
            msg: format!(
                "verification failed: {error}; removing its private input also failed: {source} (capture-cleanup); remove the private directory after its processes exit"
            ),
        }),
    }
}

#[cfg(unix)]
fn make_private(dir: &Path) -> Result<(), XtaskError> {
    use std::os::unix::fs::PermissionsExt as _;
    std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700)).map_err(|source| {
        XtaskError::Io {
            path: dir.display().to_string(),
            source,
        }
    })
}

#[cfg(all(test, unix))]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use std::ffi::OsString;
    use std::os::unix::{ffi::OsStringExt as _, fs::PermissionsExt as _};

    use super::*;

    #[test]
    fn unix_names_remain_distinct_and_unrepresentable_inputs_are_refused() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("a")).unwrap();
        std::fs::write(root.path().join("a/b.rs"), "slash").unwrap();
        std::fs::write(root.path().join("a\\b.rs"), "backslash").unwrap();
        let view = Snapshot::capture(root.path()).unwrap();
        assert!(view.files().contains_key("a/b.rs"));
        assert!(view.files().contains_key("a\\b.rs"));
        assert_ne!(view.files().get("a/b.rs"), view.files().get("a\\b.rs"));
        assert_eq!(
            std::fs::metadata(view.root()).unwrap().permissions().mode() & 0o777,
            0o700
        );
        view.close().unwrap();
        let invalid = root.path().join(OsString::from_vec(b"bad\xff.rs".to_vec()));
        std::fs::write(&invalid, "invalid name").unwrap();
        assert!(Snapshot::capture(root.path()).is_err());
        std::fs::remove_file(invalid).unwrap();
        std::os::unix::fs::symlink("a/b.rs", root.path().join("alias.rs")).unwrap();
        assert!(Snapshot::capture(root.path()).is_err());
        root.close().unwrap();
    }
}

#[cfg(windows)]
#[expect(
    clippy::disallowed_methods,
    reason = "developer-tool private input directory, same OS door as Keying section 8-22"
)]
fn make_private(dir: &Path) -> Result<(), XtaskError> {
    let root = std::env::var_os("SystemRoot").ok_or_else(|| XtaskError::Doc {
        file: "SystemRoot (capture-permissions)".to_owned(),
        msg: "set SystemRoot to the Windows installation before creating a private input view"
            .to_owned(),
    })?;
    let name = std::env::var("USERNAME").map_err(|error| XtaskError::Doc {
        file: "USERNAME (capture-permissions)".to_owned(),
        msg: format!(
            "identify the current account: {error}; restore its USERNAME environment variable"
        ),
    })?;
    let user = match std::env::var("USERDOMAIN") {
        Ok(domain) => format!("{domain}\\{name}"),
        Err(std::env::VarError::NotPresent) => name,
        Err(error) => {
            return Err(XtaskError::Doc {
                file: "USERDOMAIN (capture-permissions)".to_owned(),
                msg: format!(
                    "identify the current account: {error}; restore its USERDOMAIN environment variable"
                ),
            });
        }
    };
    let status =
        std::process::Command::new(std::path::PathBuf::from(root).join("System32/icacls.exe"))
            .arg(dir)
            .args(["/inheritance:r", "/grant:r"])
            .arg(format!("{user}:(OI)(CI)F"))
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map_err(|source| XtaskError::Io {
                path: dir.display().to_string(),
                source,
            })?;
    if status.success() {
        Ok(())
    } else {
        Err(XtaskError::Doc {
            file: dir.display().to_string(),
            msg: format!(
                "make the input directory private (capture-permissions): icacls returned {status}; check this account can set permissions on its temporary directory"
            ),
        })
    }
}
