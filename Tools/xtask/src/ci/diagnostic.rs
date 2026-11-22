// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

pub(super) struct Diagnostic<'a> {
    pub(super) code: &'a str,
    pub(super) action: &'a str,
    pub(super) subject: &'a str,
    pub(super) path: &'a Path,
    pub(super) value: &'a str,
    pub(super) rule: &'a str,
    pub(super) repair: &'a str,
}

impl Diagnostic<'_> {
    pub(super) fn emit(&self) {
        let path = self.path.display().to_string();
        let fields = [
            ("code", self.code),
            ("action", self.action),
            ("subject", self.subject),
            ("path", &path),
            ("value", self.value),
            ("rule", self.rule),
            ("repair", self.repair),
        ];
        let mut block = String::from("```toml\n");
        for (name, value) in fields {
            use std::fmt::Write as _;
            if writeln!(block, "{name} = {}", toml::Value::String(value.to_owned())).is_err() {
                eprintln!("could not format verification diagnostic {}", self.code);
                return;
            }
        }
        eprintln!("{block}```");
    }
}
