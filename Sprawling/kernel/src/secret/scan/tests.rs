// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;
use proptest::prelude::*;
#[test]
fn known_provider_shapes_are_found_with_offsets_only() {
    let text = format!("config = \"sk-ant-{}\" # key", "a1B2c3D4e5".repeat(9));
    let hits = scan(text.as_bytes());
    assert!(!hits.is_empty());
    let hit = &hits[0];
    assert_eq!(hit.provider, Some("anthropic"));
    let prefix_end = hit.start.checked_add(7).unwrap();
    assert_eq!(&text.as_bytes()[hit.start..prefix_end], b"sk-ant-");
}

#[test]
fn a_shape_s_body_runs_over_every_byte_of_its_charset() {
    let url_body = format!("ab-cd_ef={}", "x".repeat(20));
    let anthropic = format!("sk-ant-{url_body}");
    let aws = format!("AKIA{}", "AB12CD34EF56");
    for (text, provider) in [(anthropic, "anthropic"), (aws, "aws")] {
        assert_eq!(
            scan(text.as_bytes()),
            vec![SecretSpan {
                start: 0,
                len: text.len(),
                provider: Some(provider),
            }],
            "{text}"
        );
    }
}

#[test]
fn a_self_overlapping_prefix_opens_one_key() {
    let text = format!(
        "{}{}",
        "AKIA".repeat(2).replacen("AA", "A", 1),
        "B2C3D4E5F6G7H8"
    );
    let hits = scan(text.as_bytes());
    assert_eq!(hits, reference::scan(text.as_bytes()));
    assert_eq!(
        hits.iter()
            .map(|hit| (hit.start, hit.provider))
            .collect::<Vec<_>>(),
        [(0, Some("aws"))]
    );
}

#[test]
fn a_scan_counts_every_byte_it_reads() {
    let text = format!("note: {} and prose", "a1B2c3D4e5".repeat(3));
    let (_, work) = scan_counting(text.as_bytes());
    assert!(
        work.bytes_read >= u64::try_from(text.len()).unwrap(),
        "{work:?}"
    );
}

#[test]
fn high_entropy_tokens_hit_and_prose_does_not() {
    let token = ["kJ8vQ2xR9m", "W4nZ7pL3sT", "6yB1cD5fG0", "hN8aE2iU4o"].concat();
    let noisy = format!("token = {token} ok");
    let hits = scan(noisy.as_bytes());
    assert!(hits.iter().any(|h| h.provider.is_none()));
    let prose = b"the quick brown fox jumps over the lazy dog again and again";
    assert!(scan(prose).is_empty());
    let repeated = b"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    assert!(scan(repeated).is_empty(), "zero entropy never hits");
}

#[test]
fn city_native_artifacts_do_not_light_up() {
    let hex = format!("prev: {}", "ab".repeat(32));
    assert!(scan(hex.as_bytes()).is_empty());

    let uuid = b"run: 0198f6a2-7c4a-7bbb-9d1e-000000000001";
    assert!(scan(uuid).is_empty());

    let digits = b"t: 17555443211234567890123";
    assert!(scan(digits).is_empty());
}

#[test]
fn shape_hits_swallow_overlapping_entropy_hits() {
    let text = format!("sk-ant-{}", "a1B2c3D4e5".repeat(9));
    let hits = scan(text.as_bytes());
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].provider, Some("anthropic"));
}

#[test]
fn aggregator_tokens_with_lowercase_hex_bodies_are_found() {
    let hex64 = "0f1e2d3c".repeat(8);
    for (prefix, provider) in [("sk-or-v1-", "openrouter"), ("sk-ai-v1-", "zenmux")] {
        let text = format!("KEY={prefix}{hex64}");
        let found: Vec<_> = scan(text.as_bytes()).iter().map(|h| h.provider).collect();
        assert_eq!(found, vec![Some(provider)], "{prefix} is one shape hit");
    }
    let groq = format!("KEY=gsk_{}", "a1B2c3D4e5".repeat(5));
    assert_eq!(
        scan(groq.as_bytes()).first().and_then(|h| h.provider),
        Some("groq")
    );
}

#[test]
fn a_bare_hex_run_still_does_not_hit_without_its_prefix() {
    let hex64 = "0f1e2d3c".repeat(8);
    assert!(scan(format!("prev: {hex64}").as_bytes()).is_empty());
}

#[test]
fn log2_q10_matches_known_points() {
    assert_eq!(log2_q10(1), 0);
    assert_eq!(log2_q10(2), 1 << 10);
    assert_eq!(log2_q10(4), 2 << 10);

    let three = log2_q10(3);
    assert!((1622..=1624).contains(&three), "{three}");
}

#[test]
fn uniform_bytes_have_full_entropy() {
    let bytes: Vec<u8> = (0u8..64).collect();
    let mb = entropy_millibits_per_char(&bytes);
    assert!((5900..=6000).contains(&mb), "{mb}");
}

proptest! {

    #[test]
    fn scan_is_total_and_in_bounds(bytes in proptest::collection::vec(any::<u8>(), 0..256)) {
        for hit in scan(&bytes) {
            prop_assert!(hit.start.checked_add(hit.len).unwrap() <= bytes.len());
        }
    }

    #[test]
    fn scan_reports_what_the_reference_reports(pieces in proptest::collection::vec(piece(), 0..24)) {
        let text = pieces.concat();
        prop_assert_eq!(scan(text.as_bytes()), reference::scan(text.as_bytes()));
    }

    #[test]
    fn scan_matches_the_reference_on_any_bytes(bytes in proptest::collection::vec(any::<u8>(), 0..512)) {
        prop_assert_eq!(scan(&bytes), reference::scan(&bytes));
    }
}

#[test]
fn scan_matches_the_reference_on_the_test_corpus() {
    let mixed = "a1B2c3D4e5".repeat(9);
    let hex40 = ["9f3c1a7e5b", "20d48c6f1e", "a7b3905d2c", "e81f64b07a"].concat();
    let hex64 = "0f1e2d3c".repeat(8);
    let token = ["kJ8vQ2xR9m", "W4nZ7pL3sT", "6yB1cD5fG0", "hN8aE2iU4o"].concat();
    let mut corpus = vec![
        format!("config = \"sk-ant-{mixed}\" # key"),
        format!("token = {token} ok"),
        format!("prev: {}", "ab".repeat(32)),
        "run: 0198f6a2-7c4a-7bbb-9d1e-000000000001".to_owned(),
        format!("HMAC_KEY={hex40}"),
        format!("api_token: \"{hex40}\""),
        format!("{{\"secret\": \"{hex40}\"}}"),
        format!("commit {hex40} oid={hex40} KEY={}", "0f1e2d3c".repeat(5)),
        format!("KEY=gsk_{}", "a1B2c3D4e5".repeat(5)),
        format!("密钥 KEY = '{hex40}' 和 {token}é {mixed}"),
    ];
    for shape in &SECRET_SHAPES {
        corpus.push(format!("KEY={}{hex64}{mixed}", shape.prefix));
        corpus.push(format!("{}{}{}", shape.prefix, shape.prefix, &mixed[..12]));
    }
    for text in corpus {
        assert_eq!(
            scan(text.as_bytes()),
            reference::scan(text.as_bytes()),
            "{text}"
        );
    }
}

fn piece() -> impl Strategy<Value = String> {
    let shape = (0..SECRET_SHAPES.len(), "[A-Za-z0-9_=-]{0,90}").prop_map(|(at, body)| {
        let prefix = SECRET_SHAPES.get(at).map_or("", |s| s.prefix);
        format!("{prefix}{body}")
    });
    let label = prop_oneof![
        Just("HMAC_KEY"),
        Just("api_token"),
        Just("\"secret\""),
        Just("oid"),
        Just("commit"),
        Just("name.session"),
        Just("x")
    ];
    let separator = prop_oneof![Just("="), Just(": "), Just(" = '"), Just(":\""), Just(" ")];
    let hex = prop_oneof!["[0-9a-f]{28,70}", "(0f1e2d3c){4,6}", "[0-9A-F]{32,40}"];
    let labelled = (label, separator, hex).prop_map(|(l, s, h)| format!("{l}{s}{h}"));
    prop_oneof![
        shape,
        labelled,
        "[A-Za-z0-9+/_-]{15,60}",
        "[a-z ]{0,30}",
        Just("密钥 é 🔑 ".to_owned()),
        prop_oneof![
            Just(" "),
            Just(
                "
"
            ),
            Just("\""),
            Just("="),
            Just(":")
        ]
        .prop_map(str::to_owned),
    ]
}

mod reference {
    use super::super::{
        ENTROPY_SPAN_MIN_BYTES, SECRET_ENTROPY_MIN, Work, charset_admits,
        entropy_millibits_per_char, token_byte,
    };
    use crate::consts_external::SECRET_SHAPES;
    use crate::secret::hex_run::{HEX_ENTROPY_MIN_MILLIBITS, HEX_SPAN_MIN_BYTES, label_before};
    use crate::secret::names_a_credential;
    use crate::secret::span::SecretSpan;

    fn count(work: &mut Work, bytes: usize) {
        work.bytes_read += bytes as u64;
    }

    fn entropy(work: &mut Work, run: &[u8]) -> u64 {
        work.entropy_readings += 1;
        count(work, run.len());
        entropy_millibits_per_char(run)
    }

    fn mixed_alphabet(work: &mut Work, bytes: &[u8]) -> bool {
        count(work, 3 * bytes.len());
        let has_upper = bytes.iter().any(u8::is_ascii_uppercase);
        let has_lower = bytes.iter().any(u8::is_ascii_lowercase);
        let has_digit = bytes.iter().any(u8::is_ascii_digit);
        has_upper && has_lower && has_digit
    }

    fn entropy_passes(work: &mut Work, bytes: &[u8]) -> bool {
        let mb = entropy(work, bytes);
        let num = u64::from(SECRET_ENTROPY_MIN.num);
        let den = u64::from(SECRET_ENTROPY_MIN.den);
        mb.saturating_mul(den) >= num.saturating_mul(1000)
    }

    fn overlaps(a: &SecretSpan, start: usize, len: usize) -> bool {
        let a_end = a.start.saturating_add(a.len);
        let b_end = start.saturating_add(len);
        a.start < b_end && start < a_end
    }

    fn is_labelled_hex_secret(work: &mut Work, bytes: &[u8], start: usize, end: usize) -> bool {
        let Some(run) = bytes.get(start..end) else {
            return false;
        };
        run.len() >= HEX_SPAN_MIN_BYTES
            && run.iter().all(u8::is_ascii_hexdigit)
            && entropy(work, run) >= HEX_ENTROPY_MIN_MILLIBITS
            && bytes
                .get(..start)
                .and_then(label_before)
                .is_some_and(names_a_credential)
    }

    fn find_shape_hits(work: &mut Work, bytes: &[u8]) -> Vec<SecretSpan> {
        let mut hits = Vec::new();
        for shape in &SECRET_SHAPES {
            let prefix = shape.prefix.as_bytes();
            if prefix.is_empty() || bytes.len() < prefix.len() {
                continue;
            }
            count(work, bytes.len());
            let mut at = 0usize;
            while let Some(window) = bytes.get(at..) {
                let Some(rel) = window.windows(prefix.len()).position(|w| w == prefix) else {
                    break;
                };
                let start = at + rel;
                let body_start = start + prefix.len();
                let mut end = body_start;
                while bytes
                    .get(end)
                    .is_some_and(|b| charset_admits(shape.charset, *b))
                {
                    end += 1;
                }
                count(work, end - body_start);
                let total = end - start;
                let (min_len, max_len) = (usize::from(shape.len.0), usize::from(shape.len.1));
                if total >= min_len {
                    hits.push(SecretSpan {
                        start,
                        len: total.min(max_len),
                        provider: Some(shape.provider),
                    });
                }
                at = body_start;
            }
        }
        hits.sort_by_key(|h| (h.start, h.len));
        hits
    }

    pub(super) fn scan(bytes: &[u8]) -> Vec<SecretSpan> {
        scan_counting(bytes).0
    }

    pub(super) fn scan_counting(bytes: &[u8]) -> (Vec<SecretSpan>, Work) {
        let mut work = Work::default();
        let mut hits = find_shape_hits(&mut work, bytes);
        let mut at = 0usize;
        while at < bytes.len() {
            if !bytes.get(at).copied().is_some_and(token_byte) {
                at += 1;
                continue;
            }
            let mut end = at;
            while bytes.get(end).copied().is_some_and(token_byte) {
                end += 1;
            }
            let len = end - at;
            count(&mut work, len);
            if len >= ENTROPY_SPAN_MIN_BYTES
                && !hits.iter().any(|h| overlaps(h, at, len))
                && (bytes.get(at..end).is_some_and(|run| {
                    mixed_alphabet(&mut work, run) && entropy_passes(&mut work, run)
                }) || is_labelled_hex_secret(&mut work, bytes, at, end))
            {
                hits.push(SecretSpan {
                    start: at,
                    len,
                    provider: None,
                });
            }
            at = end;
        }
        hits.sort_by_key(|h| (h.start, h.len));
        (hits, work)
    }
}

fn history(lines: usize) -> String {
    let hex64 = "9f3c1a7e5b20d48c6f1ea7b3905d2ce81f64b07a".repeat(2);
    let token = ["kJ8vQ2xR9m", "W4nZ7pL3sT", "6yB1cD5fG0", "hN8aE2iU4o"].concat();
    let line = format!(
        "{{\"prev\":\"{hex64}\",\"oid\":\"{}\",\"HMAC_KEY\":\"{}\",\"note\":\"see sk-ant-{} and {token} for the run\"}}\n",
        &hex64[..40],
        &hex64[3..43],
        "a1B2c3D4e5".repeat(4)
    );
    line.repeat(lines)
}

#[test]
fn scan_work_is_linear_and_below_the_reference() {
    let sizes = [64usize, 128, 256];
    let mut readings = Vec::new();
    for lines in sizes {
        let text = history(lines);
        let (spans, work) = scan_counting(text.as_bytes());
        let (expected, before) = reference::scan_counting(text.as_bytes());
        assert_eq!(spans, expected);
        assert!(
            work.bytes_read < before.bytes_read,
            "{lines}: {work:?} {before:?}"
        );
        assert!(
            work.entropy_readings < before.entropy_readings,
            "{lines}: {work:?} {before:?}"
        );
        readings.push(work);
    }
    let base = readings[0];
    for (factor, work) in [(1u64, readings[0]), (2, readings[1]), (4, readings[2])] {
        assert!(work.bytes_read <= factor * base.bytes_read, "{work:?}");
        assert!(
            work.entropy_readings <= factor * base.entropy_readings,
            "{work:?}"
        );
    }
}
