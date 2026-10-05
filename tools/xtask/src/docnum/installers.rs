// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Standalone repository defaults generated from workspace metadata
//! (tools/xtask/Spec.lean §8-16 D30).

use std::path::Path;

use crate::report::{Violation, XtaskError};
use crate::{package, walk};

const SHELL_FILE: &str = "install.sh";
const POWERSHELL_FILE: &str = "install.ps1";

struct DefaultLine {
    file: &'static str,
    prefix: &'static str,
    expected: String,
}

pub(super) fn check(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let mut violations = Vec::new();
    for default in defaults(root)? {
        let text = walk::read_text(&root.join(default.file))?;
        let lines: Vec<_> = text
            .lines()
            .filter(|line| line.starts_with(default.prefix))
            .collect();
        if lines != [default.expected.as_str()] {
            violations.push(Violation {
                gate: "docnum",
                location: default.file.to_owned(),
                rule: "standalone repository defaults derive from [workspace.package] repository"
                    .to_owned(),
                violation: "the repository assignment is missing, repeated or stale".to_owned(),
                alternative:
                    "run cargo xtask docnum --write; keep SPRAWLING_REPO as the explicit override"
                        .to_owned(),
            });
        }
    }
    Ok(violations)
}

pub(super) fn write(root: &Path) -> Result<usize, XtaskError> {
    let mut changed = 0_usize;
    for default in defaults(root)? {
        let path = root.join(default.file);
        let text = walk::read_text(&path)?;
        if text
            .lines()
            .filter(|line| line.starts_with(default.prefix))
            .count()
            != 1
        {
            return Err(XtaskError::Doc {
                file: default.file.to_owned(),
                msg: "restore exactly one repository assignment before regeneration".to_owned(),
            });
        }
        let rewritten: String = text
            .split_inclusive('\n')
            .map(|line| {
                if line.starts_with(default.prefix) {
                    format!("{}\n", default.expected)
                } else {
                    line.to_owned()
                }
            })
            .collect();
        if rewritten != text {
            std::fs::write(&path, rewritten).map_err(|source| XtaskError::Io {
                path: default.file.to_owned(),
                source,
            })?;
            changed = changed.saturating_add(1);
            println!("written: {}", default.file);
        }
    }
    Ok(changed)
}

fn defaults(root: &Path) -> Result<Vec<DefaultLine>, XtaskError> {
    let shell = root
        .join(SHELL_FILE)
        .try_exists()
        .map_err(|source| XtaskError::Io {
            path: SHELL_FILE.to_owned(),
            source,
        })?;
    let powershell = root
        .join(POWERSHELL_FILE)
        .try_exists()
        .map_err(|source| XtaskError::Io {
            path: POWERSHELL_FILE.to_owned(),
            source,
        })?;
    if !shell && !powershell {
        return Ok(Vec::new());
    }
    let repository = package::workspace_package(root, "repository")?;
    let identity = repository.strip_prefix("https://github.com/").filter(|identity| {
        let parts: Vec<_> = identity.split('/').collect();
        parts.len() == 2 && parts.iter().all(|part| {
            !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
        })
    }).ok_or_else(|| XtaskError::Doc {
        file: "Cargo.toml".to_owned(),
        msg: "workspace repository must name a GitHub owner/repo without shell syntax; correct the repository before generating standalone defaults".to_owned(),
    })?;
    Ok(vec![
        DefaultLine {
            file: SHELL_FILE,
            prefix: "REPO=",
            expected: format!("REPO=\"${{SPRAWLING_REPO:-{identity}}}\""),
        },
        DefaultLine {
            file: POWERSHELL_FILE,
            prefix: "$repo =",
            expected: format!(
                "$repo = if ($env:SPRAWLING_REPO) {{ $env:SPRAWLING_REPO }} else {{ '{identity}' }}"
            ),
        },
    ])
}
