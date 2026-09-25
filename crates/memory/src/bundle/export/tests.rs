// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Fixtures for exporting, restoring and weighing a bundle (memory-SPEC 8-12).

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
    // The work came with it, not only the history.
    assert!(elsewhere.path().join("City.md").exists());
    assert!(elsewhere.path().join("lab").join("Roadmap.md").exists());
    // And the restored city is one a writer can continue.
    open_restored(elsewhere.path(), TimeMs::new(9)).unwrap();
}

/// A filesystem that accepts one write and does not keep it, which
/// is what a full disk and a cancelled copy look like from here.
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
        if Self::swallowed(path) {
            return Ok(());
        }
        self.0.append(path, bytes)
    }
    fn truncate(&mut self, path: &std::path::Path, len: u64) -> std::io::Result<()> {
        self.0.truncate(path, len)
    }
    fn sync_data(&mut self, path: &std::path::Path) -> std::io::Result<()> {
        if Self::swallowed(path) {
            return Ok(());
        }
        self.0.sync_data(path)
    }
    fn rename(&mut self, from: &std::path::Path, to: &std::path::Path) -> std::io::Result<()> {
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
}

/// The manifest is read back from the bundle, so on its own it
/// cannot tell a complete bundle from a short one. This is the case
/// that the source-side counts exist for.
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

    // Lose the last record, as an interrupted copy would.
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

/// A bundle can lose every object it carries and still verify as a
/// history: the chain is intact, and each Locator in it points at
/// nothing. Only the object count says so, which is why restore
/// compares four numbers rather than two.
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

    // The copy reached another machine with its objects missing.
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

    // The city it came from still has its ledger, so restoring back
    // onto it would be a merge of two histories.
    let err = Bundle::restore(carried.path(), home.path()).unwrap_err();
    assert!(err.into_ax().subject().contains("already holds a ledger"));
}

/// The acceptance at the restore door: a write whose path crosses a
/// link into the reserved subtree is refused whole, and the file
/// the link reaches keeps - or gains - nothing.
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
        // This machine hands out no link: nothing could have been
        // written through one.
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

/// A restore whose destination reaches a protected path through a
/// link is refused whole, and the link's target gains no directory:
/// the alias check precedes directory creation, so no parent is ever
/// made through the link (memory-SPEC 8-25).
#[test]
fn a_restore_through_a_link_lands_no_directory_at_the_link_target() {
    let home = tempfile::tempdir().unwrap();
    city_with(1, home.path());
    let carried = tempfile::tempdir().unwrap();
    Bundle::export(home.path(), carried.path()).unwrap();
    // A nested city file, so its parent must be created at the
    // destination - that creation is the step the rule guards. The
    // shallow sibling is removed so this nested file is the first
    // one the walk reaches under the link.
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

/// The reserved predicate reaches write targets at the restore door:
/// git metadata is no city file, never travels, and a bundle that
/// carries it is forged - restoring it would plant hooks.
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

    // What a forged bundle looks like: the metadata arrives beside
    // the city files after the export.
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

/// Every commit subject from `HEAD` back, newest first; a directory
/// that is no repository has no history to list.
fn log_of(root: &std::path::Path) -> Vec<String> {
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

/// The history is part of the city: a restored city that lost its
/// commits has lost every checkpoint a rollback could reach.
/// Makes `root` a repository of two commits and returns its log.
fn committed_twice(root: &std::path::Path) -> Vec<String> {
    let repo = git2::Repository::init(root).unwrap();
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
    let log = log_of(root);
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

    assert_eq!(log_of(elsewhere.path()), before);
    assert!(
        !elsewhere
            .path()
            .join(".git")
            .join("hooks")
            .join("post-checkout")
            .exists()
    );
}

fn copy_dir(from: &std::path::Path, to: &std::path::Path) -> u64 {
    std::fs::create_dir_all(to).unwrap();
    let mut copied = 0;
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copied += copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), target).unwrap();
            copied += 1;
        }
    }
    copied
}

/// A v0.0.6 export copied the repository whole into `city/.git` and
/// counted its files in the manifest; its objects and refs come back,
/// its hooks and config do not.
#[test]
fn a_v006_bundle_brings_back_its_history_and_not_its_hooks() {
    let home = tempfile::tempdir().unwrap();
    city_with(1, home.path());
    let before = committed_twice(home.path());
    let hooks = home.path().join(".git").join("hooks");
    std::fs::create_dir_all(&hooks).unwrap();
    std::fs::write(hooks.join("post-checkout"), b"escalate").unwrap();
    let carried = tempfile::tempdir().unwrap();
    Bundle::export(home.path(), carried.path()).unwrap();
    std::fs::remove_dir_all(carried.path().join(super::history::HISTORY)).unwrap();
    let whole = copy_dir(
        &home.path().join(".git"),
        &carried.path().join(CITY).join(".git"),
    );
    let at = carried.path().join(MANIFEST);
    let mut manifest = Manifest::from_json(&std::fs::read(&at).unwrap(), &at).unwrap();
    manifest.files += whole;
    std::fs::write(&at, manifest.to_json()).unwrap();

    let elsewhere = tempfile::tempdir().unwrap();
    let restored = Bundle::restore(carried.path(), elsewhere.path()).map_err(|e| e.to_string());
    assert_eq!(restored.map(|_| ()), Ok(()));
    assert_eq!(log_of(elsewhere.path()), before);
    let git = elsewhere.path().join(".git");
    assert!(!git.join("hooks").join("post-checkout").exists());
}
