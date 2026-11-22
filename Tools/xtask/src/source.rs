// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! First-party Rust and client sources shared by the file-level gates.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::report::XtaskError;
use crate::walk;

#[derive(Debug, Clone)]
pub(crate) struct Scope {
    files: BTreeSet<String>,
}

impl Scope {
    pub(crate) fn from_capture(paths: &BTreeSet<String>) -> Self {
        Self {
            files: paths.clone(),
        }
    }

    pub(crate) fn new(paths: &[String]) -> Result<Self, XtaskError> {
        let mut files = BTreeSet::new();
        for path in paths {
            let normalized = path.replace('\\', "/");
            let relative = normalized.strip_prefix("./").unwrap_or(&normalized);
            if relative.is_empty()
                || relative.starts_with('/')
                || relative.contains(':')
                || relative.split('/').any(|part| part == "..")
            {
                return Err(XtaskError::InvalidFileScope { path: path.clone() });
            }
            files.insert(relative.to_owned());
        }
        Ok(Self { files })
    }

    pub(crate) fn allows(&self, relative: &str) -> bool {
        self.files.contains(relative)
    }

    pub(crate) fn paths(&self) -> &BTreeSet<String> {
        &self.files
    }
}

pub(crate) fn files_in(root: &Path, scope: Option<&Scope>) -> Result<Vec<PathBuf>, XtaskError> {
    let members = crate::members::members(root)?;
    let mut files = walk::files_under(
        root,
        members.iter().map(|member| member.dir.as_str()),
        &["rs"],
    )?;
    let page = root.join("RefRain");
    if page.is_dir() {
        files.extend(walk::files_with_ext(&page, &["ts", "tsx", "svelte"])?);
    }
    files.sort_by_key(|path| walk::rel(root, path));
    files.dedup();
    if let Some(scope) = scope {
        files.retain(|path| scope.allows(&walk::rel(root, path)));
    }
    Ok(files)
}
