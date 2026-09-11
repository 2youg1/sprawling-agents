// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The Gecko browsers, and where each platform puts them
//! (sprawling-SPEC.md section 8-57).
//!
//! Data, with no branch in it. Every member here takes the same four
//! launch arguments, which is why they are one row: the engine starts
//! them the same way and the session that follows is the same session.

use super::{Confidence, Member};
use crate::doctor::PerPlatform;

/// Firefox and its forks, ordinary ones first.
pub(super) const MEMBERS: &[Member] = &[
    Member {
        name: "firefox",
        program: "firefox",
        homepage: "https://www.mozilla.org/firefox/",
        confidence: Confidence::Tried,
        places: PerPlatform {
            windows: &[
                r"C:\Program Files\Mozilla Firefox\firefox.exe",
                r"C:\Program Files (x86)\Mozilla Firefox\firefox.exe",
            ],
            macos: &["/Applications/Firefox.app/Contents/MacOS/firefox"],
            linux: &[
                "/usr/bin/firefox",
                "/snap/bin/firefox",
                "/usr/lib/firefox/firefox",
            ],
        },
        start_menu: "FIREFOX.EXE",
    },
    Member {
        name: "zen",
        program: "zen",
        homepage: "https://zen-browser.app/",
        confidence: Confidence::Tried,
        places: PerPlatform {
            windows: &[r"C:\Program Files\Zen Browser\zen.exe"],
            macos: &["/Applications/Zen.app/Contents/MacOS/zen"],
            linux: &["/usr/bin/zen", "/opt/zen/zen", "/usr/lib/zen/zen"],
        },
        start_menu: "Zen Browser",
    },
    Member {
        name: "librewolf",
        program: "librewolf",
        homepage: "https://librewolf.net/",
        confidence: Confidence::Tried,
        places: PerPlatform {
            windows: &[r"C:\Program Files\LibreWolf\librewolf.exe"],
            macos: &["/Applications/LibreWolf.app/Contents/MacOS/librewolf"],
            linux: &["/usr/bin/librewolf", "/snap/bin/librewolf"],
        },
        start_menu: "LibreWolf",
    },
    Member {
        name: "waterfox",
        program: "waterfox",
        homepage: "https://www.waterfox.net/",
        confidence: Confidence::Tried,
        places: PerPlatform {
            windows: &[r"C:\Program Files\Waterfox\waterfox.exe"],
            macos: &["/Applications/Waterfox.app/Contents/MacOS/waterfox"],
            linux: &["/usr/bin/waterfox", "/opt/waterfox/waterfox"],
        },
        start_menu: "Waterfox",
    },
    Member {
        name: "floorp",
        program: "floorp",
        homepage: "https://floorp.app/",
        confidence: Confidence::Tried,
        places: PerPlatform {
            windows: &[r"C:\Program Files\Ablaze Floorp\floorp.exe"],
            macos: &["/Applications/Floorp.app/Contents/MacOS/floorp"],
            linux: &["/usr/bin/floorp", "/opt/floorp/floorp"],
        },
        start_menu: "Floorp",
    },
    Member {
        name: "firefox-developer",
        program: "firefox-developer-edition",
        homepage: "https://www.mozilla.org/firefox/developer/",
        confidence: Confidence::Tried,
        places: PerPlatform {
            windows: &[r"C:\Program Files\Firefox Developer Edition\firefox.exe"],
            macos: &["/Applications/Firefox Developer Edition.app/Contents/MacOS/firefox"],
            linux: &[
                "/usr/bin/firefox-developer-edition",
                "/opt/firefox-developer-edition/firefox",
            ],
        },
        start_menu: "",
    },
    Member {
        name: "firefox-nightly",
        program: "firefox-nightly",
        homepage: "https://www.mozilla.org/firefox/channel/desktop/#nightly",
        confidence: Confidence::Tried,
        places: PerPlatform {
            windows: &[r"C:\Program Files\Firefox Nightly\firefox.exe"],
            macos: &["/Applications/Firefox Nightly.app/Contents/MacOS/firefox"],
            linux: &["/usr/bin/firefox-nightly", "/opt/firefox-nightly/firefox"],
        },
        start_menu: "",
    },
    // Last on purpose: the launcher starts a second process and the
    // proxy it insists on is between this city and the session, so a
    // machine with any other Gecko browser answers with that one.
    Member {
        name: "tor-browser",
        program: "tor-browser",
        homepage: "https://www.torproject.org/",
        confidence: Confidence::NeedsConfirmation,
        places: PerPlatform {
            windows: &[r"C:\Program Files\Tor Browser\Browser\firefox.exe"],
            macos: &["/Applications/Tor Browser.app/Contents/MacOS/firefox"],
            linux: &[
                "/opt/tor-browser/Browser/firefox",
                "/usr/bin/torbrowser-launcher",
            ],
        },
        start_menu: "",
    },
];
