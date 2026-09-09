// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The shell: socket, starting the client, and its theme.

#[cfg(target_arch = "wasm32")]
use dioxus::prelude::*;

#[cfg(target_arch = "wasm32")]
use super::frame::{FrameWiring, apply_frame};
#[cfg(target_arch = "wasm32")]
use super::outbound::Outbound;
#[cfg(target_arch = "wasm32")]
use super::outbound::send_through;
#[cfg(target_arch = "wasm32")]
use super::wiring::Wiring;
#[cfg(target_arch = "wasm32")]
use crate::shell::App;

#[cfg(target_arch = "wasm32")]
pub(crate) fn connect(wiring: Wiring) -> Outbound {
    use dioxus::prelude::use_hook;

    // Only two of these move here now. The rest are copied into the
    // frame's own wiring and moved once per animation frame instead of
    // once per arriving message, which is the whole of what this loop
    // changed (`web::pace`).
    let Wiring {
        mut snapshot,
        endpoints,
        city,
        cost,
        building,
        discards,
        inbox,
        hits,
        filed,
        vitals,
        changes,
        rounds,
        records,
        mut live,
        view,
        expecting,
        refused,
        lang,
    } = wiring;
    use_hook(move || {
        let outbound = std::rc::Rc::new(std::cell::RefCell::new(None));
        let Some(url) = crate::socket::socket_url() else {
            return send_through(outbound);
        };
        // The code the host put on this URL. Hard-coded `None` here made
        // every exposed city unreachable by its own WebUI: the server
        // asked for a token and the page had no way to have one.
        let link = std::rc::Rc::new(std::cell::RefCell::new(crate::socket::Link::new(
            crate::socket::pairing_token(),
        )));
        // What has already claimed somebody's attention. Held beside the
        // link because a reconnect re-delivers events, and one fact must
        // not interrupt twice for having been sent twice.
        let alerts = std::rc::Rc::new(std::cell::RefCell::new(crate::alert::Alerts::new()));
        let socket = std::rc::Rc::new(std::cell::RefCell::new(None));
        // Where frames wait for the next animation frame. A run does not
        // deliver one event at a time - a tool wave writes five in a few
        // milliseconds - and applying each on arrival repainted the page
        // once per event at whatever rate the network chose. A display
        // cannot show more than one frame per refresh, so those paints
        // were work produced for nobody (`web::pace`).
        let buffer = crate::pace::browser::Buffer::default();
        {
            let buffer_for_loop = buffer.clone();
            let alerts = std::rc::Rc::clone(&alerts);
            let socket = std::rc::Rc::clone(&socket);
            crate::pace::browser::each_frame(buffer_for_loop, move |paint| {
                apply_frame(
                    paint,
                    &socket,
                    &alerts,
                    FrameWiring {
                        snapshot,
                        endpoints,
                        city,
                        cost,
                        building,
                        discards,
                        inbox,
                        hits,
                        filed,
                        vitals,
                        changes,
                        rounds,
                        records,
                        view,
                        expecting,
                        refused,
                        lang,
                    },
                );
            });
        }
        let opened = {
            let link = std::rc::Rc::clone(&link);
            let socket = std::rc::Rc::clone(&socket);
            let buffer = buffer.clone();
            crate::socket::open(&url, move |event| {
                let action = match link.try_borrow_mut() {
                    Ok(mut link) => link.advance(event),
                    Err(_) => return,
                };
                // The pages watch this to know when asking is worth
                // anything. Read from the link rather than inferred from
                // the action, because the link owns what "live" means.
                let flowing = link.try_borrow().is_ok_and(|link| link.is_live());
                let opened = flowing && !*live.peek();
                if *live.peek() != flowing {
                    live.set(flowing);
                }
                if let Ok(link) = link.try_borrow()
                    && let Some(city) = link.city()
                    && snapshot.peek().city() != Some(city)
                {
                    snapshot.write().adopt_city(city.clone());
                }
                let held = socket.borrow();
                let Some(socket) = held.as_ref() else {
                    return;
                };
                // The stream carries what happens next, so a tab opened
                // over a city that has been running for a month saw an
                // empty one. Asked once, the moment frames start
                // flowing, and before any live record can have been
                // folded - which is the condition `backfill` refuses to
                // work without.
                if opened {
                    let _ = crate::socket::send(
                        socket,
                        &channels::ClientFrame::Query(channels::Query::History {
                            before: None,
                            limit: channels::HISTORY_MAX,
                        }),
                    );
                }
                match action {
                    crate::socket::LinkAction::Send(hello) => {
                        let _ = crate::socket::send(socket, &channels::ClientFrame::Hello(*hello));
                    }
                    // The three actions that change the page do not change
                    // it here. They go into the buffer and the animation
                    // frame applies them together, because the rate a
                    // network delivers at is not a rate a display can show
                    // (`web::pace`).
                    crate::socket::LinkAction::Deliver(event) => {
                        buffer.push(crate::pace::Arrived::Event(event));
                    }
                    crate::socket::LinkAction::Answered(answer) => {
                        buffer.push(crate::pace::Arrived::Answer(answer));
                    }
                    crate::socket::LinkAction::Report(error) => {
                        buffer.push(crate::pace::Arrived::Refusal(error));
                    }
                    crate::socket::LinkAction::Saying(delta) => {
                        buffer.push(crate::pace::Arrived::Saying(delta));
                    }
                    // The retry ladder is not history either, and
                    // closing on the way out of view is the transport
                    // layer's to carry out; here they are the same as
                    // any other instruction that moves no snapshot.
                    crate::socket::LinkAction::WaitMs(_)
                    | crate::socket::LinkAction::OpenSocket
                    | crate::socket::LinkAction::CloseSocket
                    | crate::socket::LinkAction::Nothing => {}
                }
            })
        };
        if let Ok(handle) = opened {
            *socket.borrow_mut() = Some(handle);
            let _ = link.borrow_mut().connect();
        }
        *outbound.borrow_mut() = Some(std::rc::Rc::clone(&socket));
        send_through(outbound)
    })
}

/// Hands the client to the browser. The only wasm-specific entry in this
/// crate, and it decides nothing.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(start)]
pub fn start() {
    install_theme();
    crate::alert::ask_to_interrupt();
    launch(App);
}

/// Writes the token set into the document before the first paint.
///
/// The shipped page names no colour; it reads custom properties that arrive
/// here. That is what makes "one production point for colour" true of what
/// the browser renders and not only of the Rust source.
#[cfg(target_arch = "wasm32")]
fn install_theme() {
    let Some(document) = web_sys::window().and_then(|w| w.document()) else {
        return;
    };
    let Some(head) = document.head() else {
        return;
    };
    let Ok(style) = document.create_element("style") else {
        return;
    };
    style.set_text_content(Some(&crate::theme::custom_properties()));
    let _ = head.append_child(&style);
}
