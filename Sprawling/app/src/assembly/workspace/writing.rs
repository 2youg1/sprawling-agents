// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use kernel::{AxCode, AxError, B3Hash, Retry};
use wire::{WorkspaceRecord, WorkspaceSave};

use super::disk::{Seen, conflict, refused};
use super::{Pending, Record, Registry, identity_token};

impl Registry {
    pub(super) fn save(
        &mut self,
        save: WorkspaceSave,
        bytes: &[u8],
    ) -> Result<WorkspaceRecord, AxError> {
        let WorkspaceSave {
            workspace,
            id,
            baseline,
        } = save;
        self.recover(workspace, id)?;
        let current = self.workspace(workspace)?;
        let root = current.open()?;
        let mut record = current.record(id)?.clone();
        let root_path = current.root.clone();
        let (path, observed) = record.locate(&root)?;
        let full_path = root_path.join(&path);
        if !editable(&path) || observed.permissions.readonly() {
            return Err(refused(
                &full_path,
                "the file format or the operating system marks this file read-only",
            ));
        }
        let version = B3Hash::digest(bytes);
        if record.last_save == Some(baseline)
            && observed.version == version
            && record.exposed.version == version
        {
            return Ok(record.exposed);
        }
        verify(&full_path, &observed, &record, baseline)?;
        record.exposed.path = path.clone();
        if observed.version == version {
            record.exposed.version = version;
            return self.keep(workspace, record);
        }
        let nonce = identity_token()?;
        let stage = root.stage(&path, nonce, bytes, observed.permissions)?;
        record.pending = Some(Pending {
            stage: nonce,
            identity: stage.id.clone(),
            version,
            baseline,
        });
        if let Err(error) = self.keep(workspace, record.clone()) {
            stage.discard()?;
            return Err(error);
        }
        let land = self
            .workspace(workspace)?
            .open()
            .and_then(|root| stage.replace(&root, &path, &record.identity, baseline));
        match land {
            Ok(identity) => {
                record.exposed.version = version;
                record.identity = identity;
                record.last_save = Some(baseline);
                record.pending = None;
                self.keep(workspace, record)
                    .map_err(|err| unknown(&full_path, &err))
            }
            Err(error) if error.retry() == Retry::Unknown => Err(error),
            Err(error) => {
                stage.discard()?;
                record.pending = None;
                self.keep(workspace, record)?;
                Err(error)
            }
        }
    }

    pub(super) fn recover(&mut self, workspace: B3Hash, id: B3Hash) -> Result<(), AxError> {
        let current = self.workspace(workspace)?;
        let mut record = current.record(id)?.clone();
        let Some(pending) = record.pending.take() else {
            return Ok(());
        };
        let root = current.open()?;
        let path = record.exposed.path.clone();
        let observed = match root.read(&path) {
            Ok(observed) => Some(observed),
            Err(err)
                if err.code() == &AxCode::PathNotFound || err.code() == &AxCode::GateDenied =>
            {
                None
            }
            Err(err) => return Err(err),
        };
        if let Some(found) = observed.filter(|found| {
            found.id.matches_stage(&pending.identity) && found.version == pending.version
        }) {
            record.identity = found.id;
            record.exposed.version = found.version;
            record.last_save = Some(pending.baseline);
        } else {
            root.discard_stage(&path, pending.stage, &pending.identity)?;
        }
        self.keep(workspace, record).map(|_| ())
    }
}

fn verify(path: &Path, actual: &Seen, record: &Record, baseline: B3Hash) -> Result<(), AxError> {
    if actual.id != record.identity || actual.version != baseline {
        return Err(conflict(
            path,
            &format!(
                "expected baseline {baseline}, observed {}; the save was not applied",
                actual.version
            ),
        ));
    }
    Ok(())
}

fn unknown(path: &Path, error: &AxError) -> AxError {
    AxError::failure(AxCode::StorageFatal, "record the workspace save", format!("{}: {}", path.display(), error.subject()))
        .effect_unknown()
        .with_recovery("reopen the file to recover its catalogue entry and check the version before repeating the save")
}

fn editable(path: &str) -> bool {
    const TEXT_EXTENSIONS: &[&str] = &[
        "asm",
        "bash",
        "bat",
        "c",
        "cc",
        "cfg",
        "clj",
        "cmake",
        "conf",
        "cpp",
        "cs",
        "css",
        "csv",
        "dart",
        "diff",
        "dockerfile",
        "env",
        "ex",
        "fish",
        "go",
        "graphql",
        "h",
        "hpp",
        "hs",
        "ini",
        "java",
        "jl",
        "js",
        "json",
        "jsonl",
        "jsx",
        "kt",
        "lean",
        "log",
        "lua",
        "m",
        "make",
        "markdown",
        "md",
        "mjs",
        "mmd",
        "mermaid",
        "ml",
        "php",
        "pl",
        "ps1",
        "py",
        "r",
        "rb",
        "rs",
        "rst",
        "scala",
        "sh",
        "sql",
        "svelte",
        "swift",
        "tex",
        "toml",
        "ts",
        "tsv",
        "tsx",
        "txt",
        "vb",
        "vue",
        "wgsl",
        "xml",
        "yaml",
        "yml",
        "zig",
        "zsh",
    ];
    let path = Path::new(path);
    path.extension()
        .and_then(|extension| extension.to_str())
        .map_or_else(
            || {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| {
                        [
                            "Dockerfile",
                            "Makefile",
                            "README",
                            "LICENSE",
                            "AUTHORS",
                            "CHANGELOG",
                        ]
                        .iter()
                        .any(|known| name.eq_ignore_ascii_case(known))
                    })
            },
            |extension| {
                TEXT_EXTENSIONS
                    .iter()
                    .any(|known| extension.eq_ignore_ascii_case(known))
            },
        )
}
