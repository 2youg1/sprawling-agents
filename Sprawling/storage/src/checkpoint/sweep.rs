// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::Payload;
use serde_json::{Map, Value};

use crate::error::StorageError;

use super::commit::{Checkpoint, git_err};

impl Checkpoint {

    pub fn wave_post(&mut self, pre_oid: &str) -> Result<Vec<Payload>, StorageError> {
        let oid = git2::Oid::from_str(pre_oid).map_err(git_err("parse checkpoint oid"))?;
        let commit = self
            .repo
            .find_commit(oid)
            .map_err(git_err("find checkpoint commit"))?;
        let tree = commit.tree().map_err(git_err("read checkpoint tree"))?;
        let workdir = self
            .repo
            .workdir()
            .ok_or_else(|| StorageError::Checkpoint {
                op: "sweep the working tree",
                detail: "the city repository is bare".to_owned(),
            })?
            .to_path_buf();
        let mut deleted: Vec<String> = Vec::new();
        let mut fault: Option<StorageError> = None;
        let walked = tree.walk(git2::TreeWalkMode::PreOrder, |dir, entry| {
            if entry.kind() != Some(git2::ObjectType::Blob) {
                return git2::TreeWalkResult::Ok;
            }

            let Ok(name) = entry.name() else {
                fault = Some(StorageError::Checkpoint {
                    op: "sweep the working tree",
                    detail: format!(
                        "{dir}{}: the checkpoint holds a name that is not UTF-8",
                        String::from_utf8_lossy(entry.name_bytes())
                    ),
                });
                return git2::TreeWalkResult::Abort;
            };
            let path = format!("{dir}{name}");
            match std::fs::symlink_metadata(workdir.join(&path)) {
                Ok(_) => {}
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => deleted.push(path),
                Err(err) => {
                    fault = Some(StorageError::Checkpoint {
                        op: "sweep the working tree",
                        detail: format!("{path}: {err}"),
                    });
                    return git2::TreeWalkResult::Abort;
                }
            }
            git2::TreeWalkResult::Ok
        });
        walked.map_err(git_err("walk the checkpoint tree"))?;
        if let Some(err) = fault {
            return Err(err);
        }

        deleted.sort();
        let mut payloads = Vec::new();
        for path in deleted {
            let mut map = Map::new();
            map.insert(
                "paths".to_owned(),
                Value::Array(vec![Value::String(format!("file:{path}"))]),
            );
            let mut restoration = Map::new();
            restoration.insert(
                "tracked".to_owned(),
                Value::String(format!("file:{path}@{pre_oid}")),
            );
            map.insert("restoration".to_owned(), Value::Object(restoration));
            payloads.push(Payload::new(map).map_err(|source| StorageError::Draft { source })?);
        }
        Ok(payloads)
    }
}
