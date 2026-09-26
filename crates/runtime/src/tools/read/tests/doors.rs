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
    let tool = ReadTool::new(dir.path(), catalog, only_lab).unwrap();

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
    let package = dir.path().join(".sprawling").join("library").join("review");
    std::fs::create_dir_all(package.join("scripts")).unwrap();
    std::fs::write(package.join("SKILL.md"), "check the diff first\n").unwrap();
    std::fs::write(package.join("scripts").join("check.sh"), "git diff\n").unwrap();
    std::fs::write(dir.path().join(".sprawling").join("CONFIG.toml"), "x\n").unwrap();
    let (tool, catalog) = tool(dir.path());
    catalog
        .lock()
        .unwrap()
        .admit_skill(CatalogEntry {
            name: "review".to_owned(),
            disclosure: "how this building reviews".to_owned(),
            expansion: ".sprawling/library/review/SKILL.md".to_owned(),
            hash: None,
            package: Some(".sprawling/library/review".to_owned()),
        })
        .unwrap();

    make_link(&package.join("out"), &dir.path().join(".sprawling"));
    let outcome = tool.invoke(&call("review/scripts/check.sh")).unwrap();
    assert_eq!(outcome.result.as_map()["text"], "git diff\n");
    for (leaving, code) in [
        ("review/out/CONFIG.toml", AxCode::GateDenied),
        ("review/out/absent.md", AxCode::GateDenied),
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
        ["review/SKILL.md", "review/out", "review/scripts"],
        "a miss in a package offers the package's entries as the caller names them"
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
    let tool = ReadTool::new(dir.path(), Arc::new(Mutex::new(Catalog::new())), between).unwrap();

    let refused = tool.invoke(&call("lab/door/sub/secret.md"));
    assert!(
        matches!(&refused, Err(err) if err.code() == &AxCode::InvalidArgs),
        "a file absent at the check was opened through a link placed after it: {refused:?}"
    );
}

/// A file present at the check is read only while it is still the file
/// the check judged: a directory on its path swapped for a link between
/// the check and the open would lead the open wherever the link points.
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
    let tool = ReadTool::new(dir.path(), Arc::new(Mutex::new(Catalog::new())), between).unwrap();

    let refused = tool.invoke(&call("lab/door/sub/secret.md"));
    assert!(
        matches!(&refused, Err(err) if err.code() == &AxCode::GateDenied),
        "a file was read through a link swapped in after the check: {refused:?}"
    );
}

/// A single document that happens to be called `SKILL.md` is not a
/// package: its section holds skills the reading room did not admit.
#[test]
fn a_document_named_like_a_package_opens_nothing_beside_it() {
    let dir = tempfile::tempdir().unwrap();
    let section = dir.path().join(".sprawling").join("library").join("ops");
    std::fs::create_dir_all(&section).unwrap();
    std::fs::write(section.join("SKILL.md"), "how ops works\n").unwrap();
    std::fs::write(section.join("deploy.md"), "not admitted\n").unwrap();
    let (tool, catalog) = tool(dir.path());
    catalog
        .lock()
        .unwrap()
        .admit_skill(CatalogEntry {
            name: "SKILL".to_owned(),
            disclosure: "how ops works".to_owned(),
            expansion: ".sprawling/library/ops/SKILL.md".to_owned(),
            hash: None,
            package: None,
        })
        .unwrap();

    let refused = tool.invoke(&call("SKILL/deploy.md"));
    assert!(
        refused.is_err(),
        "a skill the reading room did not admit was read beside a document: {refused:?}"
    );
}
