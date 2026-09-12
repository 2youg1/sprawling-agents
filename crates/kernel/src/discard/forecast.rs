// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Discard forecasts: reading a command whole before it runs.

use crate::tool::ExecArm;

/// Suspicious-command snippets for the text arms (data face, pub(crate)).
/// Text prediction is obfuscatable by design — hits route conservatively,
/// and the git checkpoint net (S3) is the honest backstop.
pub(crate) const SUSPECT_SNIPPETS_TEXT: [&str; 5] =
    ["rm ", "rmdir", "-delete", "git reset --hard", "git clean"];

pub(crate) const SUSPECT_SNIPPETS_PYTHON: [&str; 3] = ["os.remove", "shutil.rmtree", "os.unlink"];

/// Deliberately exhaustive forecast.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscardForecast {
    Clear,
    Suspected { pattern: String },
}

fn program_basename(path: &str) -> &str {
    let name = path.rsplit(['/', '\\']).next().unwrap_or(path);
    name.strip_suffix(".exe").unwrap_or(name)
}

/// The pre-judgment (7.2): Program reads `(path, args)` whole — a
/// whitelisted name with poisoned args is no trust at all; Python and
/// Shell get substring conservatism.
pub fn forecast(arm: &ExecArm) -> DiscardForecast {
    match arm {
        ExecArm::Program { path, args } => {
            let base = program_basename(path);
            if matches!(base, "rm" | "rmdir" | "del") {
                return DiscardForecast::Suspected {
                    pattern: base.to_owned(),
                };
            }
            let joined = args.join(" ");
            if base == "git" && (joined.contains("reset --hard") || joined.contains("clean")) {
                return DiscardForecast::Suspected {
                    pattern: format!("git {joined}"),
                };
            }
            if base == "find" && args.iter().any(|a| a == "-delete") {
                return DiscardForecast::Suspected {
                    pattern: "find -delete".to_owned(),
                };
            }
            DiscardForecast::Clear
        }
        ExecArm::Python { code } => {
            for pattern in SUSPECT_SNIPPETS_TEXT
                .iter()
                .chain(SUSPECT_SNIPPETS_PYTHON.iter())
            {
                if code.contains(pattern) {
                    return DiscardForecast::Suspected {
                        pattern: (*pattern).to_owned(),
                    };
                }
            }
            if code.contains("open(") && (code.contains("'w'") || code.contains("\"w\"")) {
                return DiscardForecast::Suspected {
                    pattern: "open(..., 'w')".to_owned(),
                };
            }
            DiscardForecast::Clear
        }
        ExecArm::Shell { text } => {
            for pattern in SUSPECT_SNIPPETS_TEXT.iter() {
                if text.contains(pattern) {
                    return DiscardForecast::Suspected {
                        pattern: (*pattern).to_owned(),
                    };
                }
            }
            if text.contains('>') && !text.contains(">>") {
                return DiscardForecast::Suspected {
                    pattern: "> truncation".to_owned(),
                };
            }
            DiscardForecast::Clear
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::super::request::DiscardRequest;
    use super::super::request::{Discard, Restoration};
    use super::super::verdict::{DiscardVerdict, decide};
    use super::*;
    use crate::address::Address;
    use crate::budget::ByteLen;
    use crate::locator::Locator;
    use crate::registry::Registry;
    use crate::taint::TaintSet;
    use crate::taint::TaintSource;
    use proptest::prelude::*;
    fn addr(raw: &str) -> Address {
        Address::parse(raw).unwrap()
    }
    fn tracked() -> Restoration {
        Restoration::Tracked(Locator::parse(&format!("file:b/x.md@{}", "ab".repeat(20))).unwrap())
    }
    #[test]
    fn program_forecast_reads_path_and_args_whole() {
        let rm = ExecArm::Program {
            path: "/usr/bin/rm".into(),
            args: vec!["-rf".into(), "docs".into()],
        };
        assert!(matches!(forecast(&rm), DiscardForecast::Suspected { .. }));
        let git_ok = ExecArm::Program {
            path: "git".into(),
            args: vec!["status".into()],
        };
        assert_eq!(forecast(&git_ok), DiscardForecast::Clear);
        let git_hard = ExecArm::Program {
            path: "git".into(),
            args: vec!["reset".into(), "--hard".into()],
        };
        assert!(matches!(
            forecast(&git_hard),
            DiscardForecast::Suspected { .. }
        ));
        let find_delete = ExecArm::Program {
            path: "find.exe".into(),
            args: vec![".".into(), "-delete".into()],
        };
        assert!(matches!(
            forecast(&find_delete),
            DiscardForecast::Suspected { .. }
        ));
    }

    #[test]
    fn text_arms_are_conservative() {
        let py = ExecArm::Python {
            code: "import shutil\nshutil.rmtree('build')".into(),
        };
        assert!(matches!(forecast(&py), DiscardForecast::Suspected { .. }));
        let py_write = ExecArm::Python {
            code: "open('x.txt', 'w').write('hi')".into(),
        };
        assert!(matches!(
            forecast(&py_write),
            DiscardForecast::Suspected { .. }
        ));
        let py_read = ExecArm::Python {
            code: "print(open('x.txt').read())".into(),
        };
        assert_eq!(forecast(&py_read), DiscardForecast::Clear);
        let sh_trunc = ExecArm::Shell {
            text: "echo hi > file.txt".into(),
        };
        assert!(matches!(
            forecast(&sh_trunc),
            DiscardForecast::Suspected { .. }
        ));
        let sh_append = ExecArm::Shell {
            text: "echo hi >> log.txt".into(),
        };
        assert_eq!(forecast(&sh_append), DiscardForecast::Clear);
    }

    proptest! {
        /// Kani mirror: Allow implies planned, clean-handed, in-scale.
        #[test]
        fn allow_implies_every_guard_passed(files in 1usize..24, bytes in any::<u64>(),
                                            tainted in any::<bool>()) {
            let registry = Registry::new();
            let paths: Vec<Address> = (0..files).map(|i| addr(&format!("b/f{i}"))).collect();
            let taint = if tainted {
                TaintSet::of(TaintSource::new("web:x").unwrap())
            } else {
                TaintSet::empty()
            };
            let discard = Discard::new(paths, tracked(), taint, ByteLen::new(bytes)).unwrap();
            let verdict = decide(&DiscardRequest::Planned(discard), &registry);
            if verdict == DiscardVerdict::Allow {
                prop_assert!(!tainted);
                prop_assert!(files <= 16);
                prop_assert!(bytes <= 1_048_576);
            }
        }
    }
}
