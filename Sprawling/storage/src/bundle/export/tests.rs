// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::super::files::open_restored;
use super::super::fixture::city_with;
use super::super::manifest::RESERVED;
use super::*;
use kernel::{GENESIS_PREV, TimeMs};

#[test]
fn a_city_comes_back_in_an_empty_directory_and_its_chain_verifies() {
    let home = tempfile::tempdir().unwrap();
    city_with(3, home.path());
    let carried = tempfile::tempdir().unwrap();
    let exported = Bundle::export(home.path(), carried.path()).unwrap();
    assert_eq!(exported.records(), 3);
    assert_ne!(exported.head(), GENESIS_PREV.to_string());

    let elsewhere = tempfile::tempdir().unwrap();
    let restored = Bundle::restore(carried.path(), elsewhere.path()).unwrap();
    assert_eq!(restored, exported);

    assert!(elsewhere.path().join("City.md").exists());
    assert!(elsewhere.path().join("lab").join("Roadmap.md").exists());

    open_restored(elsewhere.path(), TimeMs::new(9)).unwrap();
}

struct LosesRoadmap(RealFs);

impl LosesRoadmap {
    fn swallowed(path: &std::path::Path) -> bool {
        path.ends_with("Roadmap.md")
    }
}

impl crate::vfs::Vfs for LosesRoadmap {
    fn create_dir_all(&mut self, dir: &std::path::Path) -> std::io::Result<()> {
        self.0.create_dir_all(dir)
    }
    fn list(&self, dir: &std::path::Path) -> std::io::Result<Vec<std::path::PathBuf>> {
        self.0.list(dir)
    }
    fn list_dirs(&self, dir: &std::path::Path) -> std::io::Result<Vec<std::path::PathBuf>> {
        self.0.list_dirs(dir)
    }
    fn read(&self, path: &std::path::Path) -> std::io::Result<Vec<u8>> {
        self.0.read(path)
    }
    fn size(&self, path: &std::path::Path) -> std::io::Result<u64> {
        self.0.size(path)
    }
    fn read_at(&self, path: &std::path::Path, offset: u64, len: u64) -> std::io::Result<Vec<u8>> {
        self.0.read_at(path, offset, len)
    }
    fn append(&mut self, path: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
        self.0.append(path, bytes)
    }
    fn write_at(
        &mut self,
        path: &std::path::Path,
        offset: u64,
        bytes: &[u8],
    ) -> std::io::Result<()> {
        self.0.write_at(path, offset, bytes)
    }
    fn truncate(&mut self, path: &std::path::Path, len: u64) -> std::io::Result<()> {
        self.0.truncate(path, len)
    }
    fn sync_data(&mut self, path: &std::path::Path) -> std::io::Result<()> {
        self.0.sync_data(path)
    }
    fn rename(&mut self, from: &std::path::Path, to: &std::path::Path) -> std::io::Result<()> {
        if Self::swallowed(to) {
            return self.0.remove_file(from);
        }
        self.0.rename(from, to)
    }
    fn sync_dir(&mut self, dir: &std::path::Path) -> std::io::Result<()> {
        self.0.sync_dir(dir)
    }
    fn remove_file(&mut self, path: &std::path::Path) -> std::io::Result<()> {
        self.0.remove_file(path)
    }
    fn exists(&self, path: &std::path::Path) -> bool {
        self.0.exists(path)
    }
    fn copy_permissions(
        &mut self,
        from: &std::path::Path,
        to: &std::path::Path,
    ) -> std::io::Result<()> {
        self.0.copy_permissions(from, to)
    }
}

#[test]
fn a_city_file_the_bundle_never_received_fails_the_export() {
    let home = tempfile::tempdir().unwrap();
    city_with(2, home.path());
    let carried = tempfile::tempdir().unwrap();
    let err = Bundle::export_with(
        Box::new(LosesRoadmap(RealFs::new())),
        home.path(),
        carried.path(),
    )
    .unwrap_err();
    let ax = err.into_ax();
    assert!(
        ax.subject().contains("city file"),
        "the export has to name the count that disagreed: {}",
        ax.subject()
    );
}

#[test]
fn a_short_copy_is_refused_rather_than_restored_quietly() {
    let home = tempfile::tempdir().unwrap();
    city_with(4, home.path());
    let carried = tempfile::tempdir().unwrap();
    Bundle::export(home.path(), carried.path()).unwrap();

    let segment = std::fs::read_dir(carried.path().join(LEDGER))
        .unwrap()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .find(|path| path.extension().and_then(|e| e.to_str()) == Some("jsonl"))
        .unwrap();
    let bytes = std::fs::read(&segment).unwrap();
    let cut = bytes
        .iter()
        .rposition(|byte| *byte == b'\n')
        .and_then(|last| bytes[..last].iter().rposition(|byte| *byte == b'\n'))
        .unwrap();
    std::fs::write(&segment, &bytes[..=cut]).unwrap();

    let elsewhere = tempfile::tempdir().unwrap();
    let err = Bundle::restore(carried.path(), elsewhere.path()).unwrap_err();
    let ax = err.into_ax();
    assert!(
        ax.subject().contains("record(s) ending"),
        "a partial copy is not a city: {}",
        ax.subject()
    );
}

#[test]
fn a_bundle_that_lost_its_objects_is_refused_rather_than_restored_empty() {
    let home = tempfile::tempdir().unwrap();
    city_with(2, home.path());
    let mut cas = crate::Cas::open(&home.path().join(RESERVED).join(CAS)).unwrap();
    cas.put(b"what a run offloaded").unwrap();
    drop(cas);

    let carried = tempfile::tempdir().unwrap();
    let exported = Bundle::export(home.path(), carried.path()).unwrap();
    assert_eq!(exported.cas_objects(), 1);

    std::fs::remove_dir_all(carried.path().join(CAS)).unwrap();

    let elsewhere = tempfile::tempdir().unwrap();
    let err = Bundle::restore(carried.path(), elsewhere.path()).unwrap_err();
    assert!(
        err.into_ax().subject().contains("cas object"),
        "the refusal has to name the count that disagreed"
    );
}

#[test]
fn a_city_is_never_restored_on_top_of_another() {
    let home = tempfile::tempdir().unwrap();
    city_with(2, home.path());
    let carried = tempfile::tempdir().unwrap();
    Bundle::export(home.path(), carried.path()).unwrap();

    let err = Bundle::restore(carried.path(), home.path()).unwrap_err();
    assert!(err.into_ax().subject().contains("already holds a ledger"));
}

#[test]
fn a_restore_through_a_link_to_a_reserved_path_is_refused() {
    let home = tempfile::tempdir().unwrap();
    city_with(2, home.path());
    let carried = tempfile::tempdir().unwrap();
    Bundle::export(home.path(), carried.path()).unwrap();

    let elsewhere = tempfile::tempdir().unwrap();
    let root = elsewhere.path();
    std::fs::create_dir_all(root.join(".git")).unwrap();
    if !crate::alias::tests::place_link(true, &root.join(".git"), &root.join("lab")) {
        return;
    }
    let err = Bundle::restore(carried.path(), root).unwrap_err();
    assert_eq!(
        *err.into_ax().code(),
        kernel::AxCode::OutsideWriteDomain,
        "the refusal names the write-domain rule, not a count"
    );
    assert!(
        !root.join(".git").join("Roadmap.md").exists(),
        "the reserved file the link reaches is never written"
    );
}

#[test]
fn a_restore_through_a_link_lands_no_directory_at_the_link_target() {
    let home = tempfile::tempdir().unwrap();
    city_with(1, home.path());
    let carried = tempfile::tempdir().unwrap();
    Bundle::export(home.path(), carried.path()).unwrap();

    let lab = carried.path().join(CITY).join("lab");
    std::fs::remove_file(lab.join("Roadmap.md")).unwrap();
    let nested = lab.join("room").join("Notes.md");
    std::fs::create_dir_all(nested.parent().unwrap()).unwrap();
    std::fs::write(&nested, b"# Notes\n").unwrap();

    let root = tempfile::tempdir().unwrap();
    let reserved = root.path().join(".git");
    std::fs::create_dir_all(&reserved).unwrap();
    if !crate::alias::tests::place_link(true, &reserved, &root.path().join("lab")) {
        return;
    }
    let err = Bundle::restore(carried.path(), root.path()).unwrap_err();
    assert_eq!(*err.into_ax().code(), kernel::AxCode::OutsideWriteDomain);
    assert_eq!(
        std::fs::read_dir(&reserved).unwrap().count(),
        0,
        "no directory is made through the link"
    );
}

#[test]
fn a_bundle_carrying_git_metadata_is_refused_rather_than_planted() {
    let home = tempfile::tempdir().unwrap();
    city_with(1, home.path());
    let hooks = home.path().join(".git").join("hooks");
    std::fs::create_dir_all(&hooks).unwrap();
    std::fs::write(hooks.join("post-checkout"), b"escalate").unwrap();
    let carried = tempfile::tempdir().unwrap();
    Bundle::export(home.path(), carried.path()).unwrap();
    assert!(
        !carried.path().join(CITY).join(".git").exists(),
        "git metadata is no city file and never travels"
    );

    let planted = carried.path().join(CITY).join(".git").join("hooks");
    std::fs::create_dir_all(&planted).unwrap();
    std::fs::write(planted.join("post-checkout"), b"escalate").unwrap();

    let elsewhere = tempfile::tempdir().unwrap();
    let err = Bundle::restore(carried.path(), elsewhere.path()).unwrap_err();
    assert!(
        err.into_ax().subject().contains("protected"),
        "the refusal names what it refuses"
    );
    assert!(
        !elsewhere.path().join(".git").exists(),
        "a refused restore plants nothing"
    );
}

pub(super) fn private(root: &std::path::Path) -> std::path::PathBuf {
    crate::ProjectRepository::at(root, &kernel::Address::parse("lab").unwrap())
        .unwrap()
        .root()
        .to_owned()
}

pub(super) fn log_of(root: &std::path::Path) -> Vec<String> {
    let Ok(repo) = git2::Repository::open(root) else {
        return Vec::new();
    };
    let mut walk = repo.revwalk().unwrap();
    walk.push_head().unwrap();
    walk.map(|oid| {
        let commit = repo.find_commit(oid.unwrap()).unwrap();
        format!("{} {:?}", commit.id(), commit.summary())
    })
    .collect()
}

pub(super) fn committed_twice(root: &std::path::Path) -> Vec<String> {
    let repo = git2::Repository::init(private(root)).unwrap();
    repo.set_workdir(root, false).unwrap();
    let who = git2::Signature::new("owner", "owner@city", &git2::Time::new(1, 0)).unwrap();
    let mut parents = Vec::new();
    for (step, file) in ["City.md", "lab/Roadmap.md"].into_iter().enumerate() {
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new(file)).unwrap();
        let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
        let parent_refs: Vec<&git2::Commit<'_>> = parents.iter().collect();
        let id = repo
            .commit(
                Some("HEAD"),
                &who,
                &who,
                &format!("step {step}"),
                &tree,
                &parent_refs,
            )
            .unwrap();
        parents = vec![repo.find_commit(id).unwrap()];
    }
    let log = log_of(&private(root));
    assert_eq!(log.len(), 2);
    log
}

#[test]
fn the_git_history_comes_back_commit_for_commit() {
    let home = tempfile::tempdir().unwrap();
    city_with(1, home.path());
    let before = committed_twice(home.path());

    let carried = tempfile::tempdir().unwrap();
    Bundle::export(home.path(), carried.path()).unwrap();
    let elsewhere = tempfile::tempdir().unwrap();
    Bundle::restore(carried.path(), elsewhere.path()).unwrap();

    assert_eq!(log_of(&private(elsewhere.path())), before);
    assert!(
        !elsewhere
            .path()
            .join(".git")
            .join("hooks")
            .join("post-checkout")
            .exists()
    );
}
