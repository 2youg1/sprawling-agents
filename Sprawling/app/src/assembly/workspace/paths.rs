// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::{Component, Path};

use kernel::AxError;

use super::disk::refused;

pub(super) const STAGE_PREFIX: &str = ".refrain-stage-";

pub(super) fn split(path: &str) -> Result<(&str, &str), AxError> {
    let (parent, leaf) = path.rsplit_once('/').unwrap_or(("", path));
    if !usable_name(leaf) || path.starts_with('/') {
        return Err(refused(
            Path::new(path),
            "a relative file name is protected or invalid",
        ));
    }
    Ok((parent, leaf))
}

pub(super) fn usable_name(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && !name.contains(['\\', ':', '\0'])
        && !name.chars().any(char::is_control)
        && (!cfg!(windows) || !name.ends_with([' ', '.']))
        && !kernel::PROTECTED_METADATA
            .iter()
            .any(|protected| name.eq_ignore_ascii_case(protected))
        && !name.eq_ignore_ascii_case(".refrain")
        && !name.starts_with(STAGE_PREFIX)
        && !kernel::layout::is_document_staging_name(name.as_ref())
}

pub(super) fn usable_root(path: &Path) -> bool {
    path.components().all(|component| match component {
        Component::Normal(name) => name.to_str().is_some_and(usable_name),
        #[cfg(windows)]
        Component::Prefix(prefix) => matches!(
            prefix.kind(),
            std::path::Prefix::Disk(_) | std::path::Prefix::VerbatimDisk(_)
        ),
        #[cfg(not(windows))]
        Component::Prefix(_) => false,
        Component::RootDir => true,
        Component::ParentDir | Component::CurDir => false,
    })
}

pub(super) fn joined(parent: &str, leaf: &str) -> String {
    if parent.is_empty() {
        leaf.to_owned()
    } else {
        format!("{parent}/{leaf}")
    }
}
