// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The facts the out-of-tree package quotes from inside the wall.
//!
//! The manifest is not the only copy `desktop/` keeps. The protocol
//! revision it speaks is `agent_protocols::PROTOCOL_VERSION` written a
//! second time, because the two ends must agree to talk at all; the
//! `_meta` key that marks an effect as unknown is
//! `agent_protocols::mcp::tools::EFFECT_META_KEY` written a second time,
//! because the city reads the key the server sets; the quality domain is
//! `kernel`'s written a second time, because both sides refuse at it; and
//! every `E_` code the package spells is required to be one `kernel`
//! already defines, which is the boundary desktop-SPEC.md section 8.5
//! drew around that duplication — the package may quote the city's
//! spelling and may never mint a code of its own.

use std::path::Path;

use super::diverged;
use crate::report::{Violation, XtaskError};

/// Where the protocol revision is decided, and where it is copied.
const PROTOCOL_HOME: &str = "crates/agent_protocols/src/mcp/handshake.rs";
const PROTOCOL_COPY: &str = "desktop/src/rpc.rs";
const PROTOCOL_NEEDLE: &str = "PROTOCOL_VERSION: &str =";
/// Where the effect-unknown key is decided, and where the server that
/// sets it quotes it.
const EFFECT_HOME: &str = "crates/agent_protocols/src/mcp/tools.rs";
const EFFECT_QUOTE: &str = "desktop/src/refusal.rs";
const EFFECT_NEEDLE: &str = "EFFECT_META_KEY: &str =";
/// Where the city's error codes are defined, and where the out-of-tree
/// package quotes them.
const CODE_HOME: &str = "crates/kernel/src/error/code.rs";
const CODE_QUOTE: &str = "desktop/src/refusal.rs";
/// The quality domain, stated in the city and copied into the
/// out-of-tree package, which sits outside the workspace and cannot read
/// the kernel's constant.
const QUALITY_HOME: &str = "crates/kernel/src/consts_policy.rs";
const QUALITY_QUOTE: &str = "desktop/src/platform/windows/encode.rs";

/// Every quoted fact, compared with the file that decides it.
pub(super) fn check(root: &Path, out: &mut Vec<Violation>) -> Result<(), XtaskError> {
    protocol(root, out)?;
    effect_key(root, out)?;
    codes_quoted(root, out)?;
    quality_domain(root, out)
}

/// The protocol revision both ends name.
fn protocol(root: &Path, out: &mut Vec<Violation>) -> Result<(), XtaskError> {
    let decided = stated(root, PROTOCOL_HOME, PROTOCOL_NEEDLE)?;
    let copied = stated(root, PROTOCOL_COPY, PROTOCOL_NEEDLE)?;
    if decided != copied {
        out.push(diverged(
            PROTOCOL_COPY.to_owned(),
            "both ends of this protocol name one revision, so the two constants hold one value",
            format!("`{copied}` against `{decided}` in {PROTOCOL_HOME}"),
            format!(
                "set `PROTOCOL_VERSION` in {PROTOCOL_COPY} to `{decided}`; a server and a \
                 client that disagree here do not finish a handshake"
            ),
        ));
    }
    Ok(())
}

/// The `_meta` key the server sets and the city reads.
fn effect_key(root: &Path, out: &mut Vec<Violation>) -> Result<(), XtaskError> {
    let decided = stated(root, EFFECT_HOME, EFFECT_NEEDLE)?;
    let copied = stated(root, EFFECT_QUOTE, EFFECT_NEEDLE)?;
    if decided != copied {
        out.push(diverged(
            EFFECT_QUOTE.to_owned(),
            "the effect-unknown key names one fact on both sides of the wall, so the two \
             constants hold one value",
            format!("`{copied}` against `{decided}` in {EFFECT_HOME}"),
            format!(
                "set `EFFECT_META_KEY` in {EFFECT_QUOTE} to `{decided}`; a key the city does \
                 not read makes every effect-unknown refusal read as an ordinary failure"
            ),
        ));
    }
    Ok(())
}

/// Every `E_` code the package spells, each one the city defines.
fn codes_quoted(root: &Path, out: &mut Vec<Violation>) -> Result<(), XtaskError> {
    let defined = codes(root, CODE_HOME)?;
    for minted in codes(root, CODE_QUOTE)?
        .into_iter()
        .filter(|code| !defined.contains(code))
    {
        out.push(diverged(
            CODE_QUOTE.to_owned(),
            "the out-of-tree package quotes the city's error codes and mints none of its own \
             (desktop-SPEC.md section 8.5, first pair)",
            format!("`{minted}` is spelled here and defined nowhere in {CODE_HOME}"),
            format!(
                "add the code to `kernel::AxCode` first, where every consumer of it can read \
                 it, then quote that spelling in {CODE_QUOTE}"
            ),
        ));
    }
    Ok(())
}

/// The quality domain the city states and the package copies.
///
/// The package cannot read `kernel::consts_policy::IMAGE_QUALITY`, so the
/// two numbers are compared as text, the way the protocol revision is.
fn quality_domain(root: &Path, out: &mut Vec<Violation>) -> Result<(), XtaskError> {
    let decided = number_after(root, QUALITY_HOME, "ImageQuality::new(")?;
    let copied = number_after(root, QUALITY_QUOTE, "QUALITY_MAX: u8 =")?;
    if decided != copied {
        out.push(diverged(
            QUALITY_QUOTE.to_owned(),
            "a value outside the quality domain is refused by whoever parses it, so both sides must refuse at the same number",
            format!("`{copied}` against `{decided}` in {QUALITY_HOME}"),
            format!("set `QUALITY_MAX` in {QUALITY_QUOTE} to `{decided}`; an encoder asked for a quality the city would have refused must not answer as if it were asked for something it has"),
        ));
    }
    Ok(())
}

/// The first run of digits after `needle` on the line that states it.
fn number_after(root: &Path, rel: &str, needle: &str) -> Result<String, XtaskError> {
    let text = crate::walk::read_text(&root.join(rel))?;
    text.lines()
        .find_map(|line| {
            let after = line.split_once(needle)?.1;
            let digits: String = after
                .trim_start()
                .chars()
                .take_while(char::is_ascii_digit)
                .collect();
            (!digits.is_empty()).then_some(digits)
        })
        .ok_or_else(|| XtaskError::Doc {
            file: rel.to_owned(),
            msg: format!(
                "this file no longer states `{needle}`, which is the shape the quality domain \
                 is compared in"
            ),
        })
}

/// The string one file states after `needle`: the first double-quoted
/// string on that line.
fn stated(root: &Path, rel: &str, needle: &str) -> Result<String, XtaskError> {
    let text = crate::walk::read_text(&root.join(rel))?;
    text.lines()
        .find_map(|line| quotes(line.split_once(needle)?.1).next())
        .ok_or_else(|| XtaskError::Doc {
            file: rel.to_owned(),
            msg: format!(
                "this file no longer states `{needle} \"…\"`, which is the shape both sides \
                 of the wall are compared in"
            ),
        })
}

/// Every `E_` code one file spells.
fn codes(root: &Path, rel: &str) -> Result<std::collections::BTreeSet<String>, XtaskError> {
    let text = crate::walk::read_text(&root.join(rel))?;
    Ok(text
        .lines()
        .flat_map(quotes)
        .filter(|found| found.starts_with("E_"))
        .collect())
}

/// Every double-quoted string on one line.
fn quotes(line: &str) -> impl Iterator<Item = String> + '_ {
    line.split('"')
        .skip(1)
        .step_by(2)
        .map(std::borrow::ToOwned::to_owned)
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::*;

    /// The copied constants are read out of real source, so the reader is
    /// the shape the source is written in rather than a regular expression
    /// somebody hoped would match it.
    #[test]
    fn the_quoted_facts_are_read_out_of_the_files_that_hold_them() {
        let root = crate::root::this_checkout().to_path_buf();
        assert_eq!(
            stated(&root, PROTOCOL_HOME, PROTOCOL_NEEDLE).unwrap(),
            stated(&root, PROTOCOL_COPY, PROTOCOL_NEEDLE).unwrap()
        );
        let defined = codes(&root, CODE_HOME).unwrap();
        let quoted = codes(&root, CODE_QUOTE).unwrap();
        assert!(quoted.contains("E_GATE_DENIED"), "{quoted:?}");
        assert!(quoted.is_subset(&defined), "{quoted:?}");
    }

    /// A key edited on one side only is named at the copy, which is the
    /// side that has to follow.
    #[test]
    fn a_copied_effect_key_that_drifted_is_named() {
        let root = std::env::temp_dir().join(format!("guard-effect-key-{}", std::process::id()));
        for (rel, key) in [
            (EFFECT_HOME, "sprawling/effect-unknown"),
            (EFFECT_QUOTE, "sprawling/effect-unknowable"),
        ] {
            let file = root.join(rel);
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(&file, format!("const EFFECT_META_KEY: &str = \"{key}\";\n")).unwrap();
        }
        let mut out = Vec::new();
        effect_key(&root, &mut out).unwrap();
        std::fs::remove_dir_all(&root).unwrap();
        assert_eq!(out.len(), 1, "{out:?}");
        assert_eq!(out[0].location, EFFECT_QUOTE);
    }
}
