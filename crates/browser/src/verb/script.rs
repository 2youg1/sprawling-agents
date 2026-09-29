// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The scripts a verb sends, and the one shape every reply comes back
//! in.
//!
//! Every script this crate sends returns one JSON string, so there is
//! one reply shape to read rather than the driver's whole remote-value
//! vocabulary. Keeping the scripts beside that reader is what makes the
//! promise checkable in one place.

use kernel::{AxCode, AxError, Payload};
use serde_json::{Value, json};

use super::read::missing;
use crate::snapshot::PageSnapshot;

pub(super) fn read_references(args: &Payload) -> Result<Vec<String>, AxError> {
    let raw = args
        .as_map()
        .get("refs")
        .and_then(Value::as_array)
        .ok_or_else(|| missing("refs"))?;
    let mut references = Vec::new();
    for entry in raw {
        let text = entry.as_str().ok_or_else(|| {
            AxError::failure(
                AxCode::InvalidArgs,
                "read a browser action",
                "a reference that is not a string",
            )
            .with_recovery("references are minted by the snapshot and look like `e1`")
        })?;
        references.push(text.to_owned());
    }
    Ok(references)
}

/// Builds the expression that measures the named nodes.
///
/// Every reference goes through the snapshot first, so a reference the
/// model invented is refused here rather than answered with a box that
/// belongs to something else.
pub(crate) fn measure_script(
    snapshot: &PageSnapshot,
    references: &[String],
) -> Result<String, AxError> {
    let mut parts = Vec::new();
    for reference in references {
        snapshot.resolve(reference)?;
        parts.push(format!(
            "(el => {{ if (!el) return {{ ref: {reference}, x: 0, y: 0, width: 0, height: 0 }}; \
             const box = boxOf(el); return {{ ref: {reference}, x: box[0], y: box[1], \
             width: box[2], height: box[3] }}; }})({selector})",
            reference = json!(reference),
            selector = crate::act::selector_of(snapshot, reference)?,
        ));
    }
    Ok(format!(
        "{box_of}; JSON.stringify([{parts}])",
        box_of = crate::geometry::BOX_OF,
        parts = parts.join(",")
    ))
}

/// How much of a fetched body crosses the socket, in UTF-16 units: a
/// ceiling on what the page hands back, reported with the length it cut
/// from. How far a tool result is shortened after that is the pipeline's
/// decision (browser-SPEC.md 19-2).
const FETCH_TEXT_MAX_CHARS: usize = 65_536;

/// The page's own `fetch`, run where the page runs.
///
/// The request leaves from the open page with its cookies and under its
/// same-origin and CORS rules; a redirect stops at the redirect, so every
/// host that receives a request is one the egress door judged. HTML is
/// parsed with `DOMParser` and never attached to the live document, which
/// would fetch its images and run its scripts, then walked to text with a
/// line break around each block element. A body that is not text is not
/// read: a byte count is feedback a model can use, and mojibake is not.
const FETCH_SCRIPT: &str = concat!(
    "(async () => { const url = __URL__; const max = __MAX__; let r; ",
    "try { r = await fetch(url, { credentials: 'include', redirect: 'manual' }); } ",
    "catch (e) { return JSON.stringify({ error: String(e) }); } ",
    "const type = r.headers.get('content-type') || ''; ",
    "const base = { status: r.status, type, url: r.url || url }; ",
    "if (r.type === 'opaqueredirect') { return JSON.stringify({ ...base, ",
    "text: 'the address redirects; fetch the address it names to follow it', ",
    "cut: false, chars: 0 }); } ",
    "if (type !== '' && !/^text\\/|json|xml|javascript|html/i.test(type)) { ",
    "const held = await r.arrayBuffer(); ",
    "return JSON.stringify({ ...base, binary: true, bytes: held.byteLength }); } ",
    "let body = await r.text(); ",
    "if (/html/i.test(type)) { ",
    "const doc = new DOMParser().parseFromString(body, 'text/html'); ",
    "doc.querySelectorAll('script,style,noscript,template').forEach(e => e.remove()); ",
    "const blocks = new Set(['address','article','aside','blockquote','dd','div','dl','dt',",
    "'figcaption','figure','footer','form','h1','h2','h3','h4','h5','h6','header','hr','li',",
    "'main','nav','ol','p','pre','section','table','tr','td','th','ul']); ",
    "const out = []; ",
    "const walk = (n, pre) => { if (n.nodeType === 3) { out.push(pre ",
    "? n.nodeValue.replace(/\\n/g, '\\uE000').replace(/ /g, '\\uE001') ",
    ": n.nodeValue.replace(/\\s+/g, ' ')); return; } ",
    "if (n.nodeType !== 1) { return; } const tag = n.tagName.toLowerCase(); ",
    "if (tag === 'br') { out.push('\\n'); return; } const block = blocks.has(tag); ",
    "if (block) { out.push('\\n'); } ",
    "for (const c of n.childNodes) { walk(c, pre || tag === 'pre'); } ",
    "if (block) { out.push('\\n'); } }; ",
    "walk(doc.body || doc.documentElement, false); ",
    "const title = (doc.title || '').trim(); ",
    "body = (title ? title + '\\n\\n' : '') + out.join(''); ",
    "body = body.split('\\n').map(l => l.replace(/ +/g, ' ').trim()).join('\\n')",
    ".replace(/\\n{3,}/g, '\\n\\n').trim()",
    ".replace(/\\uE000/g, '\\n').replace(/\\uE001/g, ' '); } ",
    "const chars = body.length; ",
    "return JSON.stringify({ ...base, text: body.slice(0, max), cut: chars > max, chars }); })()"
);

/// Builds the expression that fetches `url` from the open page. The
/// address enters as a JSON string literal, so nothing in it is read as
/// code, and it enters last, so nothing in it is taken for a placeholder.
pub(crate) fn fetch_script(url: &str) -> String {
    FETCH_SCRIPT
        .replacen("__MAX__", &FETCH_TEXT_MAX_CHARS.to_string(), 1)
        .replacen("__URL__", &json!(url).to_string(), 1)
}

/// Reads what an evaluate came back with.
///
/// Every script this module sends returns one JSON string, so there is
/// one shape to read rather than the driver's whole remote-value
/// vocabulary.
///
/// # Errors
/// Refuses a reply whose shape this version does not read, and a string
/// that is not the JSON the script promised.
pub fn read_json(result: &Value) -> Result<Value, AxError> {
    let text = result
        .get("result")
        .and_then(|inner| inner.get("value"))
        .and_then(Value::as_str)
        .ok_or_else(|| {
            AxError::failure(
                AxCode::WireMismatch,
                "read a script result",
                "no string value",
            )
            .with_recovery("check the driver's protocol version")
        })?;
    serde_json::from_str(text).map_err(|err| {
        AxError::failure(
            AxCode::WireMismatch,
            "read a script result",
            err.to_string(),
        )
        .with_recovery("the script returns JSON.stringify of its answer")
    })
}

/// Whether the console said anything the development loop should stop
/// for. Error level only: a warning is the page talking, an error is the
/// page failing.
#[must_use]
pub fn complained(console: &Value) -> bool {
    console.as_array().is_some_and(|entries| {
        entries
            .iter()
            .any(|entry| entry.get("level").and_then(Value::as_str) == Some("error"))
    })
}
