// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The notification: the interruption that reaches another tab.

#[cfg(target_arch = "wasm32")]
use crate::lang::Lang;

#[cfg(target_arch = "wasm32")]
use super::judge::Alert;

#[cfg(target_arch = "wasm32")]
use crate::lang::say;
/// Asks the browser, once, whether it may interrupt.
///
/// Fire and forget: the answer arrives later, and until it is granted
/// [`interrupt`] does nothing. A page that blocked on this would be
/// holding up the city for a permission dialog.
#[cfg(target_arch = "wasm32")]
pub fn ask_to_interrupt() {
    if web_sys::Notification::permission() == web_sys::NotificationPermission::Default {
        let _ = web_sys::Notification::request_permission();
    }
}

/// Raises one browser notification.
///
/// Zero judgement: whether to interrupt was decided by [`Alerts::raise`],
/// which is the only thing that knows whether this fact already made a
/// claim on somebody's attention. Nothing here can fail in a way a person
/// could act on - a browser that refuses notifications has said its piece
/// - so nothing is returned.
#[cfg(target_arch = "wasm32")]
pub fn interrupt(lang: Lang, alert: &Alert) {
    if web_sys::Notification::permission() != web_sys::NotificationPermission::Granted {
        return;
    }
    let options = web_sys::NotificationOptions::new();
    options.set_body(&alert.message);
    // The tag is the alert key, so a browser that is still showing this
    // fact replaces it rather than stacking a second copy.
    options.set_tag(&alert.key);
    let _ = web_sys::Notification::new_with_options(say(lang, alert.kind.title()), &options);
}
