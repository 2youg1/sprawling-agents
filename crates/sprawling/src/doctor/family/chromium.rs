// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The Chromium browsers, and where each platform puts them
//! (sprawling-SPEC.md section 8-57).
//!
//! Data, with no branch in it. None of these is a way into a session on
//! its own: the driver beside them is, and it has its own row, so this
//! file answers only the question "which Chromium is on this machine".

use super::{Confidence, Member};
use crate::doctor::PerPlatform;

/// Chrome first, because its driver is the one with published builds
/// for every version.
pub(super) const MEMBERS: &[Member] = &[
    Member {
        name: "chrome",
        program: "google-chrome",
        homepage: "https://www.google.com/chrome/",
        confidence: Confidence::Tried,
        places: PerPlatform {
            windows: &[
                r"C:\Program Files\Google\Chrome\Application\chrome.exe",
                r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
            ],
            macos: &["/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"],
            linux: &["/usr/bin/google-chrome", "/opt/google/chrome/chrome"],
        },
        start_menu: "Google Chrome",
    },
    Member {
        name: "edge",
        program: "msedge",
        homepage: "https://www.microsoft.com/edge",
        confidence: Confidence::Tried,
        places: PerPlatform {
            windows: &[
                r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
                r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
            ],
            macos: &["/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge"],
            linux: &["/usr/bin/microsoft-edge", "/opt/microsoft/msedge/msedge"],
        },
        start_menu: "Microsoft Edge",
    },
    Member {
        name: "brave",
        program: "brave-browser",
        homepage: "https://brave.com/",
        confidence: Confidence::Tried,
        places: PerPlatform {
            windows: &[r"C:\Program Files\BraveSoftware\Brave-Browser\Application\brave.exe"],
            macos: &["/Applications/Brave Browser.app/Contents/MacOS/Brave Browser"],
            linux: &["/usr/bin/brave-browser", "/opt/brave.com/brave/brave"],
        },
        start_menu: "Brave",
    },
    Member {
        name: "chromium",
        program: "chromium",
        homepage: "https://www.chromium.org/",
        confidence: Confidence::Tried,
        places: PerPlatform {
            windows: &[r"C:\Program Files\Chromium\Application\chrome.exe"],
            macos: &["/Applications/Chromium.app/Contents/MacOS/Chromium"],
            linux: &[
                "/usr/bin/chromium",
                "/usr/bin/chromium-browser",
                "/snap/bin/chromium",
            ],
        },
        start_menu: "Chromium",
    },
    Member {
        name: "vivaldi",
        program: "vivaldi",
        homepage: "https://vivaldi.com/",
        confidence: Confidence::Tried,
        places: PerPlatform {
            windows: &[r"C:\Program Files\Vivaldi\Application\vivaldi.exe"],
            macos: &["/Applications/Vivaldi.app/Contents/MacOS/Vivaldi"],
            linux: &["/usr/bin/vivaldi", "/opt/vivaldi/vivaldi"],
        },
        start_menu: "Vivaldi",
    },
];
