// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::Path;

use crate::report::{Violation, XtaskError};
use crate::walk;

mod nix;

#[cfg(test)]
#[allow(clippy::unwrap_used, reason = "test code")]
mod nix_tests;

const EXPOSE_WHITELIST: [&str; 5] = [
    "Sprawling/kernel/src/secret/sealed.rs",
    "Sprawling/gateway/src/endpoint/call.rs",
    "Sprawling/agent_protocols/src/mcp/redeeming.rs",
    "Sprawling/agent_protocols/src/mcp/broker.rs",
    "Sprawling/remote_access/src/keys.rs",
];

const NOT_CREDENTIALS: [&str; 8] = [
    "CC_x86_64_unknown_linux_musl",
    "io/badge/Ask-DeepWiki-2B6CB0",
    "Win32_System_DataExchange",
    "Win32_System_Threading",
    "Win32_System_Variant",
    "Win32_UI_Accessibility",
    "Win32_UI_Input_KeyboardAndMouse",
    "Win32_UI_WindowsAndMessaging",
];

fn is_reviewed_identifier(found: &[u8]) -> bool {
    NOT_CREDENTIALS
        .iter()
        .any(|known| known.as_bytes() == found)
}

fn is_pascal_case_identifier(found: &[u8]) -> bool {
    found.first().is_some_and(u8::is_ascii_uppercase)
        && !found.last().is_some_and(u8::is_ascii_uppercase)
        && found.iter().all(u8::is_ascii_alphanumeric)
        && found.iter().zip(found.iter().skip(1)).all(|(a, b)| {
            !(a.is_ascii_uppercase() && !b.is_ascii_lowercase())
                && !(a.is_ascii_digit() && !b.is_ascii_digit())
        })
}

const LOCKFILES: [&str; 2] = ["Cargo.lock", "bun.lock"];

fn is_derived(rel: &str) -> bool {
    let name = rel.rsplit('/').next().unwrap_or(rel);
    LOCKFILES.contains(&name) || (rel.contains("/snapshots/") && rel.ends_with(".snap"))
}

fn in_city_records(rel: &str) -> bool {
    rel == kernel::RESERVED_PREFIX
        || rel
            .strip_prefix(kernel::RESERVED_PREFIX)
            .is_some_and(|tail| tail.starts_with('/'))
}

const LEDGER_RULE: &str =
    "no credential in cleartext may reach a city's records, which every tool can read (C13)";
const LEDGER_RECOVERY: &str = "capture the value through gateway::credential::Custodian before \
     it is written, so the record carries a secret:<realm>/<name> reference; a record already \
     written is discarded with the run that wrote it";

pub(crate) fn check(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let mut violations = Vec::new();
    let product = crate::members::product(root)?;
    for path in walk::files(root)? {
        let rel = walk::rel(root, &path);
        if walk::in_isolation_zone(&rel) {
            continue;
        }
        let bytes = std::fs::read(&path).map_err(|source| XtaskError::Io {
            path: rel.clone(),
            source,
        })?;

        if is_derived(&rel) {
            continue;
        }
        let records = in_city_records(&rel);

        if records && std::str::from_utf8(&bytes).is_err() {
            continue;
        }
        let nar_hashes = nix::NarHashes::classify(&rel, &bytes);
        for span in kernel::secret::scan(&bytes) {
            if nar_hashes.admits(&span) {
                continue;
            }
            let end = span.start.saturating_add(span.len);
            if bytes
                .get(span.start..end)
                .is_some_and(|t| is_reviewed_identifier(t) || is_pascal_case_identifier(t))
            {
                continue;
            }
            let what = span
                .provider
                .map_or("high-entropy token".to_owned(), |p| format!("{p} shape"));
            let (rule, alternative) = if records {
                (LEDGER_RULE, LEDGER_RECOVERY)
            } else {
                (
                    "no secret shape may live in the repository (C13)",
                    "replace the value with a secret:<realm>/<name> reference; \
                     if it is a scanner self-test sample, assemble it at runtime \
                     from short fragments",
                )
            };
            violations.push(Violation {
                gate: "secret",
                location: format!("{rel}:byte {}", span.start),
                rule: rule.to_owned(),
                violation: format!("{what}, {} bytes", span.len),
                alternative: alternative.to_owned(),
            });
        }
        if product.iter().any(|member| member.holds(&rel))
            && rel.contains("/src/")
            && rel.ends_with(".rs")
            && !EXPOSE_WHITELIST.contains(&rel.as_str())
        {
            let text = String::from_utf8_lossy(&bytes);
            for (n, line) in text.lines().enumerate() {
                if line.contains(".expose(") {
                    violations.push(Violation {
                        gate: "secret",
                        location: format!("{rel}:{}", n.saturating_add(1)),
                        rule: "Sealed::expose call sites are whitelisted".to_owned(),
                        violation: "an expose call outside the redemption points".to_owned(),
                        alternative: "pass the Sealed value onward; only gateway::endpoint \
                                      and gateway::native unseal, in the last slot before \
                                      the wire"
                            .to_owned(),
                    });
                }
            }
        }
    }
    Ok(violations)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::string_slice,
    reason = "test code"
)]
mod tests {
    use super::*;

    fn key_shaped() -> String {
        ["sk", "9fQ2xZ", "7Lm4Rt", "0Bv8Kd", "3Wp6"].join("")
    }

    fn empty_workspace(root: &Path) {
        let _ = std::fs::remove_dir_all(root);
        crate::root::fixture::write(
            root,
            "Cargo.toml",
            "[workspace]
members = []
resolver = \"3\"
",
        );
    }

    #[test]
    fn a_reviewed_identifier_is_matched_whole_and_nothing_else_is() {
        let reviewed = NOT_CREDENTIALS[0];
        assert!(is_reviewed_identifier(reviewed.as_bytes()));
        let longer = format!("{reviewed}Extra");
        assert!(
            !is_reviewed_identifier(longer.as_bytes()),
            "a longer token that starts with a reviewed name is still a finding"
        );
        let inner = &reviewed[1..reviewed.len().saturating_sub(1)];
        assert!(
            !is_reviewed_identifier(inner.as_bytes()),
            "a piece of a reviewed name is not the reviewed name"
        );
        assert!(!is_reviewed_identifier(key_shaped().as_bytes()));
    }

    #[test]
    fn a_derived_file_is_judged_by_the_inputs_that_produced_it() {
        let root = std::env::temp_dir().join(format!("secret-derived-{}", std::process::id()));
        empty_workspace(&root);
        let body = key_shaped();
        for rel in [
            "client/bun.lock",
            "Sprawling/gateway/src/dialect/snapshots/pinned.snap",
            "Sprawling/gateway/src/dialect/request.rs",
        ] {
            let path = root.join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, &body).unwrap();
        }

        let found = check(&root).unwrap();
        let places: Vec<&str> = found.iter().map(|v| v.location.as_str()).collect();
        assert_eq!(
            places,
            ["Sprawling/gateway/src/dialect/request.rs:byte 0"],
            "only the authored file is a place a credential can enter"
        );
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_ledger_line_is_reported_against_the_records_rule_and_an_object_is_not() {
        let root = std::env::temp_dir().join(format!("secret-records-{}", std::process::id()));
        empty_workspace(&root);
        let body = key_shaped();
        let ledger = root.join(kernel::RESERVED_PREFIX).join("ledger.jsonl");
        std::fs::create_dir_all(ledger.parent().unwrap()).unwrap();
        std::fs::write(&ledger, format!("{{\"value\":\"{body}\"}}")).unwrap();
        let object = root.join(kernel::RESERVED_PREFIX).join("cas").join("ab12");
        std::fs::create_dir_all(object.parent().unwrap()).unwrap();
        let mut bytes = vec![0xff_u8, 0xfe];
        bytes.extend_from_slice(body.as_bytes());
        std::fs::write(&object, bytes).unwrap();

        let found = check(&root).unwrap();
        let rules: Vec<&str> = found.iter().map(|v| v.rule.as_str()).collect();
        assert_eq!(
            rules,
            [LEDGER_RULE],
            "the ledger line is the one finding, and it names the records rule"
        );
        assert!(found[0].location.starts_with(kernel::RESERVED_PREFIX));
        assert!(found[0].alternative.contains("Custodian"));
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_pascal_case_identifier_is_not_reported_and_a_key_still_is() {
        let root = std::env::temp_dir().join(format!("secret-pascal-{}", std::process::id()));
        empty_workspace(&root);
        let provider = ["sk-", "ant-", &"a1B2c3D4e5".repeat(9)].concat();
        for (rel, body) in [
            ("src/mem.rs", "\"PeakPagedMemorySize64\"".to_owned()),
            ("src/key.rs", format!("\"{}\"", key_shaped())),
            ("src/sk.rs", format!("\"{provider}\"")),
        ] {
            let path = root.join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, body).unwrap();
        }

        let found = check(&root).unwrap();
        let places: Vec<&str> = found.iter().map(|v| v.location.as_str()).collect();
        assert_eq!(places, ["src/key.rs:byte 1", "src/sk.rs:byte 1"]);
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn a_shields_badge_url_is_not_reported_and_a_key_still_is() {
        let root = std::env::temp_dir().join(format!("secret-badge-{}", std::process::id()));
        empty_workspace(&root);
        let badge = ["badge/Ask-Deep", "Wiki-2B6CB0"].concat();
        let body = format!("src=\"https://img.shields.io/{badge}?labelColor=171717\"");
        let path = root.join("README.md");
        std::fs::write(&path, &body).unwrap();

        let found = check(&root).unwrap();
        assert!(
            found.is_empty(),
            "a badge URL is not a place a credential can enter: {found:?}"
        );
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn the_detector_still_finds_a_key_shaped_token() {
        let sample = key_shaped();
        let spans = kernel::secret::scan(sample.as_bytes());
        assert!(
            !spans.is_empty(),
            "the allowlist must not have loosened what the detector looks for"
        );
        for span in spans {
            let end = span.start.saturating_add(span.len);
            assert!(
                !sample
                    .as_bytes()
                    .get(span.start..end)
                    .is_some_and(is_reviewed_identifier),
                "a key-shaped token is never a reviewed identifier"
            );
        }
    }
}
