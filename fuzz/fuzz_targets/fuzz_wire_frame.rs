// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![no_main]

use libfuzzer_sys::fuzz_target;

// The bytes of one inbound WebSocket frame, as a stranger wrote them.
//
// Property under fuzz: decoding a ClientFrame never panics, and an
// accepted frame is stable - re-encoding it and decoding that again
// yields an equal frame. Stability is what lets the server decide once
// on the decoded value: a frame that re-read differently would let the
// auth check and the dispatch see two different commands.
fuzz_target!(|data: &[u8]| {
    let Ok(frame) = serde_json::from_slice::<channels::ClientFrame>(data) else {
        return; // a refused frame is the ordinary outcome, a panic is not
    };
    let again = serde_json::to_vec(&frame).expect("a decoded frame re-encodes");
    let reread: channels::ClientFrame =
        serde_json::from_slice(&again).expect("our own encoding decodes");
    assert_eq!(frame, reread, "a frame must read the same way twice");

    // A Hello states which wire it speaks; the handshake compares that
    // against this build, so the decoder must not invent either half.
    if let channels::ClientFrame::Hello(hello) = &frame {
        assert!(
            hello.token.as_ref().is_none_or(|token| !token.is_empty()),
            "a present pairing token is a token, never an empty string"
        );
    }
});
