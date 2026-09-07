// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Enrolment: the one credential that never becomes a command.

/// What the enrolment route answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Enrolment {
    /// The reference to put in the attach form. The credential itself
    /// is now in the vault and will not be seen again.
    Stored { reference: String },
    /// The server refused, in its own words. Shown rather than
    /// paraphrased: a refusal from a tunnelled session names the one
    /// machine that can do this instead.
    Refused { reason: String },
}

/// Sends one credential to this city's enrolment route.
///
/// It goes over HTTP rather than the socket because the socket's frame
/// type cannot spell a credential. Both halves are needed: the frame is
/// unspellable, and this route only answers a caller on the machine
/// running the city.
///
/// The value is never held by this module beyond the send, and never
/// enters a frame, a snapshot, or a log line.
#[cfg(target_arch = "wasm32")]
pub fn enrol(realm: &str, name: &str, value: &str, on_done: impl FnOnce(Enrolment) + 'static) {
    use wasm_bindgen::JsCast;
    use wasm_bindgen::closure::Closure;

    let Some(url) = enrol_url() else {
        on_done(Enrolment::Refused {
            reason: "this page has no origin to enrol against".to_owned(),
        });
        return;
    };
    let Ok(request) = web_sys::XmlHttpRequest::new() else {
        on_done(Enrolment::Refused {
            reason: "this browser refused to make the request".to_owned(),
        });
        return;
    };
    if request.open_with_async("POST", &url, true).is_err() {
        on_done(Enrolment::Refused {
            reason: format!("{url} could not be opened"),
        });
        return;
    }
    let body = serde_json::json!({ "realm": realm, "name": name, "value": value }).to_string();
    let reference = format!("secret:{realm}/{name}");
    let handle = request.clone();
    let mut finish = Some(on_done);
    let settled = Closure::<dyn FnMut()>::new(move || {
        if handle.ready_state() != web_sys::XmlHttpRequest::DONE {
            return;
        }
        let Some(finish) = finish.take() else {
            return;
        };
        let answer = match handle.status() {
            Ok(201) => Enrolment::Stored {
                reference: reference.clone(),
            },
            _ => Enrolment::Refused {
                reason: handle
                    .response_text()
                    .ok()
                    .flatten()
                    .unwrap_or_else(|| "the city did not say why".to_owned()),
            },
        };
        finish(answer);
    });
    request.set_onreadystatechange(Some(settled.as_ref().unchecked_ref()));
    settled.forget();
    if request.send_with_opt_str(Some(&body)).is_err() {
        // The callback above will not fire, so the caller hears it here.
        request.set_onreadystatechange(None);
    }
}

/// The enrolment route on this page's own origin.
#[cfg(target_arch = "wasm32")]
fn enrol_url() -> Option<String> {
    let location = web_sys::window()?.location();
    Some(format!(
        "{}//{}/enroll",
        location.protocol().ok()?,
        location.host().ok()?
    ))
}

/// Off the browser there is nowhere to send it, and saying so is the
/// honest answer: the page shows this refusal rather than appearing to
/// have stored something.
#[cfg(not(target_arch = "wasm32"))]
pub fn enrol(_realm: &str, _name: &str, _value: &str, on_done: impl FnOnce(Enrolment) + 'static) {
    on_done(Enrolment::Refused {
        reason: "this client is not running in a browser".to_owned(),
    });
}
