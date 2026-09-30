// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A checkout whose packages do not sit at `crates/<name>`: the
//! directory, the package name and the lib name are three different
//! spellings, so a gate that derives a path from any one of them finds
//! nothing.

use std::path::{Path, PathBuf};

/// `sprawling-k` (lib `k`) under `tools/k`, and `sprawling-j` (lib `j`)
/// under `crates/j`, which depends on it; an `ARCHITECTURE.md` whose
/// depmap block allows exactly that edge, and a module map with one row
/// under `tools/`. Each test passes its own `label`, so tests running at
/// once never share a directory.
pub(crate) fn relocated(label: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("xtask-relocated-{label}-{}", std::process::id()));
    if root.exists() {
        std::fs::remove_dir_all(&root).unwrap();
    }
    for (path, text) in FILES {
        write(&root, path, text);
    }
    root
}

pub(crate) fn write(root: &Path, path: &str, text: &str) {
    let full = root.join(path);
    std::fs::create_dir_all(full.parent().unwrap()).unwrap();
    std::fs::write(full, text).unwrap();
}

const FILES: [(&str, &str); 8] = [
    (
        "Cargo.toml",
        "[workspace]\nmembers = [\"tools/k\", \"crates/j\"]\nresolver = \"3\"\n",
    ),
    (
        "tools/k/Cargo.toml",
        "[package]\nname = \"sprawling-k\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n[lib]\nname = \"k\"\n",
    ),
    (
        "tools/k/src/lib.rs",
        "#[cfg(kani)]\nmod verification {\n    #[kani::proof]\n    fn holds() {}\n}\n",
    ),
    ("tools/k/src/a.rs", ""),
    (
        "crates/j/Cargo.toml",
        "[package]\nname = \"sprawling-j\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n[lib]\nname = \"j\"\n\n\
         [dependencies]\nk = { package = \"sprawling-k\", path = \"../../tools/k\" }\n",
    ),
    ("crates/j/src/lib.rs", ""),
    (
        "ARCHITECTURE.md",
        "# Fixture\n\n## 2 Crates\n\n```depmap\nk:\nj: k\n```\n\n## 4 Seams\n\n```directions\n```\n",
    ),
    (
        "architecture.toml",
        "module = [\n  { name = \"k::a\", file = \"tools/k/src/a.rs\", owns = \"a module the map places under tools/\", \
         shape = \"value\", since = \"T\", status = \"built\", spec = \"k-SPEC.md#8-9\" },\n]\n",
    ),
];
