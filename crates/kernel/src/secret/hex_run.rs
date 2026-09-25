// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Hex-run detection: a pure-hex run is reported only when it is the
//! value of a credential name (kernel-SPEC §8-25).

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing, reason = "test code")]
mod tests {
    use crate::secret::scan;

    /// Probes are assembled at runtime so this file never holds a
    /// scannable span (`xtask secret` runs on it).
    fn hex40() -> String {
        ["9f3c1a7e5b", "20d48c6f1e", "a7b3905d2c", "e81f64b07a"].concat()
    }

    #[test]
    fn a_labelled_hex_key_is_reported() {
        let key = hex40();
        for text in [
            format!("HMAC_KEY={key}"),
            format!("api_token: \"{key}\""),
            format!("{{\"secret\": \"{key}\"}}"),
        ] {
            let hits = scan(text.as_bytes());
            assert_eq!(hits.len(), 1, "{text}");
            let hit = &hits[0];
            assert_eq!(&text.as_bytes()[hit.start..hit.start + hit.len], key.as_bytes());
            assert_eq!(hit.provider, None);
        }
    }
}
