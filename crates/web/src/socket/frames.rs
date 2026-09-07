// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Reading frames: text in, link events out.

use channels::ServerFrame;

use super::link::LinkEvent;

/// Parses one text frame the way the client must read it. A frame this
/// build cannot parse is read as a closed link rather than skipped: the
/// two ends disagree about the wire, and the machine already knows what
/// to do about that.
#[must_use]
pub fn read_frame(text: &str) -> LinkEvent {
    match serde_json::from_str::<ServerFrame>(text) {
        Ok(frame) => LinkEvent::Received(Box::new(frame)),
        Err(_) => LinkEvent::Closed,
    }
}

/// The pairing code the host put on the URL that opened this page.
///
/// Pure over the query string, so the judgement is testable off the
/// browser and the browser half below holds none of it. The value is
/// taken as written: the codes this city mints come from an alphabet of
/// digits and lower-case letters, which needs no unescaping, and a
/// configured token carrying reserved characters fails visibly at the
/// handshake with "the pairing token does not match" rather than
/// silently connecting as somebody else.
///
/// An empty value is not a value. A peer that sends `token=` would be
/// refused anyway, and answering `None` keeps the refusal at the one
/// place that decides it.
#[must_use]
pub fn token_in(search: &str) -> Option<String> {
    search
        .trim_start_matches('?')
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .find(|(name, _)| *name == "token")
        .map(|(_, value)| value.to_owned())
        .filter(|value| !value.is_empty())
}

/// The pairing code this page was opened with, read from where the host
/// put it.
///
/// One call and no judgement: everything that could be decided is in
/// [`token_in`]. Browser-only for the same reason [`socket_url`] below
/// is - off the browser there is no location to read, and its one caller
/// is unreachable without one.
#[cfg(target_arch = "wasm32")]
#[must_use]
pub fn pairing_token() -> Option<String> {
    token_in(&web_sys::window()?.location().search().ok()?)
}

/// The address of this city's socket, derived from the page's own origin.
/// A client that is served by the city it talks to needs no configured
/// endpoint, and cannot be pointed at a second one by accident.
#[cfg(target_arch = "wasm32")]
#[must_use]
pub fn socket_url() -> Option<String> {
    let location = web_sys::window()?.location();
    let host = location.host().ok()?;
    let scheme = match location.protocol().ok()?.as_str() {
        "https:" => "wss",
        _ => "ws",
    };
    Some(format!("{scheme}://{host}/ws"))
}
