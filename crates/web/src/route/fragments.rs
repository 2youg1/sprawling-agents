// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The fragments: one spelling written, every old spelling read.

use channels::{Address, RunId};

use super::view::{Lens, View};

/// The address-bar form of a view, fragment marker included.
///
/// Always begins `#/`, so a fragment written by hand and one written
/// here are the same string. Each view has exactly one spelling: the
/// older spellings resolve in [`from_fragment`] and are never produced,
/// which is what keeps the address bar from teaching two names for one
/// place.
#[must_use]
pub fn to_fragment(view: &View) -> String {
    match view {
        View::Sessions => "#/".to_owned(),
        View::Session(addr) => format!("#/s/{}", addr.as_str()),
        View::Waiting => "#/waiting".to_owned(),
        View::Record(Lens::Ledger) => "#/record".to_owned(),
        View::Record(Lens::Archive) => "#/record/archive".to_owned(),
        View::Record(Lens::Bin) => "#/record/bin".to_owned(),
        View::Cost => "#/cost".to_owned(),
        View::Setup => "#/setup".to_owned(),
        View::Building(addr) => format!("#/b/{}", addr.as_str()),
        View::Run(run) => format!("#/live/{run}"),
    }
}

/// The view a fragment names, or `None` when it names nothing.
///
/// `None` rather than a silent fall back to the first page: a link that
/// does not resolve is a fact the caller may want to say something
/// about, and a router that quietly lands somewhere else teaches people
/// their bookmarks are unreliable without ever admitting it.
#[must_use]
pub fn from_fragment(raw: &str) -> Option<View> {
    let path = raw.trim_start_matches('#').trim_start_matches('/');
    let (head, tail) = path.split_once('/').unwrap_or((path, ""));
    match (head, tail) {
        ("", "") => Some(View::Sessions),
        ("s", addr) => Address::parse(addr).ok().map(View::Session),
        ("waiting", "") => Some(View::Waiting),
        ("record", "") => Some(View::Record(Lens::Ledger)),
        ("record", "archive") => Some(View::Record(Lens::Archive)),
        ("record", "bin") => Some(View::Record(Lens::Bin)),
        ("cost", "") => Some(View::Cost),
        ("setup", "") => Some(View::Setup),
        ("b", addr) => Address::parse(addr).ok().map(View::Building),
        ("live", run) if !run.is_empty() => RunId::parse(run).ok().map(View::Run),

        // Everything below this line is a spelling this build no longer
        // writes. Each one lands on the page that inherited its
        // question, and the three that had a page of their own became
        // one lens each of the record.
        ("overview", "") | ("city", "") | ("live", "") => Some(View::Sessions),
        ("approvals", "") => Some(View::Waiting),
        ("ledger", "") => Some(View::Record(Lens::Ledger)),
        ("archive", "") => Some(View::Record(Lens::Archive)),
        ("recycle-bin", "") => Some(View::Record(Lens::Bin)),
        ("dashboard", "") => Some(View::Cost),
        ("settings", "") => Some(View::Setup),
        ("building", addr) => Address::parse(addr).ok().map(View::Building),
        _ => None,
    }
}

/// What the address bar says, when this build cannot resolve it.
///
/// `None` for an empty fragment, which is the first page rather than a
/// broken link. Separate from [`current`] because the two answers are
/// different questions: one asks where to go, the other asks what to say
/// about not going anywhere.
#[cfg(target_arch = "wasm32")]
#[must_use]
pub fn unresolved() -> Option<String> {
    let hash = web_sys::window()?.location().hash().ok()?;
    let named = hash.trim_start_matches('#').trim_start_matches('/');
    if named.is_empty() || from_fragment(&hash).is_some() {
        return None;
    }
    Some(format!("#/{named}"))
}

/// Reads the address bar. `None` when there is no browser to read.
#[cfg(target_arch = "wasm32")]
#[must_use]
pub fn current() -> Option<View> {
    let hash = web_sys::window()?.location().hash().ok()?;
    from_fragment(&hash)
}

/// Puts a view in the address bar without reloading the page.
///
/// Writing the fragment is the only way a view changes: the listener on
/// `hashchange` is what moves the signal, so a click, an `<a href>` and
/// the browser's own back button take the same path and cannot disagree.
#[cfg(target_arch = "wasm32")]
pub fn go(view: &View) {
    let Some(window) = web_sys::window() else {
        return;
    };
    // Assigning the hash is what pushes a history entry; `replace` would
    // make the back button skip the page a person just left.
    let _ = window.location().set_hash(&to_fragment(view));
}
