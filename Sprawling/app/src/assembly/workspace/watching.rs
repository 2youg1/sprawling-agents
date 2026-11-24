// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use cap_std::fs::Dir;
use kernel::{AxError, B3Hash};
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::watch;
use wire::WorkspaceWatch;

use super::disk::Identity;
use super::{Registry, identity_token};

const WATCH_LIMIT: usize = 16;

pub(super) struct WatchPoint {
    pub(super) directory: Dir,
    #[cfg(not(target_os = "linux"))]
    pub(super) shown: PathBuf,
    pub(super) identity: Identity,
}

#[derive(Default)]
pub(super) struct Watching(VecDeque<DirectoryWatch>);

struct DirectoryWatch {
    workspace: B3Hash,
    path: String,
    identity: Identity,
    signal: watch::Sender<Option<String>>,
    _watcher: RecommendedWatcher,
    #[cfg(target_os = "linux")]
    _anchor: Dir,
}

impl Registry {
    pub(super) fn directory_revision(
        &mut self,
        workspace: B3Hash,
        path: &str,
    ) -> Result<Option<String>, AxError> {
        let point = self.workspace(workspace)?.open()?.watch_point(path)?;
        self.watching.revision(workspace, path, point)
    }

    pub(super) fn observe(
        &mut self,
        workspace: B3Hash,
        path: &str,
        revision: String,
    ) -> Result<WorkspaceWatch, AxError> {
        let point = self.workspace(workspace)?.open()?.watch_point(path)?;
        let _current = self.watching.revision(workspace, path, point)?;
        let observed = self
            .watching
            .0
            .iter()
            .find(|item| item.workspace == workspace && item.path == path)
            .map(|item| item.signal.subscribe());
        Ok(WorkspaceWatch { revision, observed })
    }
}

impl Watching {
    fn revision(
        &mut self,
        workspace: B3Hash,
        path: &str,
        point: WatchPoint,
    ) -> Result<Option<String>, AxError> {
        if let Some(at) = self
            .0
            .iter()
            .position(|item| item.workspace == workspace && item.path == path)
            && let Some(existing) = self.0.remove(at)
            && existing.identity == point.identity
        {
            let revision = existing.signal.borrow().clone();
            if revision.is_some() {
                self.0.push_back(existing);
                return Ok(revision);
            }
        }
        if self.0.len() == WATCH_LIMIT {
            let Some(idle) = self
                .0
                .iter()
                .position(|item| item.signal.receiver_count() == 0)
            else {
                return Ok(None);
            };
            self.0.remove(idle);
        }
        let Some(made) = DirectoryWatch::start(workspace, path, point, identity_token()?) else {
            return Ok(None);
        };
        let revision = made.signal.borrow().clone();
        self.0.push_back(made);
        Ok(revision)
    }
}

impl DirectoryWatch {
    fn start(workspace: B3Hash, path: &str, point: WatchPoint, id: B3Hash) -> Option<Self> {
        let serial = AtomicU64::new(0);
        let (signal, _) = watch::channel(Some(format!("{id}:0")));
        let publish = signal.clone();
        let mut watcher =
            match notify::recommended_watcher(move |event: notify::Result<Event>| match event {
                Ok(event)
                    if event.need_rescan()
                        || (!event.kind.is_access()
                            && event.paths.iter().any(|path| {
                                path.file_name()
                                    .and_then(|name| name.to_str())
                                    .is_none_or(super::paths::usable_name)
                            })) =>
                {
                    let count = serial.fetch_add(1, Ordering::Relaxed).wrapping_add(1);
                    publish.send_replace(Some(format!("{id}:{count}")));
                }
                Ok(_) => {}
                Err(_) => {
                    publish.send_replace(None);
                }
            }) {
                Ok(watcher) => watcher,
                Err(_) => return None,
            };
        if let Err(_unavailable) = watcher.watch(&point.native_path(), RecursiveMode::NonRecursive)
        {
            return None;
        }
        #[cfg(not(target_os = "linux"))]
        drop(point.directory);
        Some(Self {
            workspace,
            path: path.to_owned(),
            identity: point.identity,
            signal,
            _watcher: watcher,
            #[cfg(target_os = "linux")]
            _anchor: point.directory,
        })
    }
}

impl WatchPoint {
    #[cfg(target_os = "linux")]
    fn native_path(&self) -> PathBuf {
        use std::os::fd::AsRawFd as _;
        PathBuf::from(format!("/proc/self/fd/{}", self.directory.as_raw_fd()))
    }

    #[cfg(not(target_os = "linux"))]
    fn native_path(&self) -> PathBuf {
        self.shown.clone()
    }
}
