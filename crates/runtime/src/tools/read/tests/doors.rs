// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The doors a read must not pass: a link out of where the path was
//! admitted, a path out of a package, and a file that was not there
//! when the path was judged.

use super::*;
use crate::tools::chosen_path::make_link;

/// A link is judged where it lands, not where it was written: a door in
/// an open building leading into a confidential one, into a reserved
/// subtree, or out of the city opens nothing, and each refusal is the
/// gate's.
#[test]
fn a_link_is_judged_by_where_it_lands() {
    let dir = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    for room in ["lab", "vault/room1", ".sprawling"] {
        std::fs::create_dir_all(dir.path().join(room)).unwrap();
    }
    std::fs::write(dir.path().join("vault/room1/secret.md"), "the vault\n").unwrap();
    std::fs::write(dir.path().join(".sprawling/secret.md"), "governance\n").unwrap();
    std::fs::write(outside.path().join("secret.md"), "elsewhere\n").unwrap();
    let lab = dir.path().join("lab");
    make_link(
        &lab.join("to-vault"),
        &dir.path().join("vault").join("room1"),
    );
    make_link(&lab.join("to-reserved"), &dir.path().join(".sprawling"));
    make_link(&lab.join("to-outside"), outside.path());
    let only_lab: ReadBound = Arc::new(|addr: &kernel::Address| {
        if addr.as_str().starts_with("vault") {
            kernel::ReadVerdict::Confidential
        } else {
            kernel::ReadVerdict::Open
        }
    });
    let catalog = Arc::new(Mutex::new(Catalog::new()));
    let tool = ReadTool::new(dir.path(), catalog, only_lab, Path::new("no-store")).unwrap();

    for asked in [
        "lab/to-vault/secret.md",
        "lab/to-reserved/secret.md",
        "lab/to-outside/secret.md",
        "lab/to-vault/absent.md",
        "lab/to-outside/absent.md",
    ] {
        let refused = tool.invoke(&call(asked));
        assert!(
            matches!(&refused, Err(err) if err.code() == &AxCode::GateDenied),
            "{asked} was read through a link: {refused:?}"
        );
    }
}

/// The reading room admits a package whole: `<name>/<path>` opens a file
/// beside the package's `SKILL.md`, and a path that climbs out of the
/// package, by its segments or through a link, is refused.
#[test]
fn a_package_the_reading_room_admits_opens_by_name_and_path() {
    let dir = tempfile::tempdir().unwrap();
    let manifest = skill_manifest(
        dir.path(),
        &[
            ("SKILL.md", b"check the diff first\n"),
            ("scripts/check.sh", b"git diff\n"),
        ],
    );
    let (tool, catalog) = tool(dir.path());
    catalog
        .lock()
        .unwrap()
        .admit_skill(CatalogEntry {
            name: "review".to_owned(),
            disclosure: "how this building reviews".to_owned(),
            expansion: manifest.to_string(),
            hash: Some(manifest),
        })
        .unwrap();

    let outcome = tool.invoke(&call("review/scripts/check.sh")).unwrap();
    assert_eq!(outcome.result.as_map()["text"], "git diff\n");
    for (leaving, code) in [
        ("review/out/CONFIG.toml", AxCode::InvalidArgs),
        ("review/out/absent.md", AxCode::InvalidArgs),
        ("review/../../CONFIG.toml", AxCode::InvalidArgs),
        ("review/./SKILL.md", AxCode::InvalidArgs),
        ("review/", AxCode::InvalidArgs),
    ] {
        let err = tool.invoke(&call(leaving)).unwrap_err();
        assert_eq!(err.code(), &code, "{leaving} was opened");
    }
    let miss = tool.invoke(&call("review/scriptz/check.sh")).unwrap_err();
    assert_eq!(
        miss.nearby(),
        ["review/SKILL.md", "review/scripts/check.sh"],
        "a miss in a package offers the package's entries as the caller names them"
    );
}

/// A changed CAS object never reaches the model: the full read verifies
/// its digest rather than trusting the shelf's index.
#[test]
fn a_changed_cas_object_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let manifest = skill_manifest(dir.path(), &[("SKILL.md", b"on the shelf\n")]);
    let (tool, catalog) = tool(dir.path());
    catalog
        .lock()
        .unwrap()
        .admit_skill(CatalogEntry {
            name: "kept".to_owned(),
            disclosure: "the kept skill".to_owned(),
            expansion: manifest.to_string(),
            hash: Some(manifest),
        })
        .unwrap();
    let hex = manifest.to_string();
    let path = kernel::layout::CityLayout::new(dir.path())
        .cas()
        .join("b3")
        .join(&hex[..2])
        .join(&hex);
    let mut permissions = std::fs::metadata(&path).unwrap().permissions();
    permissions.set_readonly(false);
    std::fs::set_permissions(&path, permissions).unwrap();
    std::fs::write(&path, b"changed\n").unwrap();

    let refused = tool.invoke(&call("kept"));
    assert!(
        matches!(&refused, Err(err) if err.code() == &AxCode::CasCorrupt),
        "changed bytes were served: {refused:?}"
    );
}

/// A file that was not there when the path was judged is not opened
/// afterwards: a link placed at the absent segment between the check
/// and the open would lead the open wherever it points.
#[test]
fn a_file_absent_at_the_check_is_not_opened_later() {
    let dir = tempfile::tempdir().unwrap();
    for room in ["lab", "annex", "vault/room1"] {
        std::fs::create_dir_all(dir.path().join(room)).unwrap();
    }
    std::fs::write(dir.path().join("vault/room1/secret.md"), "the vault\n").unwrap();
    make_link(
        &dir.path().join("lab").join("door"),
        &dir.path().join("annex"),
    );
    let root = dir.path().to_path_buf();
    let between: ReadBound = Arc::new(move |addr: &kernel::Address| {
        let placed = root.join("annex").join("sub");
        if addr.as_str().starts_with("annex/") && !placed.exists() {
            make_link(&placed, &root.join("vault").join("room1"));
        }
        if addr.as_str().starts_with("vault") {
            kernel::ReadVerdict::Confidential
        } else {
            kernel::ReadVerdict::Open
        }
    });
    let tool = ReadTool::new(
        dir.path(),
        Arc::new(Mutex::new(Catalog::new())),
        between,
        Path::new("no-store"),
    )
    .unwrap();

    let refused = tool.invoke(&call("lab/door/sub/secret.md"));
    assert!(
        matches!(&refused, Err(err) if err.code() == &AxCode::InvalidArgs),
        "a file absent at the check was opened through a link placed after it: {refused:?}"
    );
}

/// A file present at the check is read only while it is still the file
/// the check judged: a directory on its path swapped for a link during
/// the check never leads the read into the vault. A read judged on the
/// handle it reads from already holds the judged file when the swap
/// happens, so the answer is that file or a refusal, never the vault.
#[test]
fn a_directory_swapped_for_a_link_after_the_check_opens_nothing() {
    let dir = tempfile::tempdir().unwrap();
    for room in ["lab", "annex/sub", "vault/room1"] {
        std::fs::create_dir_all(dir.path().join(room)).unwrap();
    }
    std::fs::write(dir.path().join("annex/sub/secret.md"), "the annex\n").unwrap();
    std::fs::write(dir.path().join("vault/room1/secret.md"), "the vault\n").unwrap();
    make_link(
        &dir.path().join("lab").join("door"),
        &dir.path().join("annex"),
    );
    let root = dir.path().to_path_buf();
    // The link on the way makes the check judge `annex/...` a second
    // time, after the real location was resolved: the swap lands there.
    let between: ReadBound = Arc::new(move |addr: &kernel::Address| {
        let checked = root.join("annex").join("sub");
        if addr.as_str() == "annex/sub/secret.md" && !checked.is_symlink() {
            std::fs::remove_dir_all(&checked).unwrap();
            make_link(&checked, &root.join("vault").join("room1"));
        }
        if addr.as_str().starts_with("vault") {
            kernel::ReadVerdict::Confidential
        } else {
            kernel::ReadVerdict::Open
        }
    });
    let tool = ReadTool::new(
        dir.path(),
        Arc::new(Mutex::new(Catalog::new())),
        between,
        Path::new("no-store"),
    )
    .unwrap();

    let answered = tool.invoke(&call("lab/door/sub/secret.md"));
    match &answered {
        Err(err) => assert_eq!(err.code(), &AxCode::GateDenied, "{err:?}"),
        Ok(opened) => assert_eq!(
            opened.result.as_map()["text"],
            "the annex
",
            "a file was read through a link swapped in during the check"
        ),
    }
}

/// A single document that happens to be called `SKILL.md` is not a
/// package: its section holds skills the reading room did not admit.
#[test]
fn a_document_named_like_a_package_opens_nothing_beside_it() {
    let dir = tempfile::tempdir().unwrap();
    let manifest = skill_manifest(dir.path(), &[("SKILL.md", b"how ops works\n")]);
    let (tool, catalog) = tool(dir.path());
    catalog
        .lock()
        .unwrap()
        .admit_skill(CatalogEntry {
            name: "SKILL".to_owned(),
            disclosure: "how ops works".to_owned(),
            expansion: manifest.to_string(),
            hash: Some(manifest),
        })
        .unwrap();

    let refused = tool.invoke(&call("SKILL/deploy.md"));
    assert!(
        refused.is_err(),
        "a skill the reading room did not admit was read beside a document: {refused:?}"
    );
}
