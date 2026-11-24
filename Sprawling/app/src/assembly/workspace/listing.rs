// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{AxCode, AxError, B3Hash};
use wire::{WorkspaceEntry, WorkspacePage};

use super::{Registry, identity_token};

pub(super) const DIRECTORY_PAGE: usize = 128;
const OPEN_LISTS: usize = 16;

pub(super) struct Listing {
    id: B3Hash,
    workspace: B3Hash,
    path: String,
    revision: Option<String>,
    entries: Vec<WorkspaceEntry>,
}

impl Registry {
    pub(super) fn list(
        &mut self,
        workspace: B3Hash,
        path: &str,
        cursor: Option<&str>,
    ) -> Result<WorkspacePage, AxError> {
        if let Some(cursor) = cursor {
            let (key, offset) = cursor
                .split_once(':')
                .and_then(|(key, offset)| {
                    Some((key.parse::<B3Hash>().ok()?, offset.parse::<usize>().ok()?))
                })
                .ok_or_else(stale_cursor)?;
            let listing = self
                .listings
                .iter()
                .find(|listing| {
                    listing.id == key && listing.workspace == workspace && listing.path == path
                })
                .ok_or_else(stale_cursor)?;
            return Ok(listing.page(offset));
        }
        let revision = self.directory_revision(workspace, path)?;
        let entries = self.workspace(workspace)?.open()?.entries(path)?;
        let listing = Listing {
            id: identity_token()?,
            workspace,
            path: path.to_owned(),
            revision,
            entries,
        };
        let result = listing.page(0);
        if result.next.is_some() {
            if self.listings.len() == OPEN_LISTS {
                self.listings.pop_front();
            }
            self.listings.push_back(listing);
        }
        Ok(result)
    }
}

impl Listing {
    fn page(&self, offset: usize) -> WorkspacePage {
        let end = offset
            .saturating_add(DIRECTORY_PAGE)
            .min(self.entries.len());
        let entries = match self.entries.get(offset..end) {
            Some(page) => page.to_vec(),
            None => Vec::new(),
        };
        WorkspacePage {
            entries,
            revision: self.revision.clone(),
            next: (end < self.entries.len()).then(|| format!("{}:{end}", self.id)),
        }
    }
}

fn stale_cursor() -> AxError {
    AxError::failure(
        AxCode::VersionConflict,
        "continue listing a workspace directory",
        "the directory snapshot is no longer available",
    )
    .with_recovery("reload the directory from its first page")
}
