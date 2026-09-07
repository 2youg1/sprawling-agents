// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The address bar: the one reader.

use dioxus::prelude::*;

#[cfg(target_arch = "wasm32")]
use crate::lang::Msg;

use crate::route::View;

/// Mounts the one reader of the address bar.
///
/// Registered once for the life of the page: `use_hook` runs on the
/// first render only, so the listener is not rebuilt on every state
/// change - a second listener would apply the same change twice.
#[cfg(target_arch = "wasm32")]
pub(crate) fn follow_the_address_bar(
    mut view: Signal<View>,
    mut refused: Signal<Option<crate::alert::Refused>>,
    lang: Signal<crate::lang::Lang>,
) {
    use dioxus::prelude::use_hook;
    use wasm_bindgen::JsCast as _;
    use_hook(move || {
        // What the person arrived at, before any event has happened.
        match crate::route::current() {
            Some(arrived) => view.set(arrived),
            // A link that does not land is a fact the person may want to
            // act on. Leaving them on the first page without a word is the
            // quiet substitution this design refuses: it teaches somebody
            // their own bookmarks are unreliable while never admitting it.
            None => {
                if let Some(named) = crate::route::unresolved() {
                    let said = lang();
                    refused.set(Some(crate::alert::Refused {
                        code: "E_NO_SUCH_PAGE".to_owned(),
                        what: crate::lang::fill(
                            crate::lang::say(said, Msg::RouteNoSuchPage),
                            &[("named", &named)],
                        ),
                        recovery: crate::lang::say(said, Msg::RouteNoSuchPageRecovery).to_owned(),
                    }));
                }
            }
        }
        let Some(window) = web_sys::window() else {
            return;
        };
        let moved = wasm_bindgen::closure::Closure::<dyn FnMut()>::new(move || {
            // A fragment that names nothing leaves the page where it is
            // rather than landing somewhere the person did not ask for.
            if let Some(next) = crate::route::current() {
                view.set(next);
            }
        });
        if window
            .add_event_listener_with_callback("hashchange", moved.as_ref().unchecked_ref())
            .is_ok()
        {
            // The listener outlives this scope, and the page outlives
            // the listener: dropping the closure here would unregister
            // the only thing that reads the address bar.
            moved.forget();
        }
    });
}

/// Off the browser there is no address bar, so the signal is the only
/// authority and nothing has to follow anything.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn follow_the_address_bar(
    _view: Signal<View>,
    _refused: Signal<Option<crate::alert::Refused>>,
    _lang: Signal<crate::lang::Lang>,
) {
}
