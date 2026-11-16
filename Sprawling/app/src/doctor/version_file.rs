// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The version an installer wrote beside a browser (`crates/sprawling/spec/Doctor.lean`
//! §8-80).
//!
//! **A browser is never started to learn its version.** On Windows a
//! Chromium browser given `--version` opens a window instead of
//! printing a line, and when one is already running the new process
//! hands the request to it and exits, so the window outlives every
//! attempt to stop the child. A health check that opens browser windows
//! is worse than one that reports no version at all.
//!
//! Three readings in one order, and no branch on which family the path
//! belongs to - that question is answered by `family::claims` and
//! `family::member_at`, and answering it again here would give it a
//! second home:
//!
//! 1. `application.ini`, which every Gecko browser ships beside its
//!    program and, inside a macOS bundle, under `../Resources`.
//! 2. A directory beside the program named for a version, which every
//!    Chromium installer writes: `Application\<x.y.z.w>\` on Windows and
//!    `Contents/Frameworks/<name>.framework/Versions/<x.y.z.w>` inside a
//!    macOS bundle.
//! 3. `CFBundleShortVersionString` out of the bundle's `Info.plist`,
//!    which is the only one of the three Safari has.
//!
//! Nothing here starts a process, and nothing here decides whether the
//! item is present: a version that cannot be read is `Version::Silent`,
//! which is the same answer a program that printed nothing gives.

use std::path::{Path, PathBuf};

use super::Version;

/// The version of the program at this path, out of what its installer
/// wrote beside it.
pub(super) fn beside(program: &Path) -> Version {
    let Some(dir) = program.parent() else {
        return Version::Silent;
    };
    application_ini(dir)
        .or_else(|| version_directory(dir))
        .or_else(|| bundle_plist(dir))
        .map_or(Version::Silent, Version::Said)
}

/// How much of a file this reads before deciding it is not the small
/// text file it was looking for. Every `application.ini` and every
/// `Info.plist` a browser ships is a few kilobytes; a doctor that read
/// an arbitrarily large file into memory to find one line would be
/// paying for somebody else's mistake.
const MOST_BYTES: u64 = 64 * 1024;

/// `[App] Version` out of the `application.ini` beside the program, or
/// out of the one a macOS bundle keeps in `Resources`.
fn application_ini(dir: &Path) -> Option<String> {
    let beside = dir.join("application.ini");
    let in_bundle = dir.parent().map(|up| up.join("Resources/application.ini"));
    [Some(beside), in_bundle]
        .into_iter()
        .flatten()
        .find_map(|path| small_text(&path))
        .and_then(|text| app_version(&text))
}

/// The value of `Version` in the `[App]` section, and in no other:
/// `[Gecko] MinVersion` is the engine this build needs rather than the
/// version this browser is.
fn app_version(text: &str) -> Option<String> {
    let mut in_app = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with(';') || line.starts_with('#') {
            continue;
        }
        if let Some(section) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            in_app = section.eq_ignore_ascii_case("App");
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if in_app && key.trim().eq_ignore_ascii_case("Version") && !value.trim().is_empty() {
            return Some(value.trim().to_owned());
        }
    }
    None
}

/// The newest version-named directory a Chromium installer left beside
/// the program, or inside the framework a macOS bundle carries.
fn version_directory(dir: &Path) -> Option<String> {
    let frameworks = dir.parent().map(|up| up.join("Frameworks"));
    let inside: Vec<PathBuf> = frameworks
        .into_iter()
        .flat_map(|at| entries(&at))
        .filter(|path| path.extension().is_some_and(|kind| kind == "framework"))
        .map(|path| path.join("Versions"))
        .collect();
    std::iter::once(dir.to_path_buf())
        .chain(inside)
        .filter_map(|at| newest_in(&at))
        .max()
        .map(|(_, name)| name)
}

/// The greatest version-named directory in one directory, as the
/// numbers it is made of and as it is spelled.
///
/// Compared by number and never by spelling: an upgrade leaves the
/// directory it replaced behind, and `154.0.4258.9` sorts after
/// `154.0.4258.32` in every ordering that reads them as text.
fn newest_in(at: &Path) -> Option<(Vec<u64>, String)> {
    entries(at)
        .into_iter()
        .filter(|path| path.is_dir())
        .filter_map(|path| {
            let name = path.file_name()?.to_str()?.to_owned();
            Some((numbered(&name)?, name))
        })
        .max()
}

/// The numbers a version-named directory is made of, when that is what
/// the name is. A name with one number or with a part that is not a
/// number belongs to something else in that directory.
fn numbered(name: &str) -> Option<Vec<u64>> {
    let parts: Vec<&str> = name.split('.').collect();
    if parts.len() < 2 {
        return None;
    }
    parts
        .into_iter()
        .map(|part| part.parse::<u64>().ok())
        .collect()
}

/// `CFBundleShortVersionString` out of the bundle above the program.
fn bundle_plist(dir: &Path) -> Option<String> {
    let text = small_text(&dir.parent()?.join("Info.plist"))?;
    let (_, after) = text.split_once("<key>CFBundleShortVersionString</key>")?;
    let (_, value) = after.split_once("<string>")?;
    let (version, _) = value.split_once("</string>")?;
    let version = version.trim();
    (!version.is_empty()).then(|| version.to_owned())
}

/// The paths in one directory, and an empty list for a directory this
/// machine does not have or will not open. A directory that cannot be
/// read is not a fault of the browser beside it: the next reading
/// answers, or the version is silent.
fn entries(at: &Path) -> Vec<PathBuf> {
    let Ok(listing) = std::fs::read_dir(at) else {
        return Vec::new();
    };
    listing
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .collect()
}

/// One small text file, or nothing when it is absent, larger than a
/// configuration file has any reason to be, or not text.
fn small_text(path: &Path) -> Option<String> {
    let size = std::fs::metadata(path).ok()?.len();
    if size > MOST_BYTES {
        return None;
    }
    std::fs::read_to_string(path).ok()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::{Version, beside};

    /// The three layouts an installer leaves behind, each answering
    /// without the browser being started, and a program with nothing
    /// beside it answering that this machine cannot read its version.
    #[test]
    fn a_browser_says_its_version_through_the_files_beside_it() {
        let root = tempfile::tempdir().unwrap();
        let said = |text: &str| Version::Said(text.to_owned());

        // Gecko on Windows and Linux: `application.ini` beside the
        // program, with the engine's own version in another section.
        let zen = root.path().join("Zen Browser");
        std::fs::create_dir_all(&zen).unwrap();
        std::fs::write(
            zen.join("application.ini"),
            "; a comment the installer leaves\n[App]\nVendor=Mozilla\nName=Zen\n\
             Version=1.22.2b\n\n[Gecko]\nMinVersion=156.0\n",
        )
        .unwrap();
        assert_eq!(beside(&zen.join("zen.exe")), said("1.22.2b"));

        // Gecko inside a macOS bundle: one directory up, in Resources.
        let firefox = root.path().join("Firefox.app").join("Contents");
        std::fs::create_dir_all(firefox.join("MacOS")).unwrap();
        std::fs::create_dir_all(firefox.join("Resources")).unwrap();
        std::fs::write(
            firefox.join("Resources").join("application.ini"),
            "[App]\nName=Firefox\nVersion=133.0.3\n",
        )
        .unwrap();
        assert_eq!(beside(&firefox.join("MacOS/firefox")), said("133.0.3"));

        // Chromium on Windows: the upgrade left the directory it
        // replaced behind, and the newer one is not the later spelling.
        let edge = root.path().join("Edge/Application");
        std::fs::create_dir_all(edge.join("154.0.4258.9")).unwrap();
        std::fs::create_dir_all(edge.join("154.0.4258.32")).unwrap();
        assert_eq!(beside(&edge.join("msedge.exe")), said("154.0.4258.32"));

        // Chromium inside a macOS bundle: the framework's Versions.
        let chrome = root.path().join("Chrome.app").join("Contents");
        std::fs::create_dir_all(chrome.join("MacOS")).unwrap();
        std::fs::create_dir_all(
            chrome
                .join("Frameworks")
                .join("Chrome.framework")
                .join("Versions")
                .join("154.0.4258.32"),
        )
        .unwrap();
        assert_eq!(
            beside(&chrome.join("MacOS/Google Chrome")),
            said("154.0.4258.32")
        );

        // Safari: the bundle's own property list is the only one of the
        // three readings it has.
        let safari = root.path().join("Safari.app").join("Contents");
        std::fs::create_dir_all(safari.join("MacOS")).unwrap();
        std::fs::write(
            safari.join("Info.plist"),
            "<plist><dict>\n<key>CFBundleVersion</key><string>19618.1.15.11.14</string>\n\
             <key>CFBundleShortVersionString</key><string>17.4</string>\n</dict></plist>\n",
        )
        .unwrap();
        assert_eq!(beside(&safari.join("MacOS/Safari")), said("17.4"));

        // A browser whose installer wrote none of the three: present,
        // and this machine cannot say which version it is.
        let bare = root.path().join("bin");
        std::fs::create_dir_all(&bare).unwrap();
        assert_eq!(beside(&bare.join("brave")), Version::Silent);
    }
}
