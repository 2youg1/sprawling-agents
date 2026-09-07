// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The keyboard: the one place a keystroke reaches this client.

use dioxus::prelude::*;

use crate::route::View;
#[cfg(target_arch = "wasm32")]
use crate::route::place_view;

/// What a keystroke may move. Bundled for the reason [`Wiring`] is: a
/// listener that took six handles would grow a seventh without anybody
/// noticing which of them it actually writes.
#[cfg_attr(
    not(target_arch = "wasm32"),
    expect(
        dead_code,
        reason = "the only reader is the browser's keydown listener"
    )
)]
#[derive(Clone, Copy)]
pub(crate) struct Keyboard {
    pub(crate) chord: Signal<crate::keys::Chord>,
    pub(crate) palette: Signal<bool>,
    pub(crate) keymap: Signal<bool>,
    pub(crate) view: Signal<View>,
    pub(crate) refused: Signal<Option<crate::alert::Refused>>,
}

/// The one place a keystroke reaches this client.
///
/// On the window rather than on an element: a person who has clicked
/// nothing still has a keyboard, and a handler hung on the layout would
/// never see a key pressed while the body itself holds focus.
///
/// The browser contributes three facts and no judgement - which key,
/// whether the accelerator was down, and whether focus sits in something
/// the reader types into - and `web::keys` decides the rest, which is what
/// keeps the key map testable on the host.
#[cfg(target_arch = "wasm32")]
pub(crate) fn listen_for_keys(keyboard: Keyboard) {
    use dioxus::prelude::use_hook;
    use wasm_bindgen::JsCast as _;
    use_hook(move || {
        let Some(window) = web_sys::window() else {
            return;
        };
        let mut held = keyboard;
        let pressed = wasm_bindgen::closure::Closure::<dyn FnMut(web_sys::KeyboardEvent)>::new(
            move |event: web_sys::KeyboardEvent| {
                let key = event.key();
                let stroke = crate::keys::Stroke {
                    key: &key,
                    command: event.ctrl_key() || event.meta_key(),
                    in_text: typing_now(),
                };
                // `peek` rather than a read: this closure lives outside
                // the render that created it, and subscribing here would
                // tie a DOM listener to a reactive scope it outlives.
                let (next, act) = crate::keys::press(*held.chord.peek(), &stroke);
                held.chord.set(next);
                match act {
                    crate::keys::Act::Ignore => return,
                    crate::keys::Act::OpenPalette => {
                        held.keymap.set(false);
                        held.palette.set(true);
                    }
                    // One key closes whatever is open, outermost first, so
                    // a reader never has to know how deep they are.
                    crate::keys::Act::Dismiss => {
                        held.palette.set(false);
                        held.keymap.set(false);
                        held.refused.set(None);
                    }
                    crate::keys::Act::Compose => {
                        held.palette.set(false);
                        focus_where_work_starts();
                    }
                    crate::keys::Act::ShowKeys => {
                        held.keymap.set(true);
                    }
                    crate::keys::Act::Go(place) => {
                        held.palette.set(false);
                        held.keymap.set(false);
                        let going = place_view(place);
                        crate::route::go(&going);
                        held.view.set(going);
                    }
                }
                // Only what this client claimed: an ignored key belongs to
                // the browser, and taking it would break the reader's own
                // find-in-page and text entry.
                event.prevent_default();
            },
        );
        if window
            .add_event_listener_with_callback("keydown", pressed.as_ref().unchecked_ref())
            .is_ok()
        {
            // The page outlives the listener; dropping the closure here
            // would unregister the only thing that reads the keyboard.
            pressed.forget();
        }
    });
}

/// Whether focus sits in something the reader is writing into.
///
/// Without this, writing the word "goal" into the task box would navigate
/// away on its `g`.
#[cfg(target_arch = "wasm32")]
fn typing_now() -> bool {
    let Some(active) = web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.active_element())
    else {
        return false;
    };
    matches!(
        active.tag_name().to_ascii_uppercase().as_str(),
        "INPUT" | "TEXTAREA" | "SELECT"
    ) || active.has_attribute("contenteditable")
}

/// Puts the cursor in the box work is described in.
///
/// The discarded result follows `route::go`: a focus call that the
/// document refuses has no second thing to try, and the page is already
/// showing the field it failed to reach.
#[cfg(target_arch = "wasm32")]
fn focus_where_work_starts() {
    use wasm_bindgen::JsCast as _;
    if let Some(field) = web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.get_element_by_id("dispatch-task"))
        .and_then(|found| found.dyn_into::<web_sys::HtmlElement>().ok())
    {
        let _ = field.focus();
    }
}

/// Off the browser there is no keyboard to listen to.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn listen_for_keys(_keyboard: Keyboard) {}
