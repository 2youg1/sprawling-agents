// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::ffi::OsString;

use super::roster::OFFICIAL;

const PASSED: [&str; 48] = [
    "PATH",
    "PATHEXT",
    "HOME",
    "USER",
    "USERNAME",
    "LOGNAME",
    "SHELL",
    "LANG",
    "LANGUAGE",
    "LC_ALL",
    "LC_CTYPE",
    "TZ",
    "TERM",
    "COLORTERM",
    "TMPDIR",
    "TEMP",
    "TMP",
    "USERPROFILE",
    "HOMEDRIVE",
    "HOMEPATH",
    "APPDATA",
    "LOCALAPPDATA",
    "PROGRAMDATA",
    "PROGRAMFILES",
    "PROGRAMFILES(X86)",
    "COMMONPROGRAMFILES",
    "SYSTEMROOT",
    "SYSTEMDRIVE",
    "WINDIR",
    "COMSPEC",
    "OS",
    "PROCESSOR_ARCHITECTURE",
    "NUMBER_OF_PROCESSORS",
    "XDG_CONFIG_HOME",
    "XDG_DATA_HOME",
    "XDG_CACHE_HOME",
    "XDG_STATE_HOME",
    "XDG_RUNTIME_DIR",
    "DISPLAY",
    "WAYLAND_DISPLAY",
    "HTTP_PROXY",
    "HTTPS_PROXY",
    "NO_PROXY",
    "SSH_CONNECTION",
    "SSH_CLIENT",
    "SSH_TTY",
    "NO_BROWSER",
    "NODE_EXTRA_CA_CERTS",
];

pub fn passed(city: impl Iterator<Item = (OsString, OsString)>) -> Vec<(OsString, OsString)> {
    let vendor = OFFICIAL
        .iter()
        .flat_map(|official| official.set_up)
        .filter_map(|dir| dir.variable);
    let allowed: Vec<&str> = PASSED.iter().copied().chain(vendor).collect();
    city.filter(|(name, _)| {
        name.to_str().is_some_and(|name| {
            allowed.iter().any(|allowed| {
                if cfg!(windows) || allowed.ends_with("_PROXY") {
                    allowed.eq_ignore_ascii_case(name)
                } else {
                    *allowed == name
                }
            })
        })
    })
    .collect()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::*;

    #[test]
    fn only_the_whitelist_and_vendor_homes_pass() {
        let city = [
            ("PATH", "/bin"),
            ("SPRAWLING_SECRET_OPENAI_KEY", "sk"),
            ("ANTHROPIC_API_KEY", "key"),
            ("SSH_CONNECTION", "1 2 3 4"),
            ("NO_BROWSER", "1"),
            ("CODEX_HOME", "/c"),
            ("https_proxy", "http://p"),
        ]
        .map(|(name, value)| (OsString::from(name), OsString::from(value)));
        let names: Vec<OsString> = passed(city.into_iter())
            .into_iter()
            .map(|(name, _)| name)
            .collect();
        assert_eq!(
            names,
            [
                "PATH",
                "SSH_CONNECTION",
                "NO_BROWSER",
                "CODEX_HOME",
                "https_proxy"
            ]
            .map(OsString::from)
        );
    }
}
