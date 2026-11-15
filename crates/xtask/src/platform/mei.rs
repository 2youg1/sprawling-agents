// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::Platform;

pub(crate) const MEI_ROOT_PACKAGE: &str = "@sprawling/mei";
pub(crate) const MEI_ARCHIVE_PREFIX: &str = "mei-";
pub(crate) const MEI_BINARY: &str = "mei";
pub(crate) const CALYX_BINARY: &str = "calyx";

pub(crate) fn mei_package(row: &Platform) -> Option<&'static str> {
    match (row.os, row.cpu) {
        ("darwin", "arm64") => Some("@sprawling/mei-darwin-arm64"),
        ("linux", "x64") => Some("@sprawling/mei-linux-x64-musl"),
        ("win32", "x64") => Some("@sprawling/mei-windows-x64"),
        _ => None,
    }
}

pub(crate) fn mei_binary(row: &Platform) -> &'static str {
    if row.os == "win32" {
        "mei.exe"
    } else {
        MEI_BINARY
    }
}

pub(crate) fn calyx_binary(row: &Platform) -> &'static str {
    if row.os == "win32" {
        "calyx.exe"
    } else {
        CALYX_BINARY
    }
}
