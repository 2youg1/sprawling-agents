// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A Locator is read where the bytes belong: a `cas:` block at the
//! building of the ledger line that referenced it, a `file:` at its
//! address, and a block nobody in this lineage referenced not at all.

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
fn every_cas_block_is_refused_until_its_building_is_recorded_at_store_time() {
    let dir = tempfile::tempdir().unwrap();
    let cas_dir = kernel::layout::CityLayout::new(dir.path()).cas();
    let mut cas = memory::Cas::open(&cas_dir).unwrap();
    let in_lab = cas.put(b"lab notes\n").unwrap();
    let in_vault = cas.put(b"vault notes\n").unwrap();
    let stray = kernel::B3Hash::digest(b"never referenced");
    let owner: BlockOwner = Arc::new(move |hash: &kernel::B3Hash| {
        Ok(if *hash == in_lab {
            Some(kernel::Address::parse("lab").unwrap())
        } else if *hash == in_vault {
            Some(kernel::Address::parse("vault").unwrap())
        } else {
            None
        })
    });
    let mut tool = ReadTool::new(
        dir.path(),
        Arc::new(Mutex::new(Catalog::new())),
        only_lab(),
        Blocks {
            store: cas_dir,
            owner,
        },
    )
    .unwrap();

    for refused in [in_lab, in_vault, stray] {
        let err = tool
            .invoke(&call(&format!("cas:b3-{refused}")))
            .unwrap_err();
        assert_eq!(err.code(), &AxCode::GateDenied, "{refused}");
    }
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
    let mut tool = ReadTool::new(
        root,
        Arc::new(Mutex::new(Catalog::new())),
        only_lab(),
        unreferenced(),
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
