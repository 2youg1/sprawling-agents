// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Worktree names: one segment, no escape.

use crate::error::MemoryError;

/// A node's name for its tree. A newtype because this string becomes a
/// directory name and a git reference: the two ways it can be malformed
/// are checked once, here.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct WorktreeName(String);

impl WorktreeName {
    /// # Errors
    /// Refuses an empty name, a path separator, a leading dot, and any
    /// character outside `[A-Za-z0-9._-]`. A name that walks out of its
    /// directory is the whole isolation guarantee walking out with it.
    pub fn parse(raw: &str) -> Result<WorktreeName, MemoryError> {
        let refuse = |detail: &str| MemoryError::Worktree {
            op: "name a worktree",
            detail: format!("{raw}: {detail}"),
        };
        if raw.is_empty() {
            return Err(refuse("a tree with no name cannot be released either"));
        }
        if raw.starts_with('.') {
            return Err(refuse("names do not start with a dot"));
        }
        if !raw
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
        {
            return Err(refuse("letters, digits, dot, underscore and dash only"));
        }
        Ok(WorktreeName(raw.to_owned()))
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::*;
    #[test]
    fn a_name_that_could_walk_out_of_its_directory_is_not_a_name() {
        for raw in ["", ".hidden", "../escape", "node/1", "node 1", "nöde"] {
            assert!(
                WorktreeName::parse(raw).is_err(),
                "{raw:?} is not a worktree name"
            );
        }
        assert_eq!(
            WorktreeName::parse("node-1.2_3").unwrap().as_str(),
            "node-1.2_3"
        );
    }
}
