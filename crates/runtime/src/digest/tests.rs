// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;

const DOC: &str = "\
# Title

intro line

## First

body of first

```sh
# not a heading
```

## Second

body of second
";

#[test]
fn headings_become_a_tree_and_code_fences_do_not() {
    let nodes = structure_of(DOC);
    let titles: Vec<&str> = nodes.iter().map(|n| n.title.as_str()).collect();
    assert_eq!(titles, ["Title", "First", "Second"]);
    assert_eq!(nodes[0].level, 1);
    assert_eq!(nodes[1].level, 2);
    // The top heading spans the whole document; a section spans up to
    // the next heading of its own depth.
    assert_eq!(usize::try_from(nodes[0].span.get()).unwrap(), DOC.len());
    assert!(nodes[1].span.get() < nodes[0].span.get());
}

#[test]
fn a_document_without_headings_has_no_structure_rather_than_a_failure() {
    assert!(structure_of("just prose, no headings\n").is_empty());
}

#[test]
fn prose_is_suspect_and_says_so_where_a_reader_will_see_it() {
    let digest = Digest::structural(B3Hash::digest(b"x"), None, DOC);
    assert!(!digest.is_suspect());
    assert!(digest.window_header().contains("read from the source"));

    let written = digest.with_prose("it is about three things".to_owned());
    assert!(written.is_suspect());
    assert!(written.window_header().contains("suspect"));
    assert!(written.window_header().contains("the source decides"));
}

#[test]
fn a_content_hash_is_digested_once_in_its_life() {
    use std::cell::RefCell;

    let mut breaker = Breaker::new(3);
    let calls = RefCell::new(0u32);
    let store: RefCell<Option<Digest>> = RefCell::new(None);
    let first = {
        let mut cached = |hash: &B3Hash| -> Result<Option<Digest>, AxError> {
            Ok(store
                .borrow()
                .as_ref()
                .filter(|d| &d.source() == hash)
                .cloned())
        };
        let mut write = |_: &str| -> Result<String, AxError> {
            *calls.borrow_mut() += 1;
            Ok("a summary".to_owned())
        };
        digest_once(DOC, None, &mut breaker, &mut cached, &mut write).unwrap()
    };
    let DigestOutcome::Fresh(fresh) = first else {
        panic!("the first pass digests");
    };
    assert_eq!(*calls.borrow(), 1);

    *store.borrow_mut() = Some(fresh);
    let again = {
        let mut cached = |hash: &B3Hash| -> Result<Option<Digest>, AxError> {
            Ok(store
                .borrow()
                .as_ref()
                .filter(|d| &d.source() == hash)
                .cloned())
        };
        let mut write = |_: &str| -> Result<String, AxError> {
            *calls.borrow_mut() += 1;
            Ok("a summary".to_owned())
        };
        digest_once(DOC, None, &mut breaker, &mut cached, &mut write).unwrap()
    };
    assert!(matches!(again, DigestOutcome::Cached(_)));
    assert_eq!(*calls.borrow(), 1, "the same bytes cost one digest, ever");
}

#[test]
fn the_breaker_opens_after_repeated_failure_and_the_structure_still_stands() {
    let mut breaker = Breaker::new(2);
    let mut attempts = 0u32;
    let mut cached = |_: &B3Hash| -> Result<Option<Digest>, AxError> { Ok(None) };
    let mut write = |_: &str| -> Result<String, AxError> {
        attempts += 1;
        Err(AxError::failure(
            AxCode::DigestSuspect,
            "summarise a document",
            "the provider refused",
        ))
    };

    for _ in 0..4 {
        let outcome = digest_once(DOC, None, &mut breaker, &mut cached, &mut write).unwrap();
        let DigestOutcome::Structural { digest, .. } = outcome else {
            panic!("a failing digester still leaves the headings");
        };
        assert!(
            !digest.is_suspect(),
            "nothing was written, nothing to doubt"
        );
        assert_eq!(digest.structure().len(), 3);
    }
    assert_eq!(attempts, 2, "the breaker stops asking after its limit");
    assert!(matches!(breaker.verdict(), BreakerVerdict::Open { .. }));

    // One success closes it again: intermittent is not broken.
    breaker.succeeded();
    assert_eq!(breaker.verdict(), BreakerVerdict::Attempt);
}
