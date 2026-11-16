// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::{Path, PathBuf};

use super::Version;

pub(super) fn beside(program: &Path) -> Version {
    let Some(dir) = program.parent() else {
        return Version::Silent;
    };
    application_ini(dir)
        .or_else(|| version_directory(dir))
        .or_else(|| bundle_plist(dir))
        .map_or(Version::Silent, Version::Said)
}

const MOST_BYTES: u64 = 64 * 1024;

fn application_ini(dir: &Path) -> Option<String> {
    let beside = dir.join("application.ini");
    let in_bundle = dir.parent().map(|up| up.join("Resources/application.ini"));
    [Some(beside), in_bundle]
        .into_iter()
        .flatten()
        .find_map(|path| small_text(&path))
        .and_then(|text| app_version(&text))
}

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

fn bundle_plist(dir: &Path) -> Option<String> {
    let text = small_text(&dir.parent()?.join("Info.plist"))?;
    let (_, after) = text.split_once("<key>CFBundleShortVersionString</key>")?;
    let (_, value) = after.split_once("<string>")?;
    let (version, _) = value.split_once("</string>")?;
    let version = version.trim();
    (!version.is_empty()).then(|| version.to_owned())
}

fn entries(at: &Path) -> Vec<PathBuf> {
    let Ok(listing) = std::fs::read_dir(at) else {
        return Vec::new();
    };
    listing
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .collect()
}

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

    #[test]
    fn a_browser_says_its_version_through_the_files_beside_it() {
        let root = tempfile::tempdir().unwrap();
        let said = |text: &str| Version::Said(text.to_owned());

        let zen = root.path().join("Zen Browser");
        std::fs::create_dir_all(&zen).unwrap();
        std::fs::write(
            zen.join("application.ini"),
            "; a comment the installer leaves\n[App]\nVendor=Mozilla\nName=Zen\n\
             Version=1.22.2b\n\n[Gecko]\nMinVersion=156.0\n",
        )
        .unwrap();
        assert_eq!(beside(&zen.join("zen.exe")), said("1.22.2b"));

        let firefox = root.path().join("Firefox.app").join("Contents");
        std::fs::create_dir_all(firefox.join("MacOS")).unwrap();
        std::fs::create_dir_all(firefox.join("Resources")).unwrap();
        std::fs::write(
            firefox.join("Resources").join("application.ini"),
            "[App]\nName=Firefox\nVersion=133.0.3\n",
        )
        .unwrap();
        assert_eq!(beside(&firefox.join("MacOS/firefox")), said("133.0.3"));

        let edge = root.path().join("Edge/Application");
        std::fs::create_dir_all(edge.join("154.0.4258.9")).unwrap();
        std::fs::create_dir_all(edge.join("154.0.4258.32")).unwrap();
        assert_eq!(beside(&edge.join("msedge.exe")), said("154.0.4258.32"));

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

        let safari = root.path().join("Safari.app").join("Contents");
        std::fs::create_dir_all(safari.join("MacOS")).unwrap();
        std::fs::write(
            safari.join("Info.plist"),
            "<plist><dict>\n<key>CFBundleVersion</key><string>19618.1.15.11.14</string>\n\
             <key>CFBundleShortVersionString</key><string>17.4</string>\n</dict></plist>\n",
        )
        .unwrap();
        assert_eq!(beside(&safari.join("MacOS/Safari")), said("17.4"));

        let bare = root.path().join("bin");
        std::fs::create_dir_all(&bare).unwrap();
        assert_eq!(beside(&bare.join("brave")), Version::Silent);
    }
}
