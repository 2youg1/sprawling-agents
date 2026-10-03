// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! A playback page: its data block, its references, what it loads, and
//! what a browser saw it do, each reported as its own item.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

use kernel::{AxCode, B3Hash, EventKind, RunId};
use serde_json::json;

use super::super::offline::findings;
use super::super::page::Page;
use super::super::{
    Asked, BUNDLE_BLOCK, City, Confidential, Reader, Verdict, check, embed, export,
};
use super::{city, lines, person, run, write};

/// A policy that passes: nothing from outside, the three closed
/// directives closed.
const CSP: &str = "<meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'; \
                   script-src 'unsafe-inline'; style-src 'unsafe-inline'; img-src data:; \
                   connect-src 'none'; base-uri 'none'; form-action 'none'\">";

/// A page with the policy first in its head.
fn page(head: &str, body: &str) -> String {
    format!(
        "<!doctype html><html><head>{CSP}<meta charset=\"utf-8\"><title>day</title>{head}\
         </head><body>{body}</body></html>"
    )
}

/// The page `body` makes around the bundle of the shared city.
fn embedded(body: &str) -> (tempfile::TempDir, Vec<u8>, B3Hash) {
    let (dir, _) = city();
    let bundle = export(dir.path(), &person()).unwrap();
    let made = embed(page(BUNDLE_BLOCK, body).as_bytes(), &bundle).unwrap();
    (dir, made, bundle.digest())
}

#[test]
fn an_embedded_page_holds_the_bundle_byte_for_byte_and_passes_both_static_items() {
    let dir = tempfile::tempdir().unwrap();
    let written = lines(vec![
        (RunId::CITY, None, EventKind::CityInitialized, json!({})),
        (
            run(1),
            Some("lab/a"),
            EventKind::ToolCalled,
            json!({"tool": "</script><img src=https://example.com/a.png>"}),
        ),
    ]);
    write(dir.path(), &written, b"");
    let bundle = export(dir.path(), &person()).unwrap();
    let made = embed(
        page(BUNDLE_BLOCK, "<p data-seq=\"1\">a call</p>").as_bytes(),
        &bundle,
    )
    .unwrap();
    let found = check(
        &made,
        &Asked {
            city: Some(City {
                root: dir.path(),
                reader: Reader::Person(Confidential::Withheld),
            }),
            ..Asked::default()
        },
    );
    assert_eq!(
        (
            found.holds(),
            found.digest,
            found.structure,
            found.source,
            found.offline,
        ),
        (
            true,
            Some(bundle.digest()),
            Verdict::Passed,
            Verdict::Passed,
            Verdict::Passed,
        )
    );
}

#[test]
fn the_five_items_are_reported_apart_and_an_unasked_one_is_unchecked() {
    let (_dir, made, _) = embedded("<p>day</p>");
    let found = check(&made, &Asked::default());
    assert_eq!(
        (
            found.holds(),
            found.structure,
            found.bundle,
            found.source,
            found.offline,
            found.browser,
        ),
        (
            true,
            Verdict::Passed,
            Verdict::Unasked {
                why: "no other bundle was given"
            },
            Verdict::Unasked {
                why: "no city was given"
            },
            Verdict::Passed,
            Verdict::Unasked {
                why: "no browser observation was given; the product does not run the page"
            },
        )
    );
}

#[test]
fn a_second_data_block_or_a_reused_id_or_a_dangling_reference_fails_the_structure() {
    let (dir, _) = city();
    let bundle =
        String::from_utf8(export(dir.path(), &person()).unwrap().bytes().to_vec()).unwrap();
    let block =
        format!("<script type=\"application/json\" id=\"playback-bundle\">{bundle}</script>");
    let structure =
        |head: &str, body: &str| check(page(head, body).as_bytes(), &Asked::default()).structure;
    assert_eq!(
        [
            structure(&format!("{block}{block}"), ""),
            structure(&block, "<p id=\"a\"></p><p id=\"a\"></p>"),
            structure(&block, "<a href=\"#nowhere\">x</a>"),
            structure(&block, "<p data-seq=\"999\">x</p>"),
            structure("", "<p>no data</p>"),
        ],
        [
            Verdict::Failed {
                found: "two elements carry the id `playback-bundle`".to_owned()
            },
            Verdict::Failed {
                found: "the id `a` is used twice".to_owned()
            },
            Verdict::Failed {
                found: "the link `#nowhere` names no id on the page".to_owned()
            },
            Verdict::Failed {
                found: "data-seq `999` names no seq in the bundle's events or context".to_owned()
            },
            Verdict::Failed {
                found: "no element carries the id `playback-bundle`".to_owned()
            },
        ]
    );
}

#[test]
fn a_duplicate_json_key_in_the_data_block_fails_the_structure() {
    let (dir, _) = city();
    let bundle =
        String::from_utf8(export(dir.path(), &person()).unwrap().bytes().to_vec()).unwrap();
    let doubled = bundle.replacen("{\"schema\":", "{\"schema\":\"x\",\"schema\":", 1);
    let block =
        format!("<script type=\"application/json\" id=\"playback-bundle\">{doubled}</script>");
    let found = check(page(&block, "").as_bytes(), &Asked::default());
    let Verdict::Failed { found } = found.structure else {
        panic!("a doubled key passed: {:?}", found.structure);
    };
    assert!(found.contains("duplicate field `schema`"), "{found}");
}

#[test]
fn every_way_out_of_the_page_is_a_finding() {
    let body = "<img src=\"https://example.com/a.png\">\
                <div style=\"background: u\\72l(http://example.com/b.png)\"></div>\
                <svg><title><img src=\"//example.com/c.png\"></title></svg>\
                <svg><image href=\"https://example.com/d.png\"/><rect fill=\"url(#g)\"/></svg>\
                <base href=\"https://example.com/\">\
                <form action=\"#\"></form>\
                <a href=\"https://example.com/\">out</a>\
                <img srcset=\"data:image/png;base64,AA 1x\">";
    let head = "<meta http-equiv=\"refresh\" content=\"0; url=https://example.com/\">\
                <style>@import \"theme.css\"; p { background: url(data:image/png;base64,AA) }</style>";
    assert_eq!(
        findings(&Page::read(&page(head, body))),
        [
            "<meta>: http-equiv `refresh` is not one a page carries",
            "<style>: @import loads another stylesheet",
            "<img> src=\"https://example.com/a.png\": loads from outside the page",
            "<div> style: url(http://example.com/b.png) loads from outside the page",
            "<img> src=\"//example.com/c.png\": loads from outside the page",
            "<image> href=\"https://example.com/d.png\": loads from outside the page",
            "<base>: a page holds no base",
            "<base> href=\"https://example.com/\": loads from outside the page",
            "<form>: a page holds no form",
            "<a> href=\"https://example.com/\": loads from outside the page",
            "<img> srcset: a page holds no candidate list",
        ]
    );
}

#[test]
fn a_policy_missing_late_or_open_to_a_host_is_a_finding() {
    let late = "<!doctype html><html><head><style>p{}</style>\
                <meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none' \
                https://example.com; connect-src 'none'; base-uri 'none'\"></head><body></body></html>";
    assert_eq!(
        [
            findings(&Page::read("<!doctype html><p>hi</p>")),
            findings(&Page::read(late)),
        ],
        [
            vec!["no Content-Security-Policy meta element".to_owned()],
            vec![
                "the Content-Security-Policy allows `https://example.com` in default-src, which \
                 is not a local source"
                    .to_owned(),
                "the Content-Security-Policy is not the first thing in the head: an element \
                 before it loads under no policy"
                    .to_owned(),
                "the Content-Security-Policy does not set form-action to 'none'".to_owned(),
            ],
        ]
    );
}

#[test]
fn an_observation_counts_only_for_the_bytes_it_names_and_fails_on_what_it_saw() {
    let (_dir, made, _) = embedded("<p>day</p>");
    let digest = B3Hash::digest(&made).to_string();
    let record = |page: &str, requests: serde_json::Value| {
        serde_json::to_vec(&json!({
            "page": page, "paths": ["load", "every evidence link"], "requests": requests,
            "navigations": [], "popups": [], "unresolved": [],
        }))
        .unwrap()
    };
    let browser = |record: &[u8]| {
        let found = check(
            &made,
            &Asked {
                observed: Some(record),
                ..Asked::default()
            },
        );
        (found.browser, found.covered)
    };
    let other = B3Hash::digest(b"another page").to_string();
    assert_eq!(
        [
            browser(&record(&digest, json!([]))),
            browser(&record(&digest, json!(["https://example.com/beacon"]))),
            browser(&record(&other, json!([]))),
        ],
        [
            (
                Verdict::Passed,
                vec!["load".to_owned(), "every evidence link".to_owned()]
            ),
            (
                Verdict::Failed {
                    found: "a request to https://example.com/beacon".to_owned()
                },
                Vec::new()
            ),
            (
                Verdict::Unable {
                    why: format!(
                        "the observation speaks of the page {other}, and this page is {digest}"
                    )
                },
                Vec::new()
            ),
        ]
    );
}

#[test]
fn a_template_without_exactly_one_empty_block_or_with_a_way_out_is_not_embedded() {
    let (dir, _) = city();
    let bundle = export(dir.path(), &person()).unwrap();
    let refused = |template: String| {
        embed(template.as_bytes(), &bundle).map_err(|err| (*err.code(), err.subject().to_owned()))
    };
    assert_eq!(
        [
            refused(page("", "")),
            refused(page(
                BUNDLE_BLOCK,
                "<img src=\"https://example.com/a.png\">"
            )),
        ],
        [
            Err((
                AxCode::InvalidArgs,
                "the template holds the empty bundle block 0 times, where it needs it once"
                    .to_owned()
            )),
            Err((
                AxCode::InvalidArgs,
                "<img> src=\"https://example.com/a.png\": loads from outside the page".to_owned()
            )),
        ]
    );
}

/// `crates/city/skills/playback/SKILL.md`, which ships beside the binary.
const SKILL: &str = include_str!("../../../../city/skills/playback/SKILL.md");

#[test]
fn the_skill_states_the_contract_this_build_writes() {
    let (dir, _) = city();
    let bundle = export(dir.path(), &person()).unwrap();
    let raw = String::from_utf8(bundle.bytes().to_vec()).unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(bundle.bytes()).unwrap();
    let mut sections: Vec<&String> = parsed.as_object().unwrap().keys().collect();
    sections.sort_by_key(|key| raw.find(&format!("\"{key}\":")).unwrap());
    let rows: Vec<Option<usize>> = sections
        .iter()
        .map(|key| SKILL.find(&format!("| `{key}` |")))
        .collect();
    let mut items: Vec<String> = Vec::new();
    let mut words: Vec<String> = Vec::new();
    for found in [
        check(bundle.bytes(), &Asked::default()),
        check(b"<p>no page</p>", &Asked::default()),
    ] {
        for (name, item) in found.line().as_object().unwrap() {
            if let Some(status) = item.get("status").and_then(serde_json::Value::as_str) {
                items.push(format!("| `{name}` |"));
                words.push(format!("`{status}`"));
            }
        }
    }
    let example = SKILL
        .split("```json\n")
        .nth(1)
        .and_then(|block| block.split("```").next())
        .unwrap();
    let (read, _) = super::super::observed::judge(example.as_bytes(), B3Hash::digest(b"x"));
    assert_eq!(
        (
            SKILL.contains(super::super::SCHEMA),
            SKILL.contains(BUNDLE_BLOCK),
            rows.iter().all(Option::is_some) && rows.is_sorted(),
            items
                .iter()
                .chain(&words)
                .all(|said| SKILL.contains(said.as_str())),
            matches!(&read, Verdict::Unable { why } if why.starts_with("the observation speaks of the page")),
        ),
        (true, true, true, true, true),
        "sections {sections:?} at {rows:?}; items {items:?}; statuses {words:?}; the example reads as {read:?}"
    );
}
