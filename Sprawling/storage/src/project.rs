// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::{Path, PathBuf};

use kernel::Address;

use crate::{Checkpoint, StorageError, Worktrees};

mod publishing;
mod source;

pub use source::SourceSnapshot;

pub struct ProjectRepository {
    city: PathBuf,
    building: Address,
    root: PathBuf,
}

impl ProjectRepository {
    pub fn at(city: &Path, address: &Address) -> Result<Self, StorageError> {
        let name = address
            .as_str()
            .split('/')
            .next()
            .unwrap_or(address.as_str());
        let building = Address::parse(name).map_err(|source| StorageError::Draft { source })?;
        if building.is_reserved() {
            return Err(StorageError::Checkpoint {
                op: "select a project repository",
                detail: format!("{address} belongs to the reserved city directory"),
            });
        }
        let root = Self::directory(city).join(building.as_str());
        let git = root.join(".git");
        crate::alias::WriteTarget::within("select a project repository", city, &git)?;
        let refused = |source| StorageError::Io {
            op: "inspect the private repository directory",
            path: git.clone(),
            source,
        };
        match std::fs::symlink_metadata(&git) {
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => return Err(refused(err)),
            Ok(metadata)
                if metadata.is_dir() && !git.join("commondir").try_exists().map_err(refused)? => {}
            Ok(_) => {
                return Err(StorageError::Checkpoint {
                    op: "select a project repository",
                    detail: format!(
                        "{} must be an owned Git directory; choose a fresh project address",
                        git.display()
                    ),
                });
            }
        }
        Ok(Self {
            city: city.to_owned(),
            building,
            root,
        })
    }

    pub fn directory(city: &Path) -> PathBuf {
        city.join(kernel::RESERVED_PREFIX).join("projects")
    }

    pub fn building(&self) -> &Address {
        &self.building
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn list(city: &Path) -> Result<Vec<Self>, StorageError> {
        let dir = Self::directory(city);
        let failed = |source| StorageError::Io {
            op: "list private projects",
            path: dir.clone(),
            source,
        };
        let entries = match std::fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(err) => return Err(failed(err)),
        };
        let mut projects = Vec::new();
        for entry in entries {
            let entry = entry.map_err(failed)?;
            let name = entry.file_name();
            let address =
                Address::parse(name.to_str().ok_or_else(|| StorageError::Checkpoint {
                    op: "list private projects",
                    detail: format!("{} has no Unicode project name", entry.path().display()),
                })?)
                .map_err(|source| StorageError::Draft { source })?;
            projects.push(Self::at(city, &address)?);
        }
        projects.sort_by(|left, right| left.building.cmp(&right.building));
        Ok(projects)
    }

    pub fn containing(city: &Path, point: kernel::GitOid) -> Result<Self, StorageError> {
        let id = git2::Oid::from_str(&point.to_string())
            .map_err(source::failed("read the checkpoint identifier", city))?;
        for project in Self::list(city)? {
            let repo = git2::Repository::open(project.root()).map_err(source::failed(
                "open private project history",
                project.root(),
            ))?;
            if repo
                .odb()
                .map_err(source::failed(
                    "read private project history",
                    project.root(),
                ))?
                .exists(id)
            {
                return Ok(project);
            }
        }
        Err(StorageError::Checkpoint {
            op: "locate the checkpoint repository",
            detail: format!(
                "{point} is absent from the registered projects of {}",
                city.display()
            ),
        })
    }

    fn read_repository(&self) -> Result<git2::Repository, StorageError> {
        let repo = git2::Repository::open(self.root())
            .map_err(source::failed("read private project history", self.root()))?;
        self.attach(&repo)?;
        Ok(repo)
    }

    pub fn changes(
        &self,
        base: kernel::GitOid,
        head: crate::Head,
    ) -> Result<Vec<kernel::FileChange>, StorageError> {
        crate::changes::between_in(
            &self.read_repository()?,
            base,
            head,
            Some(self.building.as_str()),
        )
    }

    pub fn status(
        &self,
        point: Option<kernel::GitOid>,
    ) -> Result<crate::WorkingStatus, StorageError> {
        crate::status::working_status_in(
            &self.read_repository()?,
            Some(self.building.as_str()),
            point,
        )
    }

    pub fn checkpoint(&self) -> Result<Checkpoint, StorageError> {
        let checkpoint = Checkpoint::open(&self.root)?;
        self.attach(&checkpoint.repo)?;
        Ok(checkpoint)
    }

    pub fn claim(
        &self,
        run: kernel::RunId,
        scopes: &[String],
    ) -> Result<crate::WorktreeLease, StorageError> {
        let trees = self.trees()?;
        let name = crate::WorktreeName::parse(&format!("run-{run}"))?;
        let lease = trees.claim(&name, scopes)?;
        let result = (|| {
            let refused = source::failed("pin the run baseline", self.root());
            let base = trees
                .repo
                .refname_to_id(&format!("refs/heads/{}", name.as_str()))
                .map_err(&refused)?;
            trees
                .repo
                .reference(
                    &format!("refs/sprawling/working/{run}"),
                    base,
                    false,
                    "pin run baseline",
                )
                .map_err(&refused)?;
            Ok(())
        })();
        if let Err(failure) = result {
            trees.release(lease)?;
            return Err(failure);
        }
        Ok(lease)
    }

    pub fn run_checkpoint(
        city: &Path,
        root: &Path,
        run: kernel::RunId,
        actor: &Address,
    ) -> Result<Checkpoint, StorageError> {
        let project = Self::at(city, actor)?;
        let repo = git2::Repository::open(project.root())
            .map_err(source::failed("open the run repository", project.root()))?;
        let name = format!("run-{run}");
        let registered = project
            .root()
            .join(kernel::RESERVED_PREFIX)
            .join("worktrees")
            .join(&name);
        crate::alias::WriteTarget::within("open the run checkout", city, &registered)?;
        let physical = |path: &Path| {
            std::fs::canonicalize(path).map_err(|source| StorageError::Io {
                op: "resolve the run checkout",
                path: path.to_owned(),
                source,
            })
        };
        if root != registered && physical(root)? != physical(&registered)? {
            return Err(StorageError::Checkpoint {
                op: "open the run checkout",
                detail: format!("{} does not match {}", root.display(), registered.display()),
            });
        }
        repo.set_workdir(&registered, false)
            .map_err(source::failed("attach the run checkout", &registered))?;
        let dir = repo.path().join("indexes");
        Checkpoint::writer(
            repo,
            dir,
            run,
            Some(format!("refs/sprawling/working/{run}")),
        )
    }

    pub fn trees(&self) -> Result<Worktrees, StorageError> {
        let trees = Worktrees::open(&self.root)?;
        self.attach(&trees.repo)?;
        Ok(trees)
    }

    fn attach(&self, repo: &git2::Repository) -> Result<(), StorageError> {
        repo.set_workdir(&self.city, false)
            .map_err(|err| StorageError::Checkpoint {
                op: "attach the private project's working directory",
                detail: format!("{}: {err}", self.city.display()),
            })
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "test code"
)]
mod tests;
