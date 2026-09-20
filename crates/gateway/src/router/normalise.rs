// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a person pasted, turned into the base URL the city calls and
//! the request shape that URL answers in (shape 1 decision).
//!
//! A provider's documentation prints three different things under one
//! name: the chat URL (`https://x/v1/chat/completions`), the base URL
//! (`https://x/v1`), and the host alone (`https://x`). All three are
//! the same endpoint, so all three must arrive at one registration —
//! otherwise the city appends a path to a URL that already carries one
//! and the first call answers 404.
//!
//! **This algorithm is written once, here.** The form sends what the
//! person typed and shows what came back, so a second spelling of the
//! rule cannot exist on the client side to drift from this one.
//!
//! The host table is likewise written once, in `provider::preset`, and
//! reaches this module through [`HostDefaults`]: `openrouter.ai` serves
//! at `/api/v1` and a Gemini-shaped host at `/v1beta`, so appending
//! `/v1` to every path-less URL would break the hosts the city knows
//! best. Normalisation and the preset table are two halves of one
//! thing, which is why this module has no host names in it.

use kernel::{AxCode, AxError};

/// The path appended to a host the preset table does not list, and only
/// when the person entered no path at all.
const UNLISTED_HOST_PATH: &str = "/v1";

/// Which request shape an endpoint answers in.
///
/// `Unset` is a real state, not a missing value: the person has not
/// chosen, no tail path said so, and no preset knows this host, so the
/// form must ask rather than guess. The three named states are the
/// three faces a provider serves, in the spelling the settings page
/// uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialectHint {
    /// Nobody has said yet.
    Unset,
    /// OpenAI-compatible chat completions.
    Chat,
    /// OpenAI responses.
    Responses,
    /// Anthropic messages.
    Messages,
}

/// One entered URL, resolved.
///
/// The two answers travel together because one input decides both, and
/// a caller that took the URL without the dialect would call the right
/// host in the wrong shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Normalised {
    /// The base URL every face hangs its own path off: scheme, host,
    /// and the provider's path prefix, with no trailing slash.
    pub base_url: String,
    /// The shape derived from the URL, the person's choice, or the
    /// preset table, in that order of authority.
    pub dialect: DialectHint,
}

/// What the provider preset table says about a host.
///
/// Crate-internal on purpose: normalisation stays free of host names,
/// `provider::preset` owns the table, and a test supplies its own rows
/// to state a case in one place. It is not a published seam — the
/// shipped build has one implementation, and callers outside this crate
/// reach the algorithm through [`crate::normalise_entered`], which
/// hands it the table this build ships.
pub(crate) trait HostDefaults {
    /// The path this host serves its API under, without the host —
    /// `/api/v1` for `openrouter.ai`, `/v1beta` for a Gemini-shaped
    /// host. `None` when the table does not list the host.
    fn default_path(&self, host: &str) -> Option<&str>;

    /// The shape this host answers in when the person has not chosen
    /// and the URL does not say. `None` when the table does not list
    /// the host or records no shape for it.
    fn default_dialect(&self, host: &str) -> Option<DialectHint>;
}

/// Resolve what a person entered, against the provider table this build
/// ships.
///
/// The one door out of this crate. The algorithm and the table are
/// separate inside it so neither holds the other's facts, and a caller
/// outside must not be able to pair the algorithm with a second table.
///
/// # Errors
/// When the entered text carries a scheme this city cannot call, or no
/// host to call at all.
pub fn normalise_entered(entered: &str, hint: DialectHint) -> Result<Normalised, AxError> {
    normalise(entered, hint, &crate::provider::preset::Presets)
}

/// Resolve what a person entered into the base URL the city stores and
/// the shape it answers in.
///
/// The rules apply in this order, and each later rule sees what the
/// earlier ones left:
///
/// 1. A known face at the end of the path is removed and decides the
///    shape — `/chat/completions`, `/responses`, `/messages`. The URL
///    outranks the person's choice here, because a pasted URL is
///    evidence and a toggle left on its default is not.
/// 2. An empty path is filled from the preset table, and from
///    [`UNLISTED_HOST_PATH`] only when the table does not list the
///    host. A path the person entered is never rewritten.
/// 3. A missing scheme becomes `https://`, or `http://` for an address
///    on this machine, judged by the city's one locality test.
///
/// Running the result through this function again returns that same
/// result, provided the dialect it produced is passed back as the hint.
///
/// # Errors
/// Refuses a URL whose scheme is neither `http` nor `https`, and one
/// with no host to call.
pub fn normalise(
    entered: &str,
    hint: DialectHint,
    presets: &dyn HostDefaults,
) -> Result<Normalised, AxError> {
    let trimmed = entered.trim();
    let (given_scheme, locator) = match trimmed.split_once("://") {
        Some((scheme, rest)) => (Some(scheme.to_ascii_lowercase()), rest),
        None => (None, trimmed),
    };
    if let Some(scheme) = given_scheme.as_deref()
        && scheme != "http"
        && scheme != "https"
    {
        return Err(refusal(
            entered,
            format!("{scheme} is not a scheme the city can call"),
            "enter an http:// or https:// URL, the one the provider's documentation prints",
        ));
    }
    // Locality decides the scheme, and the locality test needs one, so
    // the probe carries the plain scheme and is never stored.
    let probe = match given_scheme.as_deref() {
        Some(scheme) => format!("{scheme}://{locator}"),
        None => format!("http://{locator}"),
    };
    let Some((host, _, _)) = crate::reach::split(&probe) else {
        return Err(refusal(
            entered,
            "no host to call",
            "enter the base URL of the provider, for example https://api.openai.com/v1",
        ));
    };
    let host = host.to_ascii_lowercase();
    let scheme = match given_scheme {
        Some(scheme) => scheme,
        None if crate::is_local(&probe) => "http".to_owned(),
        None => "https".to_owned(),
    };
    let (address, suffix) = split_suffix(locator);
    let (authority, entered_path) = match address.split_once('/') {
        Some((authority, path)) => (authority, path),
        None => (address, ""),
    };
    let segments: Vec<&str> = entered_path.split('/').filter(|s| !s.is_empty()).collect();
    let (kept, from_url) = strip_face(&segments);
    let path = resolve_path(&kept, &suffix, &host, presets);
    let dialect = match resolve_dialect(from_url, hint, &host, presets) {
        // A server on this machine is an OpenAI-compatible one until it
        // says otherwise: that is what every local inference runtime
        // serves, and it is the same assumption the chat path already
        // makes for a dialect it does not recognise.
        DialectHint::Unset if crate::is_local(&probe) => DialectHint::Chat,
        settled @ (DialectHint::Unset
        | DialectHint::Chat
        | DialectHint::Responses
        | DialectHint::Messages) => settled,
    };
    Ok(Normalised {
        base_url: format!("{scheme}://{}{path}{suffix}", lowercase_host(authority)),
        dialect,
    })
}

/// The one refusal this module makes, so that every rejected URL names
/// the same action and carries a recovery the form can print.
fn refusal(entered: &str, subject: impl Into<String>, recovery: &str) -> AxError {
    AxError::failure(
        AxCode::ConfigInvalid,
        format!("read the endpoint URL {entered}"),
        subject,
    )
    .with_recovery(recovery.to_owned())
}

/// The query or fragment a person pasted, kept apart from the path so
/// that it neither hides a known face nor counts as a path of its own.
/// Both are carried through verbatim: a provider that versions its API
/// in the query string is calling a different endpoint without it.
fn split_suffix(locator: &str) -> (&str, String) {
    if let Some((address, query)) = locator.split_once('?') {
        return (address, format!("?{query}"));
    }
    if let Some((address, fragment)) = locator.split_once('#') {
        return (address, format!("#{fragment}"));
    }
    (locator, String::new())
}

/// Remove every face at the end of the path and report the one the URL
/// actually ended in. The segments that remain keep the provider's own
/// prefix — `/api/v1/chat/completions` leaves `api/v1`.
///
/// Stripping repeats because the result is entered again the next time
/// the person edits the form: if `/v1/messages/responses` kept its
/// `messages`, a second pass would strip that too and answer a
/// different dialect for the same endpoint.
fn strip_face<'p>(segments: &[&'p str]) -> (Vec<&'p str>, DialectHint) {
    let mut kept: &[&'p str] = segments;
    let mut face = DialectHint::Unset;
    loop {
        let (rest, found) = strip_one_face(kept);
        match found {
            DialectHint::Unset => return (kept.to_vec(), face),
            DialectHint::Chat | DialectHint::Responses | DialectHint::Messages => {
                if face == DialectHint::Unset {
                    face = found;
                }
                kept = rest;
            }
        }
    }
}

/// One face removed from the end of the path, or the path unchanged.
fn strip_one_face<'s, 'p>(segments: &'s [&'p str]) -> (&'s [&'p str], DialectHint) {
    match segments.split_last() {
        Some((&"completions", head)) => match head.split_last() {
            Some((&"chat", rest)) => (rest, DialectHint::Chat),
            _ => (segments, DialectHint::Unset),
        },
        Some((&"responses", head)) => (head, DialectHint::Responses),
        Some((&"messages", head)) => (head, DialectHint::Messages),
        _ => (segments, DialectHint::Unset),
    }
}

/// The path of the stored base URL, with no trailing slash.
///
/// A path the person entered stands as entered. An empty one is filled
/// from the preset table, because the hosts the city knows do not all
/// serve at `/v1`. A URL that carries a query but no path is left
/// path-less: the query already says which endpoint it means.
fn resolve_path(kept: &[&str], suffix: &str, host: &str, presets: &dyn HostDefaults) -> String {
    let entered = kept.join("/");
    if !entered.is_empty() {
        return format!("/{entered}");
    }
    if !suffix.is_empty() {
        return String::new();
    }
    match presets.default_path(host) {
        Some(preset) => format!("/{}", preset.trim_matches('/')),
        None => UNLISTED_HOST_PATH.to_owned(),
    }
}

/// Which shape wins: the pasted URL, then the person's choice, then the
/// preset table. The person is asked only when all three are silent.
fn resolve_dialect(
    from_url: DialectHint,
    hint: DialectHint,
    host: &str,
    presets: &dyn HostDefaults,
) -> DialectHint {
    match (from_url, hint) {
        (DialectHint::Unset, DialectHint::Unset) => match presets.default_dialect(host) {
            Some(preset) => preset,
            None => DialectHint::Unset,
        },
        (DialectHint::Unset, chosen) => chosen,
        (from_url, _) => from_url,
    }
}

/// Lower-case the host of an authority and leave any credentials in it
/// alone: a host is case-insensitive and a password is not.
fn lowercase_host(authority: &str) -> String {
    match authority.rsplit_once('@') {
        Some((userinfo, hostport)) => format!("{userinfo}@{}", hostport.to_ascii_lowercase()),
        None => authority.to_ascii_lowercase(),
    }
}

#[cfg(test)]
mod tests;
