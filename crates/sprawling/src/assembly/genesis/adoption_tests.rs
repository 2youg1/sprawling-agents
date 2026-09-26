// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use crate::assembly::*;

/// Adopting a folder fences it at once, as one pack: the first dispatch
/// into a building of thousands of files would otherwise hash every one
/// of them and write each as a loose object before its first tool call
/// (memory-SPEC 8-8, the base fence).
#[test]
fn adopting_a_folder_fences_it_as_one_pack_before_any_dispatch() {
    let dir = tempfile::tempdir().unwrap();
    for name in ["a.md", "b.md", "c.md"] {
        std::fs::create_dir_all(dir.path().join("shop")).unwrap();
        std::fs::write(dir.path().join("shop").join(name), name).unwrap();
    }
    form_city(dir.path(), Adopt::EveryFolder).unwrap();

    let carried = git2::Repository::open(dir.path()).ok().is_some_and(|repo| {
        repo.head()
            .and_then(|head| head.peel_to_tree())
            .is_ok_and(|tree| tree.get_path(std::path::Path::new("shop/b.md")).is_ok())
    });
    let objects = dir.path().join(".git").join("objects");
    let loose = std::fs::read_dir(&objects)
        .map(|entries| {
            entries
                .flatten()
                .filter(|entry| entry.file_name().len() == 2)
                .count()
        })
        .unwrap_or(0);
    let packs = std::fs::read_dir(objects.join("pack"))
        .map(|entries| {
            entries
                .flatten()
                .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "pack"))
                .count()
        })
        .unwrap_or(0);
    assert_eq!((carried, loose, packs), (true, 0, 1));
}
