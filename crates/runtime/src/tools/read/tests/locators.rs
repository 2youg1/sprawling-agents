// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A Locator is read where the bytes belong: a `cas:` block at the
//! building it was put for, a `file:` at its address, and a block put
//! for no building not at all.

use super::*;

fn only_lab() -> ReadBound {
    Arc::new(|addr: &kernel::Address| {
        if addr.as_str().starts_with("vault") {
            kernel::ReadVerdict::Confidential
        } else {
            kernel::ReadVerdict::Open
        }
    })
}

fn git(root: &Path, args: &[&str]) -> String {
    let out = std::process::Command::new("git")
        .args([
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .current_dir(root)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap().trim().to_owned()
}

#[test]
fn a_cas_block_is_read_at_the_building_it_was_put_for() {
    let dir = tempfile::tempdir().unwrap();
    let cas_dir = kernel::layout::CityLayout::new(dir.path()).cas();
    let mut cas = memory::Cas::open(&cas_dir).unwrap();
    let put_in = |building: &str| memory::BlockOrigin {
        run: kernel::RunId::from_bytes([7; 16]),
        building: kernel::Address::parse(building).unwrap(),
    };
    let in_lab = cas.put_for(b"lab notes\n", &put_in("lab")).unwrap();
    let in_vault = cas.put_for(b"vault notes\n", &put_in("vault")).unwrap();
    let with_no_origin = cas.put(b"shelved\n").unwrap();
    let stray = kernel::B3Hash::digest(b"never put");
    let tool = ReadTool::new(
        dir.path(),
        Arc::new(Mutex::new(Catalog::new())),
        only_lab(),
        &cas_dir,
    )
    .unwrap();

    let read = tool.invoke(&call(&format!("cas:b3-{in_lab}"))).unwrap();
    assert_eq!(read.result.as_map()["text"], "lab notes\n");
    for refused in [in_vault, with_no_origin, stray] {
        let err = tool
            .invoke(&call(&format!("cas:b3-{refused}")))
            .unwrap_err();
        assert_eq!(err.code(), &AxCode::GateDenied, "{refused}");
    }
}

/// A block put for several buildings is read where any of them opens to
/// the reader, and refused, when none does, with the reason of the first
/// building it was put for.
#[test]
fn a_cas_block_put_for_two_buildings_is_judged_at_the_first_that_opens() {
    let dir = tempfile::tempdir().unwrap();
    let cas_dir = kernel::layout::CityLayout::new(dir.path()).cas();
    let mut cas = memory::Cas::open(&cas_dir).unwrap();
    let put_in = |building: &str| memory::BlockOrigin {
        run: kernel::RunId::from_bytes([7; 16]),
        building: kernel::Address::parse(building).unwrap(),
    };
    let shared = cas.put_for(b"shared notes\n", &put_in("vault")).unwrap();
    cas.put_for(b"shared notes\n", &put_in("lab")).unwrap();
    let closed = cas.put_for(b"closed notes\n", &put_in("vault")).unwrap();
    cas.put_for(b"closed notes\n", &put_in("mill")).unwrap();
    let tool = |bound: ReadBound| {
        ReadTool::new(
            dir.path(),
            Arc::new(Mutex::new(Catalog::new())),
            bound,
            &cas_dir,
        )
        .unwrap()
    };
    let neither_vault_nor_mill: ReadBound =
        Arc::new(|addr: &kernel::Address| match addr.as_str() {
            "vault" => kernel::ReadVerdict::Confidential,
            "mill" => kernel::ReadVerdict::RulesUnreadable(
                AxError::failure(AxCode::InvalidArgs, "read the rules", "mill/RULES.toml")
                    .with_recovery("fix the rules"),
            ),
            _ => kernel::ReadVerdict::Open,
        });

    let read = tool(only_lab())
        .invoke(&call(&format!("cas:b3-{shared}")))
        .unwrap();
    assert_eq!(read.result.as_map()["text"], "shared notes\n");
    let err = tool(neither_vault_nor_mill)
        .invoke(&call(&format!("cas:b3-{closed}")))
        .unwrap_err();
    assert_eq!(err.code(), &AxCode::GateDenied);
    assert!(err.to_string().contains("confidential"), "{err}");
}

#[test]
fn a_file_locator_reads_the_bytes_of_its_commit_under_its_address_bound() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    git(root, &["init", "-q"]);
    for building in ["lab", "vault"] {
        std::fs::create_dir_all(root.join(building)).unwrap();
        std::fs::write(root.join(building).join("Memo.md"), "at the commit\n").unwrap();
    }
    git(root, &["add", "."]);
    git(root, &["commit", "-q", "-m", "one"]);
    let oid = git(root, &["rev-parse", "HEAD"]);
    std::fs::write(root.join("lab").join("Memo.md"), "changed since\n").unwrap();
    let tool = ReadTool::new(
        root,
        Arc::new(Mutex::new(Catalog::new())),
        only_lab(),
        Path::new("no-store"),
    )
    .unwrap();

    let read = tool
        .invoke(&call(&format!("file:lab/Memo.md@{oid}")))
        .unwrap();
    assert_eq!(read.result.as_map()["text"], "at the commit\n");
    let err = tool
        .invoke(&call(&format!("file:vault/Memo.md@{oid}")))
        .unwrap_err();
    assert_eq!(err.code(), &AxCode::GateDenied);
}
