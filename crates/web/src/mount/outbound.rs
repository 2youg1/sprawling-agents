// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The outbound: the one way a component reaches the server.

/// The one way a component reaches the server. A frame sent before the
/// socket exists is dropped rather than queued: the page that sent it
/// asks again, and a queue would be a second place where "what did the
/// person ask for" lives.
#[cfg(target_arch = "wasm32")]
pub(crate) fn send_through(outbound: OutboundCell) -> Outbound {
    Outbound(outbound)
}

/// The socket handle as the component tree may hold it: cloneable,
/// because a hook's value is cloned on every render.
#[cfg(target_arch = "wasm32")]
type OutboundCell = std::rc::Rc<
    std::cell::RefCell<Option<std::rc::Rc<std::cell::RefCell<Option<web_sys::WebSocket>>>>>,
>;

#[cfg(target_arch = "wasm32")]
#[derive(Clone)]
pub struct Outbound(OutboundCell);

#[cfg(target_arch = "wasm32")]
impl Outbound {
    pub(crate) fn call(&self, frame: channels::ClientFrame) {
        let held = self.0.borrow();
        let Some(socket) = held.as_ref() else {
            return;
        };
        let socket = socket.borrow();
        if let Some(socket) = socket.as_ref() {
            let _ = crate::socket::send(socket, &frame);
        }
    }
}

/// Off the browser there is no socket, so a frame goes nowhere. The
/// type exists on both targets because the component tree names it.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone)]
pub struct Outbound;

#[cfg(not(target_arch = "wasm32"))]
impl Outbound {
    pub(crate) fn call(&self, _frame: channels::ClientFrame) {}
}
