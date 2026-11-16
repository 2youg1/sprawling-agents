// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use crate::report::{Violation, XtaskError};

mod emit;

const TARGET: &str = concat!(crate::walk::client_src!(), "/wire.ts");

pub(super) struct Constants {
    pub(super) wire_v: u32,
    pub(super) hash: String,

    pub(super) city_run: String,

    pub(super) body_px_min: u32,

    pub(super) line_height_min: u32,
    pub(super) tracking_min: i32,

    pub(super) default_effort: &'static str,

    pub(super) slash: Vec<[&'static str; 4]>,
}

fn render() -> Result<String, XtaskError> {
    let constants = Constants {
        wire_v: wire::WIRE_V,
        hash: wire::schema_hash().to_string(),
        city_run: kernel::RunId::CITY.to_string(),
        body_px_min: wire::BODY_PX_MIN,
        line_height_min: wire::LINE_HEIGHT_MIN,
        tracking_min: wire::TRACKING_MIN,
        default_effort: kernel::consts_policy::DEFAULT_EFFORT.as_str(),
        slash: wire::Slash::ALL
            .into_iter()
            .map(|verb| {
                [
                    verb.spelling(),
                    verb.takes(),
                    verb.offered().as_str(),
                    verb.says(),
                ]
            })
            .collect(),
    };
    emit::emit(&wire::wire_schema(), &constants).map_err(|refused| XtaskError::Doc {
        file: format!("wire::wire_schema at {}", refused.at),
        msg: refused.why,
    })
}

pub(crate) fn check(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let expected = render()?;
    let path = root.join(TARGET);
    let actual = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(source) => {
            return Err(XtaskError::Io {
                path: path.display().to_string(),
                source,
            });
        }
    };
    let Some((line, wanted, found)) = first_difference(&expected, &actual) else {
        return Ok(Vec::new());
    };
    Ok(vec![Violation {
        gate: "wire-ts",
        location: format!("{TARGET}:{line}"),
        rule: "the client's wire types are the ones the Rust wire generates".to_owned(),
        violation: if actual.is_empty() {
            "the file is absent".to_owned()
        } else {
            format!("line {line} reads `{found}` where the wire says `{wanted}`")
        },
        alternative: "run `cargo xtask wire-ts --write` and commit the result".to_owned(),
    }])
}

pub(crate) fn write(root: &Path) -> Result<String, XtaskError> {
    let text = render()?;
    let path = root.join(TARGET);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|source| XtaskError::Io {
            path: dir.display().to_string(),
            source,
        })?;
    }
    std::fs::write(&path, text).map_err(|source| XtaskError::Io {
        path: path.display().to_string(),
        source,
    })?;
    Ok(format!("written: {TARGET}\n"))
}

fn first_difference(expected: &str, actual: &str) -> Option<(usize, String, String)> {
    let mut wanted = expected.lines();
    let mut found = actual.lines();
    let mut line = 0_usize;
    loop {
        line = line.saturating_add(1);
        match (wanted.next(), found.next()) {
            (None, None) => return None,
            (w, f) if w == f => {}
            (w, f) => {
                return Some((
                    line,
                    w.unwrap_or_default().to_owned(),
                    f.unwrap_or_default().to_owned(),
                ));
            }
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    reason = "test code"
)]
mod tests;
