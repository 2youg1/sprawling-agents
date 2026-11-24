// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::fs;

use wire::{ReadingPosition, WorkspaceInfo, WorkspaceSave};

use super::listing::DIRECTORY_PAGE;

use super::*;

struct Library {
    _volume: tempfile::TempDir,
    city: PathBuf,
    root: PathBuf,
}

impl Library {
    fn temporary() -> Self {
        let volume = tempfile::tempdir().unwrap();
        let city = volume.path().canonicalize().unwrap().join("city");
        let root = volume.path().join("papers");
        fs::create_dir_all(city.join(kernel::RESERVED_PREFIX)).unwrap();
        fs::create_dir_all(root.join("Archive")).unwrap();
        Self {
            _volume: volume,
            city,
            root,
        }
    }

    fn register(&self, registry: &mut Registry) -> WorkspaceInfo {
        match request(
            registry,
            WorkspaceRequest::Register {
                root: self.root.display().to_string(),
            },
        )
        .unwrap()
        {
            WorkspaceReply::Registered(info) => info,
            _ => panic!("expected a registration reply"),
        }
    }
}

fn request(registry: &mut Registry, request: WorkspaceRequest) -> Result<WorkspaceReply, AxError> {
    registry.act(WorkspaceOperation::Request(request), TimeMs::new(1_234))
}

fn open(registry: &mut Registry, workspace: B3Hash, target: WorkspaceTarget) -> WorkspaceRecord {
    match request(registry, WorkspaceRequest::Open { workspace, target }).unwrap() {
        WorkspaceReply::Record(record) => record,
        _ => panic!("expected an open reply"),
    }
}

fn path(raw: &str) -> WorkspaceTarget {
    WorkspaceTarget::Path {
        path: raw.to_owned(),
    }
}

fn progress(registry: &mut Registry, workspace: B3Hash, id: B3Hash, page: u32) -> ReadingPosition {
    let progress = ReadingPosition { page, offset: 0.41 };
    request(
        registry,
        WorkspaceRequest::Remember {
            workspace,
            id,
            changes: vec![
                WorkspaceChange::Progress { progress },
                WorkspaceChange::Category {
                    category: Some("Research".to_owned()),
                },
            ],
        },
    )
    .unwrap();
    progress
}

#[test]
fn workspace_move_restart_and_copy_have_their_own_reading_positions() {
    let fixture = Library::temporary();
    let original = fixture.root.join("paper.pdf");
    fs::write(&original, b"%PDF-1.7\nexample").unwrap();
    let mut before = Registry::new(fixture.city.clone());
    let workspace = fixture.register(&mut before).id;
    let document = open(&mut before, workspace, path("paper.pdf"));
    let expected = progress(&mut before, workspace, document.id, 14);
    drop(before);

    let renamed = fixture.root.join("renamed.pdf");
    let moved = fixture.root.join("Archive/paper.pdf");
    fs::rename(original, &renamed).unwrap();
    fs::rename(renamed, &moved).unwrap();
    fs::copy(moved, fixture.root.join("copy.pdf")).unwrap();

    let mut after = Registry::new(fixture.city.clone());
    let recovered = open(
        &mut after,
        workspace,
        WorkspaceTarget::Record { id: document.id },
    );
    assert_eq!(recovered.id, document.id);
    assert_eq!(recovered.path, "Archive/paper.pdf");
    assert_eq!(recovered.progress, Some(expected));
    assert_eq!(recovered.category.as_deref(), Some("Research"));
    let copy = open(&mut after, workspace, path("copy.pdf"));
    assert_eq!(copy.version, recovered.version);
    assert_ne!(copy.id, recovered.id);
    assert_eq!(copy.progress, None);
    assert_eq!(copy.category, None);
    progress(&mut after, workspace, copy.id, 1);
    assert_eq!(
        open(
            &mut after,
            workspace,
            WorkspaceTarget::Record { id: recovered.id }
        )
        .progress,
        Some(expected)
    );
}

#[test]
fn workspace_save_refuses_external_edits_and_retries_one_checked_replacement() {
    let fixture = Library::temporary();
    let document = fixture.root.join("notes.md");
    fs::write(&document, b"# Start\r\n").unwrap();
    let mut registry = Registry::new(fixture.city.clone());
    let workspace = fixture.register(&mut registry).id;
    let original = open(&mut registry, workspace, path("notes.md"));
    fs::write(&document, b"# From the file manager\r\n").unwrap();
    let save = |baseline| {
        WorkspaceOperation::Save(
            WorkspaceSave {
                workspace,
                id: original.id,
                baseline,
            },
            axum::body::Bytes::from_static(b"# Accepted\r\n"),
        )
    };
    let error = registry
        .act(save(original.version), TimeMs::new(1))
        .err()
        .unwrap();
    assert_eq!(*error.code(), AxCode::VersionConflict);
    assert_eq!(fs::read(&document).unwrap(), b"# From the file manager\r\n");
    let updated = open(&mut registry, workspace, path("notes.md"));
    assert_eq!(updated.id, original.id);
    registry.act(save(updated.version), TimeMs::new(2)).unwrap();
    drop(registry);
    let mut restarted = Registry::new(fixture.city.clone());
    let saved = match restarted
        .act(save(updated.version), TimeMs::new(3))
        .unwrap()
    {
        WorkspaceReply::Record(saved) => saved,
        _ => panic!("expected an idempotent save reply"),
    };
    assert_eq!(saved.id, original.id);
    assert_eq!(fs::read(&document).unwrap(), b"# Accepted\r\n");
    assert_eq!(saved.version, B3Hash::digest(b"# Accepted\r\n"));
}

#[test]
fn workspace_recovery_recognizes_an_interrupted_save_by_the_stage_identity() {
    let fixture = Library::temporary();
    fs::write(fixture.root.join("draft.md"), b"Before").unwrap();
    let mut registry = Registry::new(fixture.city.clone());
    let workspace = fixture.register(&mut registry).id;
    let exposed = open(&mut registry, workspace, path("draft.md"));
    let held = registry.workspace(workspace).unwrap();
    let root = held.open().unwrap();
    let original = root.read("draft.md").unwrap();
    let mut record = held.record(exposed.id).unwrap().clone();
    let nonce = identity_token().unwrap();
    let bytes = b"After interruption";
    let stage = root
        .stage("draft.md", nonce, bytes, original.permissions)
        .unwrap();
    record.pending = Some(Pending {
        stage: nonce,
        identity: stage.id.clone(),
        version: B3Hash::digest(bytes),
        baseline: original.version,
    });
    registry.keep(workspace, record).unwrap();
    let root = registry.workspace(workspace).unwrap().open().unwrap();
    stage
        .replace(&root, "draft.md", &original.id, original.version)
        .unwrap();
    drop(registry);

    let mut restarted = Registry::new(fixture.city.clone());
    let recovered = open(
        &mut restarted,
        workspace,
        WorkspaceTarget::Record { id: exposed.id },
    );
    assert_eq!(recovered.id, exposed.id);
    assert_eq!(recovered.version, B3Hash::digest(bytes));
    assert!(
        restarted
            .workspace(workspace)
            .unwrap()
            .record(exposed.id)
            .unwrap()
            .pending
            .is_none()
    );
}

#[test]
fn workspace_directory_pages_are_bounded_and_do_not_descend_until_opened() {
    let fixture = Library::temporary();
    let files = 10_000_u32;
    for n in 0..files {
        fs::write(fixture.root.join(format!("{n:05}.txt")), []).unwrap();
    }
    fs::write(fixture.root.join("Archive/folded.txt"), []).unwrap();
    let mut registry = Registry::new(fixture.city.clone());
    let workspace = fixture.register(&mut registry).id;
    let mut total = 0;
    let mut cursor = None;
    let mut last = None;
    loop {
        let page = registry.list(workspace, "", cursor.as_deref()).unwrap();
        assert!(page.entries.len() <= DIRECTORY_PAGE);
        for entry in page.entries {
            assert_ne!(entry.name, "folded.txt");
            if let Some(last) = last {
                assert!(last < entry.name || last == "Archive");
            }
            last = Some(entry.name);
            total += 1;
        }
        cursor = page.next;
        if cursor.is_none() {
            break;
        }
    }
    assert_eq!(total, usize::try_from(files).unwrap() + 1);
    assert_eq!(
        registry
            .list(workspace, "Archive", None)
            .unwrap()
            .entries
            .first()
            .unwrap()
            .name,
        "folded.txt"
    );
}

#[cfg(unix)]
#[test]
fn workspace_links_protected_paths_and_atomic_external_replacements_stay_distinct() {
    let fixture = Library::temporary();
    let external = fixture._volume.path().join("external.txt");
    fs::write(&external, b"outside").unwrap();
    std::os::unix::fs::symlink(&external, fixture.root.join("aliased.txt")).unwrap();
    fs::hard_link(&external, fixture.root.join("hard.txt")).unwrap();
    fs::write(fixture.root.join("tracked.md"), b"former").unwrap();
    let mut registry = Registry::new(fixture.city.clone());
    let workspace = fixture.register(&mut registry).id;
    for forbidden in ["../external.txt", "aliased.txt", "hard.txt", ".git/config"] {
        let error = request(
            &mut registry,
            WorkspaceRequest::Resource {
                workspace,
                path: forbidden.to_owned(),
            },
        )
        .err()
        .unwrap();
        assert_eq!(*error.code(), AxCode::GateDenied);
    }
    let previous = open(&mut registry, workspace, path("tracked.md"));
    fs::write(fixture.root.join("external-replacement.md"), b"new").unwrap();
    fs::rename(
        fixture.root.join("external-replacement.md"),
        fixture.root.join("tracked.md"),
    )
    .unwrap();
    let old = request(
        &mut registry,
        WorkspaceRequest::Open {
            workspace,
            target: WorkspaceTarget::Record { id: previous.id },
        },
    )
    .err()
    .unwrap();
    assert_eq!(*old.code(), AxCode::VersionConflict);
    let replacement = open(&mut registry, workspace, path("tracked.md"));
    assert_ne!(replacement.id, previous.id);
    assert_eq!(replacement.progress, None);
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn workspace_native_watch_wakes_for_renames_without_reenumerating() {
    let fixture = Library::temporary();
    let from = fixture.root.join("before.md");
    fs::write(&from, b"a note").unwrap();
    let mut registry = Registry::new(fixture.city.clone());
    let workspace = fixture.register(&mut registry).id;
    let page = registry.list(workspace, "", None).unwrap();
    let revision = page
        .revision
        .expect("inotify accepts the open scoped directory");
    let mut signal = registry
        .observe(workspace, "", revision.clone())
        .unwrap()
        .observed
        .unwrap();
    assert_eq!(signal.borrow().as_ref(), Some(&revision));
    fs::rename(from, fixture.root.join("after.md")).unwrap();
    let received = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        signal.wait_for(|new| new.as_ref() != Some(&revision)),
    )
    .await
    .unwrap()
    .unwrap();
    assert!(received.is_some());
    drop(received);
    assert!(
        registry
            .list(workspace, "", None)
            .unwrap()
            .entries
            .iter()
            .any(|entry| entry.name == "after.md")
    );
}

#[test]
fn workspace_final_landing_checks_the_original_after_preparing_the_stage() {
    let fixture = Library::temporary();
    let path = fixture.root.join("draft.md");
    fs::write(&path, b"starting version").unwrap();
    let mut registry = Registry::new(fixture.city.clone());
    let workspace = fixture.register(&mut registry).id;
    let root = registry.workspace(workspace).unwrap().open().unwrap();
    let original = root.read("draft.md").unwrap();
    let prepared = root
        .stage(
            "draft.md",
            identity_token().unwrap(),
            b"approved",
            original.permissions,
        )
        .unwrap();
    fs::write(&path, b"external edit after staging").unwrap();
    let error = prepared
        .replace(&root, "draft.md", &original.id, original.version)
        .unwrap_err();
    assert_eq!(*error.code(), AxCode::VersionConflict);
    assert_eq!(fs::read(&path).unwrap(), b"external edit after staging");
    prepared.discard().unwrap();
}
