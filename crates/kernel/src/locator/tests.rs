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
        format!("CAS:b3-{H64}"),                             // scheme case
        format!("cas:B3-{H64}"),                             // tag case
        format!("cas:sha256-{H64}"),                         // unknown algorithm tag
        "cas:b3-abc".to_string(),                            // short hex
        format!("cas:b3-{}", H64.to_uppercase()),            // uppercase hex
        format!("cas:b3-{H64}#L0-5"),                        // lines are 1-based
        format!("cas:b3-{H64}#L9-5"),                        // from > to
        format!("cas:b3-{H64}#B01-2"),                       // leading zero (non-canonical)
        format!("cas:b3-{H64}#B+1-2"),                       // plus sign
        format!("cas:b3-{H64}#X1-2"),                        // unknown range unit
        format!("cas:b3-{H64}#L1-2 "),                       // trailing junk
        format!("file:notes.md@{OID}#"),                     // empty range
        "file:notes.md".to_string(),                         // missing oid
        format!("file:../up@{OID}"),                         // address grammar inside
        format!("file:notes.md@{}", OID.get(..39).unwrap()), // short oid
        "secret:openai/key".to_string(),                     // SecretRef never parses here
        format!("cas:b3-{H64}extra"),                        // overlong
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
