// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::{Path, PathBuf};

use super::ProjectRepository;
use crate::StorageError;

pub(super) fn failed<'a>(
    op: &'static str,
    path: &'a Path,
) -> impl Fn(git2::Error) -> StorageError + 'a {
    move |err| StorageError::Checkpoint {
        op,
        detail: format!("{}: {err}", path.display()),
    }
}

pub struct SourceSnapshot {
    scratch: tempfile::TempDir,
    revision: git2::Oid,
    tracked: Vec<(PathBuf, u32)>,
}

impl SourceSnapshot {
    pub fn read(
        private: &Path,
        source: &Path,
        point: Option<&kernel::GitOid>,
    ) -> Result<Self, StorageError> {
        let refused = failed("read the registered source commit", source);
        let repo = git2::Repository::open_ext(
            source,
            git2::RepositoryOpenFlags::NO_SEARCH,
            std::iter::empty::<&Path>(),
        )
        .map_err(&refused)?;
        let revision = match point {
            Some(point) => {
                repo.find_commit(git2::Oid::from_str(&point.to_string()).map_err(&refused)?)
            }
            None => repo.head().and_then(|head| head.peel_to_commit()),
        }
        .map_err(&refused)?;
        let scratch = tempfile::Builder::new()
            .prefix(".import-")
            .tempdir_in(private.parent().ok_or_else(|| {
                refused(git2::Error::from_str(
                    "the private city must have a parent directory",
                ))
            })?)
            .map_err(|source| StorageError::Io {
                op: "create the private source checkout",
                path: private.to_owned(),
                source,
            })?;
        let checkout = git2::Repository::init(scratch.path()).map_err(&refused)?;
        let objects = repo.commondir().join("objects");
        checkout
            .odb()
            .and_then(|odb| {
                odb.add_disk_alternate(objects.to_str().ok_or_else(|| {
                    git2::Error::from_str("source object directory is not Unicode")
                })?)
            })
            .map_err(&refused)?;
        let tree = checkout.find_tree(revision.tree_id()).map_err(&refused)?;
        let mut index = checkout.index().map_err(&refused)?;
        index.read_tree(&tree).map_err(&refused)?;
        let mut tracked = Vec::new();
        let mut protected = Vec::new();
        for entry in index.iter() {
            let path = Path::new(std::str::from_utf8(&entry.path).map_err(|_| {
                refused(git2::Error::from_str("a tracked file name is not Unicode"))
            })?);
            if !crate::reserved::outside_reserved(path) {
                protected.push(path.to_owned());
            } else if matches!(entry.mode, 0o100_644 | 0o100_755) {
                tracked.push((path.to_owned(), entry.mode));
            } else {
                return Err(StorageError::Checkpoint {
                    op: "register a source revision",
                    detail: format!(
                        "{}: {} has unsupported Git file mode {:o}; register a revision containing regular files",
                        source.display(),
                        path.display(),
                        entry.mode
                    ),
                });
            }
        }
        for path in protected {
            index.remove_path(&path).map_err(&refused)?;
        }
        let mut options = git2::build::CheckoutBuilder::new();
        options.force().disable_filters(true);
        checkout
            .checkout_index(Some(&mut index), Some(&mut options))
            .map_err(&refused)?;
        Ok(Self {
            scratch,
            revision: revision.id(),
            tracked,
        })
    }

    pub(crate) fn stage(
        &self,
        repo: &git2::Repository,
        building: &kernel::Address,
    ) -> Result<Vec<String>, StorageError> {
        let refused = failed("stage the registered source revision", repo.path());
        let root = repo
            .workdir()
            .ok_or_else(|| refused(git2::Error::from_str("missing private working directory")))?;
        let mut index = repo.index().map_err(&refused)?;
        let mut added = Vec::new();
        for (path, mode) in &self.tracked {
            let relative = Path::new(building.as_str()).join(path);
            if index.get_path(&relative, 0).is_none() {
                crate::alias::WriteTarget::within(
                    "stage the registered file",
                    root,
                    &root.join(&relative),
                )?;
                index.add_path(&relative).map_err(&refused)?;
                added.push(format!("{}/{}", building.as_str(), path.display()).replace('\\', "/"));
            }
            if let Some(mut entry) = index.get_path(&relative, 0)
                && entry.mode != *mode
            {
                entry.mode = *mode;
                index.add(&entry).map_err(&refused)?;
            }
        }
        Ok(added)
    }

    pub fn path(&self) -> &Path {
        self.scratch.path()
    }

    pub fn checkpoint(
        &self,
        writer: crate::Checkpoint,
        t: kernel::TimeMs,
        of: &crate::Provenance,
        progress: &mut dyn FnMut(crate::BaseProgress),
    ) -> Result<kernel::event::record::Commit, StorageError> {
        writer.import_base(
            &crate::checkpoint::base::BaseOf {
                scopes: &[of.actor().as_str().to_owned()],
                t,
                of,
                source: Some(self),
            },
            progress,
        )
    }

    pub fn bind(
        &self,
        project: &ProjectRepository,
        baseline: kernel::GitOid,
    ) -> Result<(), StorageError> {
        let refused = failed("bind the registered source commit", project.root());
        let repo = git2::Repository::open(project.root()).map_err(&refused)?;
        let base = git2::Oid::from_str(&baseline.to_string()).map_err(&refused)?;
        repo.reference(
            &format!("refs/sprawling/source/{}", self.revision),
            base,
            false,
            "register project baseline",
        )
        .map_err(&refused)?;
        Ok(())
    }
}
