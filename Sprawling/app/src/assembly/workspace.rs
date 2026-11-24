// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::{BTreeMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use kernel::{AxCode, AxError, B3Hash, TimeMs};
use serde::{Deserialize, Serialize};
use wire::{
    WorkspaceChange, WorkspaceInfo, WorkspaceOperation, WorkspaceRecord, WorkspaceReply,
    WorkspaceRequest, WorkspaceSink, WorkspaceTarget,
};

use disk::{Identity, Root, conflict, failure, refused};
use paths::usable_root;

mod catalogue;
mod disk;
mod listing;
mod paths;
mod staging;
mod watching;
mod writing;

use listing::Listing;

const RECENT_FILES: usize = 32;
const CATEGORY_MAX: usize = 128;

pub(super) fn workspace(city: PathBuf) -> WorkspaceSink {
    let registry = Mutex::new(Registry::new(city));
    Arc::new(move |operation| {
        let now = accounting::Clock::now(&super::SystemClock)?;
        registry
            .lock()
            .map_err(|err| {
                AxError::failure(
                    AxCode::StorageFatal,
                    "open the workspace registry",
                    err.to_string(),
                )
                .effect_unknown()
                .with_recovery(
                    "restart the listener and check the file version before retrying a write",
                )
            })?
            .act(operation, now)
    })
}

#[derive(Clone, Serialize, Deserialize)]
struct Catalogue {
    schema: u8,
    by_id: BTreeMap<B3Hash, Workspace>,
}

#[derive(Clone, Serialize, Deserialize)]
struct Workspace {
    root: PathBuf,
    identity: Identity,
    files: BTreeMap<B3Hash, Record>,
}

#[derive(Clone, Serialize, Deserialize)]
struct Record {
    exposed: WorkspaceRecord,
    identity: Identity,
    last_save: Option<B3Hash>,
    pending: Option<Pending>,
}

#[derive(Clone, Serialize, Deserialize)]
struct Pending {
    stage: B3Hash,
    identity: Identity,
    version: B3Hash,
    baseline: B3Hash,
}

struct Registry {
    city: PathBuf,
    table: Option<Catalogue>,
    listings: VecDeque<Listing>,
    watching: watching::Watching,
}

impl Registry {
    fn new(city: PathBuf) -> Self {
        Self {
            city,
            table: None,
            listings: VecDeque::new(),
            watching: watching::Watching::default(),
        }
    }

    fn act(
        &mut self,
        operation: WorkspaceOperation,
        now: TimeMs,
    ) -> Result<WorkspaceReply, AxError> {
        if self.table.is_none() {
            self.table = Some(self.read_catalogue()?);
        }
        match operation {
            WorkspaceOperation::Save(save, bytes) => {
                self.save(save, &bytes).map(WorkspaceReply::Record)
            }
            WorkspaceOperation::Request(request) => match request {
                WorkspaceRequest::Workspaces => self.catalogue().map(|table| {
                    WorkspaceReply::Workspaces(
                        table
                            .by_id
                            .iter()
                            .map(|(&id, workspace)| workspace.info(id))
                            .collect(),
                    )
                }),
                WorkspaceRequest::Register { root } => self
                    .register(Path::new(&root))
                    .map(WorkspaceReply::Registered),
                WorkspaceRequest::List {
                    workspace,
                    path,
                    cursor,
                } => self
                    .list(workspace, &path, cursor.as_deref())
                    .map(WorkspaceReply::Page),
                WorkspaceRequest::Watch {
                    workspace,
                    path,
                    revision,
                } => self
                    .observe(workspace, &path, revision)
                    .map(WorkspaceReply::Watch),
                WorkspaceRequest::Open { workspace, target } => self
                    .open(workspace, target, now)
                    .map(WorkspaceReply::Record),
                WorkspaceRequest::Bytes {
                    workspace,
                    id,
                    version,
                } => self
                    .bytes(workspace, id, version)
                    .map(WorkspaceReply::Bytes),
                WorkspaceRequest::Resource { workspace, path } => self
                    .workspace(workspace)?
                    .open()?
                    .read(&path)
                    .map(|observed| WorkspaceReply::Bytes(observed.bytes)),
                WorkspaceRequest::Remember {
                    workspace,
                    id,
                    changes,
                } => self
                    .remember(workspace, id, changes)
                    .map(WorkspaceReply::Record),
                WorkspaceRequest::Relink {
                    workspace,
                    id,
                    path,
                } => self
                    .relink(workspace, id, path, now)
                    .map(WorkspaceReply::Record),
            },
        }
    }

    fn register(&mut self, requested: &Path) -> Result<WorkspaceInfo, AxError> {
        if !requested.is_absolute() || !usable_root(requested) {
            return Err(refused(
                requested,
                "the registration requires an absolute local directory path",
            ));
        }
        let root = requested
            .canonicalize()
            .map_err(|err| failure("resolve the selected workspace directory", requested, &err))?;
        let city = self
            .city
            .canonicalize()
            .map_err(|err| failure("resolve the city directory", &self.city, &err))?;
        if !usable_root(&root) || root.starts_with(&city) || city.starts_with(&root) {
            return Err(refused(
                &root,
                "the directory overlaps the city or protected metadata",
            ));
        }
        let (_, identity) = Root::open(&root)?;
        let mut next = self.catalogue()?.clone();
        if let Some((&id, workspace)) = next
            .by_id
            .iter()
            .find(|(_, workspace)| workspace.root == root && workspace.identity == identity)
        {
            return Ok(workspace.info(id));
        }
        let id = identity_token()?;
        let workspace = Workspace {
            root,
            identity,
            files: BTreeMap::new(),
        };
        let info = workspace.info(id);
        next.by_id.insert(id, workspace);
        self.commit(next)?;
        Ok(info)
    }

    fn open(
        &mut self,
        workspace: B3Hash,
        target: WorkspaceTarget,
        now: TimeMs,
    ) -> Result<WorkspaceRecord, AxError> {
        if let WorkspaceTarget::Record { id } = &target {
            self.recover(workspace, *id)?;
        } else if let WorkspaceTarget::Path { path } = &target {
            let pending: Vec<_> = self
                .workspace(workspace)?
                .files
                .iter()
                .filter_map(|(&id, file)| {
                    (file.exposed.path == *path && file.pending.is_some()).then_some(id)
                })
                .collect();
            for id in pending {
                self.recover(workspace, id)?;
            }
        }
        let current = self.workspace(workspace)?;
        let root = current.open()?;
        let (path, observed, record) = match target {
            WorkspaceTarget::Path { path } => {
                let observed = root.read(&path)?;
                let record = current.files.values().find(|record| {
                    record.matches(&observed)
                        && (record.exposed.path == path || observed.id.has_birth())
                });
                (path, observed, record)
            }
            WorkspaceTarget::Record { id } => {
                let record = current.record(id)?;
                let (path, observed) = record.locate(&root)?;
                (path, observed, Some(record))
            }
        };
        let mut record = match record {
            Some(record) => record.clone(),
            None => Record {
                exposed: WorkspaceRecord {
                    id: identity_token()?,
                    path: path.clone(),
                    version: observed.version,
                    progress: None,
                    category: None,
                    last_opened: now.value(),
                },
                identity: observed.id,
                pending: None,
                last_save: None,
            },
        };
        record.exposed.path = path;
        record.exposed.version = observed.version;
        record.exposed.last_opened = now.value();
        self.keep(workspace, record)
    }

    fn bytes(
        &mut self,
        workspace: B3Hash,
        id: B3Hash,
        version: B3Hash,
    ) -> Result<Vec<u8>, AxError> {
        self.recover(workspace, id)?;
        let current = self.workspace(workspace)?;
        let root = current.open()?;
        let (path, observed) = current.record(id)?.locate(&root)?;
        if observed.version != version {
            return Err(conflict(
                &current.root.join(path),
                &format!("requested version {version}, observed {}", observed.version),
            ));
        }
        Ok(observed.bytes)
    }

    fn remember(
        &mut self,
        workspace: B3Hash,
        id: B3Hash,
        changes: Vec<WorkspaceChange>,
    ) -> Result<WorkspaceRecord, AxError> {
        let mut record = self.workspace(workspace)?.record(id)?.clone();
        for change in changes {
            match change {
                WorkspaceChange::Progress { progress } => {
                    if progress.page == 0 || !(0.0..=1.0).contains(&progress.offset) {
                        return Err(AxError::failure(AxCode::InvalidArgs, "remember reading progress", format!("{}: page {}, page offset {}", record.exposed.path, progress.page, progress.offset))
                            .with_recovery("supply a page number starting at one and a page offset from zero to one"));
                    }
                    record.exposed.progress = Some(progress);
                }
                WorkspaceChange::Category { category } => {
                    if category.as_ref().is_some_and(|category| {
                        category.chars().count() > CATEGORY_MAX
                            || category.chars().any(char::is_control)
                    }) {
                        return Err(AxError::failure(
                            AxCode::InvalidArgs,
                            "classify the workspace file",
                            &record.exposed.path,
                        )
                        .with_recovery(
                            "use a category of at most 128 characters without control characters",
                        ));
                    }
                    record.exposed.category = category;
                }
            }
        }
        self.keep(workspace, record)
    }

    fn relink(
        &mut self,
        workspace: B3Hash,
        id: B3Hash,
        path: String,
        now: TimeMs,
    ) -> Result<WorkspaceRecord, AxError> {
        self.recover(workspace, id)?;
        let current = self.workspace(workspace)?;
        let root = current.open()?;
        let mut record = current.record(id)?.clone();
        let observed = root.read(&path)?;
        if record.identity != observed.id && record.locate(&root).is_ok() {
            return Err(conflict(
                &current.root.join(path),
                "the old record still resolves to a separate file",
            ));
        }
        if current
            .files
            .values()
            .any(|matched| matched.exposed.id != id && matched.identity == observed.id)
        {
            return Err(conflict(
                &current.root.join(path),
                "the chosen file already has its own record",
            ));
        }
        record.identity = observed.id;
        record.exposed.path = path;
        record.exposed.version = observed.version;
        record.exposed.last_opened = now.value();
        record.last_save = None;
        self.keep(workspace, record)
    }

    fn keep(&mut self, workspace: B3Hash, record: Record) -> Result<WorkspaceRecord, AxError> {
        let result = record.exposed.clone();
        let mut next = self.catalogue()?.clone();
        next.by_id
            .get_mut(&workspace)
            .ok_or_else(|| missing("find a registered workspace", workspace))?
            .files
            .insert(result.id, record);
        self.commit(next)?;
        Ok(result)
    }

    fn workspace(&self, id: B3Hash) -> Result<&Workspace, AxError> {
        self.catalogue()?
            .by_id
            .get(&id)
            .ok_or_else(|| missing("find a registered workspace", id))
    }
}

impl Workspace {
    fn open(&self) -> Result<Root<'_>, AxError> {
        let (root, observed) = Root::open(&self.root)?;
        if observed != self.identity {
            return Err(conflict(
                &self.root,
                "the mounted directory has a different identity than the registered root",
            ));
        }
        Ok(root)
    }

    fn info(&self, id: B3Hash) -> WorkspaceInfo {
        let root = self.root.display().to_string();
        let name = self
            .root
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(&root)
            .to_owned();
        let mut recent: Vec<_> = self.files.values().map(|record| &record.exposed).collect();
        recent.sort_unstable_by_key(|record| (std::cmp::Reverse(record.last_opened), record.id));
        WorkspaceInfo {
            id,
            root,
            name,
            recent: recent.into_iter().take(RECENT_FILES).cloned().collect(),
        }
    }

    fn record(&self, id: B3Hash) -> Result<&Record, AxError> {
        self.files
            .get(&id)
            .ok_or_else(|| missing("find a workspace file record", id))
    }
}

impl Record {
    fn matches(&self, file: &disk::Seen) -> bool {
        self.identity == file.id && (file.id.has_birth() || file.version == self.exposed.version)
    }

    fn locate(&self, root: &Root<'_>) -> Result<(String, disk::Seen), AxError> {
        let direct = root.read(&self.exposed.path);
        match direct {
            Ok(found) if self.matches(&found) => {
                return Ok((self.exposed.path.clone(), found));
            }
            Ok(_) => {}
            Err(err)
                if err.code() == &AxCode::PathNotFound || err.code() == &AxCode::GateDenied => {}
            Err(err) => return Err(err),
        }
        let path = root.find(&self.identity, &self.exposed.path)?;
        let found = root.read(&path)?;
        if found.id != self.identity {
            return Err(conflict(
                Path::new(&path),
                "the recovered path changed while it was being verified",
            ));
        }
        Ok((path, found))
    }
}

fn missing(action: &str, id: B3Hash) -> AxError {
    AxError::failure(AxCode::PathNotFound, action, id.to_string()).with_recovery(
        "open the registered workspace and select a file or restore the workspace catalogue",
    )
}

fn identity_token() -> Result<B3Hash, AxError> {
    let mut bytes = [0; 32];
    getrandom::fill(&mut bytes).map_err(|err| {
        AxError::failure(
            AxCode::StorageFatal,
            "make a workspace record identifier",
            err.to_string(),
        )
        .with_recovery("check that the operating system provides entropy, then try again")
    })?;
    Ok(B3Hash::from_bytes(bytes))
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::wildcard_enum_match_arm,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]
mod tests;
