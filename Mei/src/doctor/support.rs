// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Python 3 support: the official end-of-life instants from the
//! `python-releases.toml` file under `release_management/` in `python/peps`,
//! mirrored by `Mei/spec/Environment.lean` and replayed from its vectors.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
}

impl Version {
    pub fn parse(text: &str) -> Option<Version> {
        let mut parts = text.split('.');
        let (Some(major), Some(minor)) = (parts.next(), parts.next()) else {
            return None;
        };
        let (Ok(major), Ok(minor)) = (major.parse::<u32>(), minor.parse::<u32>()) else {
            return None;
        };
        if !parts.all(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit())) {
            return None;
        }
        Some(Version { major, minor })
    }
}

pub fn python_supported(version: &str, now: u64) -> bool {
    let Some(version) = Version::parse(version) else {
        return false;
    };
    if version.major != 3 {
        return false;
    }
    match end_of_life(version.minor) {
        Some(instant) => now < instant,
        None => true,
    }
}

pub fn end_of_life(minor: u32) -> Option<u64> {
    Some(match minor {
        0 => 1_246_060_800,
        1 => 1_333_929_600,
        2 => 1_455_926_400,
        3 => 1_506_643_200,
        4 => 1_552_867_200,
        5 => 1_601_424_000,
        6 => 1_640_217_600,
        7 => 1_687_824_000,
        8 => 1_728_259_200,
        9 => 1_761_868_800,
        10 => 1_790_812_800,
        11 => 1_822_348_800,
        12 => 1_853_971_200,
        13 => 1_885_507_200,
        14 => 1_917_043_200,
        15 => 1_948_579_200,
        16 => 1_980_201_600,
        _ => return None,
    })
}
