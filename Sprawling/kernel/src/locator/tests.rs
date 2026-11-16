// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use super::*;
use crate::error::AxCode;

const H64: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const OID: &str = "0123456789abcdef0123456789abcdef01234567";

#[test]
fn cas_locator_roundtrips() {
    let raw = format!("cas:b3-{H64}");
    let loc = Locator::parse(&raw).unwrap();
    assert_eq!(loc.to_string(), raw);
    let ranged = format!("cas:b3-{H64}#L10-20");
    assert_eq!(Locator::parse(&ranged).unwrap().to_string(), ranged);
    let bytes = format!("cas:b3-{H64}#B0-1023");
    assert_eq!(Locator::parse(&bytes).unwrap().to_string(), bytes);
}

#[test]
fn cas_constructor_is_the_parsed_spelling_of_its_digest() {
    let hash = B3Hash::digest(b"stored bytes");
    let parsed = Locator::parse(&format!("cas:b3-{hash}")).unwrap();
    assert_eq!(Locator::cas(hash), parsed);
}

#[test]
fn a_digest_parses_only_its_canonical_hex() {
    let digest = B3Hash::digest(b"stored bytes");
    assert_eq!(digest.to_string().parse::<B3Hash>().unwrap(), digest);
    for invalid in ["", "abc", &H64.to_uppercase(), &format!("{H64}00")] {
        assert!(
            invalid.parse::<B3Hash>().is_err(),
            "{invalid:?} was accepted"
        );
    }
}

#[test]
fn file_locator_roundtrips_and_splits_on_last_at() {
    let raw = format!("file:role@building.1/notes.md@{OID}");
    let loc = Locator::parse(&raw).unwrap();
    assert_eq!(loc.to_string(), raw);
    match loc {
        Locator::File { address, .. } => {
            assert_eq!(address.as_str(), "role@building.1/notes.md");
        }
        Locator::Cas { .. } => panic!("expected file locator"),
    }
}

#[test]
fn fail_closed_on_every_malformed_shape() {
    let cases = [
        "".to_string(),
        "cas:".to_string(),
        format!("CAS:b3-{H64}"),
        format!("cas:B3-{H64}"),
        format!("cas:sha256-{H64}"),
        "cas:b3-abc".to_string(),
        format!("cas:b3-{}", H64.to_uppercase()),
        format!("cas:b3-{H64}#L0-5"),
        format!("cas:b3-{H64}#L9-5"),
        format!("cas:b3-{H64}#B01-2"),
        format!("cas:b3-{H64}#B+1-2"),
        format!("cas:b3-{H64}#X1-2"),
        format!("cas:b3-{H64}#L1-2 "),
        format!("file:notes.md@{OID}#"),
        "file:notes.md".to_string(),
        format!("file:../up@{OID}"),
        format!("file:notes.md@{}", OID.get(..39).unwrap()),
        "secret:openai/key".to_string(),
        format!("cas:b3-{H64}extra"),
    ];
    for bad in &cases {
        let err = Locator::parse(bad).unwrap_err();
        assert_eq!(err.code(), &AxCode::LocatorInvalid, "should reject {bad:?}");
    }
}

#[test]
fn range_constructor_validates_bounds() {
    assert!(Range::lines(1, 1).is_ok());
    assert!(Range::lines(0, 4).is_err());
    assert!(Range::bytes(0, 0).is_ok());
    assert!(Range::bytes(9, 3).is_err());
}

#[test]
fn b3hash_hex_roundtrip() {
    let h = B3Hash::from_bytes([0xab; 32]);
    let hex = h.to_string();
    assert_eq!(hex.len(), 64);
    assert_eq!(hex, "ab".repeat(32));
}

#[test]
fn serde_is_the_string_form() {
    let raw = format!("cas:b3-{H64}#B0-9");
    let loc: Locator = serde_json::from_str(&format!("\"{raw}\"")).unwrap();
    assert_eq!(serde_json::to_string(&loc).unwrap(), format!("\"{raw}\""));
}

proptest::proptest! {
    #[test]
    fn parse_display_roundtrip_is_identity_on_accepted_inputs(
        hexbytes in proptest::collection::vec(0u8..=255, 32),
        unit in 0..=1u8, a in 1u64..1000, span in 0u64..1000,
    ) {
        let h = B3Hash::from_bytes(<[u8; 32]>::try_from(hexbytes.as_slice()).unwrap());
        let b = a.checked_add(span).unwrap();
        let range = if unit == 0 { format!("#L{a}-{b}") } else { format!("#B{a}-{b}") };
        let raw = format!("cas:b3-{h}{range}");
        let parsed = Locator::parse(&raw).unwrap();
        proptest::prop_assert_eq!(parsed.to_string(), raw);
    }

    #[test]
    fn arbitrary_strings_never_panic(s in "\\PC*") {
        let _ = Locator::parse(&s);
    }
}

#[test]
fn a_refused_range_bound_names_the_rule_it_broke() {
    for (fragment, rule) in [
        ("L-5", "range bound must be decimal digits"),
        ("L1-", "range bound must be decimal digits"),
        ("L1-x", "range bound must be decimal digits"),
        ("B01-2", "range bound must not carry leading zeros"),
        ("B0-02", "range bound must not carry leading zeros"),
        ("B0-99999999999999999999", "range bound exceeds u64"),
    ] {
        let raw = format!("cas:b3-{H64}#{fragment}");
        let err = Locator::parse(&raw).unwrap_err();
        assert_eq!(
            (err.code(), err.subject(), err.recovery()),
            (&AxCode::LocatorInvalid, raw.as_str(), rule)
        );
    }
}

#[test]
fn a_hash_lends_the_bytes_it_holds() {
    let bytes: [u8; 32] = std::array::from_fn(|at| u8::try_from(at).unwrap());
    assert_eq!(B3Hash::from_bytes(bytes).as_bytes(), &bytes);
    assert_eq!(
        B3Hash::digest(b"content").as_bytes(),
        blake3::hash(b"content").as_bytes()
    );
}
