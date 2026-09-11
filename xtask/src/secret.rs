// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Gate: no secret shape anywhere in the repository (C13). The judge is
//! `kernel::secret::scan` — one detector, one home; this gate only walks
//! and reports. Findings carry file + offset + length, never the bytes.
//! There is no inline waiver: a waiver comment would be a hole injected
//! content could drive through. Also enforces the `Sealed::expose` call
//! site whitelist ('s redemption points).
//!
//! **The subject is plaintext credentials reaching the tree**, not
//! high-entropy bytes existing somewhere. So the gate scans **authored**
//! files. A **derived** file - one whose bytes a tool computed from inputs
//! this same gate already scans - is not a place where a credential can
//! first arrive, and judging it re-judges its inputs through a lossy copy.
//! `is_derived` names the two classes the tree holds, each with the input
//! that is scanned in its place.
//!
//! Priced this way because `client/bun.lock` and two insta snapshots
//! produced 268 findings and zero credentials. The
//! alternative was a table of 268 byte offsets, which the next
//! `bun install` invalidates in full - a category question written down as
//! coordinates. The known limit is recorded in xtask-SPEC.md section 8-9:
//! a test that read a live credential from the environment and recorded it
//! into a snapshot would pass here, and the defect in that case is the
//! test.

use std::path::Path;

use crate::report::{Violation, XtaskError};
use crate::walk;

/// The only files allowed to say `.expose(` under crates/*/src: the
/// defining module and the two redemption points in gateway.
const EXPOSE_WHITELIST: [&str; 5] = [
    "crates/kernel/src/secret/sealed.rs",
    "crates/gateway/src/endpoint/call.rs",
    "crates/gateway/src/native.rs",
    // Renewing a subscription credential sends the refresh token
    // to the provider's token endpoint, which is a redemption point of
    // exactly the same kind as the two above - the last slot before the
    // wire. Widened here rather than worked around at the call site,
    // because the alternative was the assembly holding plaintext, and
    // that is the thing this list exists to prevent.
    "crates/gateway/src/credential/oauth/flow.rs",
    // An MCP server's configured header may name a credential
    // instead of carrying one, and the header is set on the request
    // being sent - the same last slot before the wire. Listed rather
    // than redeemed one layer up, for the reason the entry above
    // records: the alternative put plaintext in `bin::assembly`, which
    // is what this list exists to prevent.
    "crates/sprawling/src/mcp_http.rs",
];

/// Exact literals the detector flags that are not credentials.
///
/// The detector is right to flag them and stays unchanged: it exists to
/// capture anything key-shaped at the entrance, where a false positive
/// costs nothing. This gate answers a different question — "is a
/// credential committed here" — where a false positive costs a build,
/// and a vendor's own API identifier cannot be a credential.
///
/// Why a table here rather than a waiver comment at the site: a comment
/// is a hole that injected content could drive through, because content
/// can write comments. Content cannot write this table. Every entry is
/// an exact match, is reviewed, and lands with a `Verdict:` trailer,
/// since this path is inside the guard's protected face.
///
/// - `CC_x86_64_unknown_linux_musl` — Cargo's per-target C compiler
///   variable, set to `musl-gcc` by the `release.yml` musl job and
///   quoted in `sprawling-SPEC.md` where that job is argued. It is the
///   *name* of an environment variable and so can never itself hold a
///   value; the target triple's digits and underscores are what trip
///   the mixed-alphabet rule.
/// - The six `Win32_*` names — Cargo features of the `windows` crate,
///   listed in `desktop/Cargo.toml` to select the API surfaces the
///   Windows arm calls: data exchange for the clipboard, threading and
///   variant for COM, accessibility for the UI Automation tree, HiDPI
///   for per-monitor scaling, keyboard and mouse for `SendInput`, and
///   windows-and-messaging for enumeration. A Cargo feature name is
///   read by the resolver and can never hold a value; the digits in
///   `Win32` beside the underscores are what trip the mixed-alphabet
///   rule. Only the six names of twenty bytes or more are listed, since
///   a shorter one never reaches the entropy detector.
const NOT_CREDENTIALS: [&str; 7] = [
    "CC_x86_64_unknown_linux_musl",
    "Win32_System_DataExchange",
    "Win32_System_Threading",
    "Win32_System_Variant",
    "Win32_UI_Accessibility",
    "Win32_UI_Input_KeyboardAndMouse",
    "Win32_UI_WindowsAndMessaging",
];

/// Whether these exact bytes are one of the reviewed identifiers.
///
/// Exact and whole: a token that merely starts with or contains a
/// reviewed name is still a finding, so nothing can be smuggled past
/// this by wearing one as a prefix.
fn is_reviewed_identifier(found: &[u8]) -> bool {
    NOT_CREDENTIALS
        .iter()
        .any(|known| known.as_bytes() == found)
}

/// Lockfiles: a package manager writes every byte from a manifest and a
/// registry, and the `sha512-`/`sha256-` runs in them are integrity
/// digests of published artifacts, meant for anybody to read. A hand
/// edit here does not survive the next resolution.
const LOCKFILES: [&str; 2] = ["Cargo.lock", "bun.lock"];

/// Whether a tool wrote this file rather than a person.
///
/// Two classes, and each is scanned at its input instead: a resolved
/// lockfile, and an insta snapshot - which records what a test produced
/// from inputs that live in a source file this gate scans.
fn is_derived(rel: &str) -> bool {
    let name = rel.rsplit('/').next().unwrap_or(rel);
    LOCKFILES.contains(&name) || (rel.contains("/snapshots/") && rel.ends_with(".snap"))
}

pub(crate) fn check(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let mut violations = Vec::new();
    for path in walk::files(root)? {
        let rel = walk::rel(root, &path);
        if walk::in_isolation_zone(&rel) {
            continue;
        }
        let bytes = std::fs::read(&path).map_err(|source| XtaskError::Io {
            path: rel.clone(),
            source,
        })?;
        // Neither class carries a `.rs` name, so the expose whitelist
        // below loses nothing by this skip.
        if is_derived(&rel) {
            continue;
        }
        for span in kernel::scan(&bytes) {
            let end = span.start.saturating_add(span.len);
            if bytes
                .get(span.start..end)
                .is_some_and(is_reviewed_identifier)
            {
                continue;
            }
            let what = span
                .provider
                .map_or("high-entropy token".to_owned(), |p| format!("{p} shape"));
            violations.push(Violation {
                gate: "secret",
                location: format!("{rel}:byte {}", span.start),
                rule: "no secret shape may live in the repository (C13)".to_owned(),
                violation: format!("{what}, {} bytes", span.len),
                alternative: "replace the value with a secret:<realm>/<name> reference; \
                              if it is a scanner self-test sample, assemble it at runtime \
                              from short fragments"
                    .to_owned(),
            });
        }
        if rel.starts_with("crates/")
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

    /// Assembled at runtime from short pieces: a literal key-shaped
    /// sample in the source would be a finding about this file.
    fn key_shaped() -> String {
        ["sk", "9fQ2xZ", "7Lm4Rt", "0Bv8Kd", "3Wp6"].join("")
    }

    #[test]
    fn a_reviewed_identifier_is_matched_whole_and_nothing_else_is() {
        // Derived from the table rather than spelled out: a long literal
        // here would be a finding about this file, which is the same
        // rule this gate applies to everyone else.
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

    /// A derived file is not where a credential first reaches the tree.
    ///
    /// The lockfile and the snapshot below carry the same key-shaped run
    /// as the source file; only the source file is authored, so only it
    /// is reported.
    #[test]
    fn a_derived_file_is_judged_by_the_inputs_that_produced_it() {
        let root = std::env::temp_dir().join(format!("secret-derived-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let body = key_shaped();
        for rel in [
            "client/bun.lock",
            "crates/gateway/src/dialect/snapshots/pinned.snap",
            "crates/gateway/src/dialect/request.rs",
        ] {
            let path = root.join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, &body).unwrap();
        }

        let found = check(&root).unwrap();
        let places: Vec<&str> = found.iter().map(|v| v.location.as_str()).collect();
        assert_eq!(
            places,
            ["crates/gateway/src/dialect/request.rs:byte 0"],
            "only the authored file is a place a credential can enter"
        );
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn the_detector_still_finds_a_key_shaped_token() {
        let sample = key_shaped();
        let spans = kernel::scan(sample.as_bytes());
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
