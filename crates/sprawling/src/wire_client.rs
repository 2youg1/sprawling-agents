// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The second client of `channels::wire` (sprawling-SPEC.md section
//! 8-10).
//!
//! ARCHITECTURE section 8 says the wire is the whole API and that a
//! second client writes against it. Until this existed there was one
//! client, which by the repository's own test (section 4: one adapter is
//! a hypothetical seam, two make it real) left `channels::wire` a
//! hypothetical seam.
//!
//! The handshake is computed here from `channels::WIRE_V` and
//! `channels::schema_hash()` rather than copied, so a command renamed in
//! `wire.rs` cannot leave this client behind.

use futures_util::{SinkExt, StreamExt};
use kernel::{AxCode, AxError};
use std::time::Duration;
use tokio_tungstenite::tungstenite::Message;

/// What came back before the city went quiet.
pub(crate) struct Heard {
    pub(crate) frames: u32,
    pub(crate) refusals: u32,
    /// Frames that arrived *after* the frame was sent.
    ///
    /// Counted apart from `frames` because the handshake's `Welcome` is
    /// a frame too, so `frames` is never zero and cannot tell silence
    /// from an answer. Subtracting one at the caller would copy the
    /// shape of the handshake into a second place.
    pub(crate) answers: u32,
}

/// What the city did about the frame it was sent. Exhaustive: these
/// three are what a caller can learn inside one quiet window, and the
/// exit code `sprawling call` returns is one per arm.
pub(crate) enum Spoken {
    /// The city refused, inside the window.
    Refused,
    /// The city answered, inside the window, and refused nothing.
    Answered,
    /// The frame went out and nothing came back before the window
    /// closed. Whether the city took the work is not knowable here, so
    /// this is neither of the other two.
    Quiet,
}

impl Heard {
    /// Refusal first, because a refusal that also carried events is
    /// still a refusal; silence last, because it is only silence when
    /// nothing at all arrived.
    pub(crate) fn spoken(&self) -> Spoken {
        match (self.refusals, self.answers) {
            (0, 0) => Spoken::Quiet,
            (0, _) => Spoken::Answered,
            _ => Spoken::Refused,
        }
    }
}

/// Splits `realm/name` into its two halves.
///
/// Fail-closed on anything else: a reference with no realm, an empty
/// half, or a second slash is not a credential name this city can hold,
/// and guessing which half was meant would put a key under a name its
/// owner did not choose.
pub(crate) fn split_reference(raw: &str) -> Option<(&str, &str)> {
    let (realm, name) = raw.split_once('/')?;
    if realm.is_empty() || name.is_empty() || name.contains('/') {
        return None;
    }
    Some((realm, name))
}

/// The greeting this build sends, computed rather than transcribed.
fn hello(token: Option<&str>) -> channels::ClientFrame {
    channels::ClientFrame::Hello(channels::Hello {
        wire_v: channels::WIRE_V,
        schema: channels::schema_hash(),
        token: token.map(str::to_owned),
    })
}

fn unreachable_city(at: &str, why: &str) -> AxError {
    AxError::failure(
        AxCode::Provider,
        "reach the city's control surface",
        format!("ws://{at}/ws: {why}"),
    )
    .with_recovery("start it with `sprawling up <dir>`, or pass --at host:port")
}

fn malformed(what: &str, why: &str) -> AxError {
    AxError::failure(AxCode::WireMismatch, what, why.to_owned()).with_recovery(
        "a frame is one JSON object: {\"command\":{\"dispatch\":{..}}} or \
         {\"query\":\"city_view\"}; `sprawling call` with no frame lists every name",
    )
}

/// Sends one frame and prints every frame that comes back, as one JSON
/// object per line, until nothing has arrived for `quiet`.
///
/// Quiet is a duration rather than a frame count because how many events
/// one dispatch produces is the city's business, not this client's.
///
/// # Errors
/// Fails when the city cannot be reached, when the greeting is refused,
/// or when the frame is not one this wire can carry. A refusal of the
/// frame itself is not an error here - it is the answer, and it is
/// printed like any other - but it does change the exit code, and so
/// does hearing nothing at all: see [`Spoken`].
pub(crate) fn call(
    at: &str,
    frame: &str,
    token: Option<&str>,
    quiet: Duration,
) -> Result<Heard, AxError> {
    // The frame is parsed before the socket is opened: a typo should
    // cost nothing and should be reported against the text a person
    // wrote, not against whatever the server made of it.
    let outgoing: channels::ClientFrame = serde_json::from_str(frame)
        .map_err(|err| malformed("read the frame to send", &err.to_string()))?;
    let body = serde_json::to_string(&outgoing)
        .map_err(|err| malformed("encode the frame to send", &err.to_string()))?;
    let greeting = serde_json::to_string(&hello(token))
        .map_err(|err| malformed("encode the greeting", &err.to_string()))?;

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|err| {
            AxError::failure(
                AxCode::StorageFatal,
                "start the async runtime",
                err.to_string(),
            )
        })?;
    runtime.block_on(converse(at, &greeting, &body, quiet))
}

async fn converse(at: &str, greeting: &str, body: &str, quiet: Duration) -> Result<Heard, AxError> {
    let url = format!("ws://{at}/ws");
    let (mut socket, _) = tokio_tungstenite::connect_async(&url)
        .await
        .map_err(|err| unreachable_city(at, &err.to_string()))?;
    socket
        .send(Message::Text(greeting.into()))
        .await
        .map_err(|err| unreachable_city(at, &err.to_string()))?;

    let mut heard = Heard {
        frames: 0,
        refusals: 0,
        answers: 0,
    };
    // The greeting is answered before anything else is sent: a client
    // that shouted its command at a server which then refused the
    // handshake would have to guess whether the command was seen.
    let welcome = next_frame(&mut socket, quiet).await?;
    match welcome {
        Some(text) => {
            report(&text, &mut heard);
            if heard.refusals > 0 {
                return Ok(heard);
            }
        }
        None => return Err(unreachable_city(at, "no answer to the greeting")),
    }

    socket
        .send(Message::Text(body.into()))
        .await
        .map_err(|err| unreachable_city(at, &err.to_string()))?;
    while let Some(text) = next_frame(&mut socket, quiet).await? {
        report(&text, &mut heard);
        heard.answers = heard.answers.saturating_add(1);
    }
    // Closing rather than dropping: a city that is told the peer has
    // gone stops holding a session open for it.
    let _closed = socket.close(None).await;
    Ok(heard)
}

/// One text frame, or `None` once the city has been quiet long enough.
async fn next_frame<S>(socket: &mut S, quiet: Duration) -> Result<Option<String>, AxError>
where
    S: StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin,
{
    loop {
        let Ok(incoming) = tokio::time::timeout(quiet, socket.next()).await else {
            return Ok(None);
        };
        match incoming {
            Some(Ok(Message::Text(text))) => return Ok(Some(text.to_string())),
            // A close or a stream end is the city going quiet for good.
            Some(Ok(Message::Close(_))) | None => return Ok(None),
            // Pings and binary frames are the transport's, not the
            // wire's; tungstenite answers pings itself.
            Some(Ok(_)) => {}
            Some(Err(err)) => {
                return Err(AxError::failure(
                    AxCode::WireMismatch,
                    "read a frame from the city",
                    err.to_string(),
                )
                .with_recovery("the connection ended mid-frame; run the command again"));
            }
        }
    }
}

/// Prints one frame and counts it.
///
/// Printed as it arrived rather than reformatted: inventing a display
/// form here would be a second, drifting description of every type on
/// the wire.
fn report(text: &str, heard: &mut Heard) {
    println!("{text}");
    heard.frames = heard.frames.saturating_add(1);
    // Read back through the wire's own type rather than by looking for
    // a word in the text, so a payload that merely mentions refusal is
    // not counted as one.
    if let Ok(channels::ServerFrame::Refusal(_)) =
        serde_json::from_str::<channels::ServerFrame>(text)
    {
        heard.refusals = heard.refusals.saturating_add(1);
    }
}

/// Hands a credential to the local enrolment route and returns the
/// reference that replaces it.
///
/// The value never travels through `argv`, which is readable in the
/// process table, in shell history, and in whatever started this
/// process. That is what makes this better custody than the browser
/// path, where the page holds the plaintext first.
///
/// # Errors
/// Fails when the route refuses - which it does for any peer that is not
/// on this machine - or when the city cannot be reached.
pub(crate) fn enrol(at: &str, realm: &str, name: &str, value: &str) -> Result<String, AxError> {
    let body = serde_json::json!({ "realm": realm, "name": name, "value": value });
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(20))
        // The city is on this machine, and a proxy in front of loopback
        // answers for something else.
        .no_proxy()
        .build()
        .map_err(|err| {
            AxError::failure(AxCode::Provider, "build an http client", err.to_string())
        })?;
    let answer = client
        .post(format!("http://{at}/enroll"))
        .json(&body)
        .send()
        .map_err(|err| unreachable_city(at, &err.to_string()))?;
    let status = answer.status();
    // The reply body is this city's own text, so quoting it is quoting
    // ourselves rather than a stranger's error page.
    let said = answer.text().unwrap_or_default();
    match status.as_u16() {
        201 => Ok(said),
        // The city took it and has not said what became of it. Reported
        // as a failure because the caller must not go on as though the
        // reference resolves - but with the city's own words, which say
        // what to check rather than what went wrong.
        202 => Err(
            AxError::failure(AxCode::CredentialMissing, "enrol a credential", said).with_recovery(
                "the city is busy; ask it again once the run it is inside has finished",
            ),
        ),
        code => Err(AxError::failure(
            AxCode::CredentialMissing,
            "enrol a credential",
            format!("the city answered {code}: {said}"),
        )
        .with_recovery("enrolment is refused for any peer that is not on this machine")),
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::{Duration, Message, SinkExt, Spoken, StreamExt, hello, split_reference};

    /// A city that answers the greeting and then says nothing is the
    /// third answer, and it must not read as the first.
    ///
    /// The defect this pins: `call` exited 0 whenever no refusal
    /// arrived inside the quiet window, while its own documentation
    /// promised 1 means "the city refused". An agent branching on the
    /// exit code read a refusal it never received as a success - which
    /// the out-of-tree checker measured against a real city
    /// (`adversary/adversary-SPEC.md` section 4).
    #[test]
    fn a_city_that_says_nothing_inside_the_window_is_not_a_success() {
        let (ready, port) = std::sync::mpsc::channel();
        let scripted = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            runtime.block_on(async move {
                let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
                ready.send(listener.local_addr().unwrap().port()).unwrap();
                let (stream, _from) = listener.accept().await.unwrap();
                // The token's reason is that a WebSocket client can only be
                // talking to our server. This accepts rather than connects: the
                // double is the city, the client under test is ours, and the
                // socket therefore points inward - the direction the gate says
                // it measures.
                // boundary-ok: accept_async makes this a city double, not a client of one
                let mut socket = tokio_tungstenite::accept_async(stream).await.unwrap();
                // The greeting is answered, because a client that never
                // got a Welcome reports an unreachable city instead.
                let _greeting = socket.next().await;
                let welcome =
                    serde_json::to_string(&channels::ServerFrame::Welcome(channels::Welcome {
                        wire_v: channels::WIRE_V,
                        schema: channels::schema_hash(),
                        resume_from: None,
                        city: None,
                    }))
                    .unwrap();
                socket.send(Message::Text(welcome.into())).await.unwrap();
                // ...and then it says nothing at all about the frame
                // that follows, which is the whole scenario.
                tokio::time::sleep(Duration::from_millis(1_500)).await;
            });
        });

        let at = format!("127.0.0.1:{}", port.recv().unwrap());
        let heard = super::call(
            &at,
            "{\"query\":\"city_view\"}",
            None,
            Duration::from_millis(200),
        )
        .unwrap();
        assert_eq!(heard.answers, 0, "nothing came back after the frame");
        assert_eq!(heard.refusals, 0, "and nothing was refused either");
        assert!(
            matches!(heard.spoken(), Spoken::Quiet),
            "silence is its own answer, not the answer 'accepted'"
        );
        let _joined = scripted.join();
    }

    #[test]
    fn a_reference_is_a_realm_and_a_name() {
        assert_eq!(
            split_reference("modelscope/api"),
            Some(("modelscope", "api"))
        );
    }

    #[test]
    fn anything_that_is_not_two_halves_is_refused_rather_than_guessed() {
        for raw in ["api", "/api", "modelscope/", "", "a/b/c"] {
            assert_eq!(split_reference(raw), None, "{raw} was accepted");
        }
    }

    /// The whole reason this module exists: the greeting is derived from
    /// `channels`, so there is no second place where the wire version or
    /// the schema hash is written down. The probe this replaces had both
    /// copied out into a file outside the workspace.
    #[test]
    fn the_greeting_is_this_build_s_own_and_not_a_transcription() {
        let channels::ClientFrame::Hello(said) = hello(None) else {
            panic!("a greeting is a Hello");
        };
        assert_eq!(said.wire_v, channels::WIRE_V);
        assert_eq!(said.schema, channels::schema_hash());
        assert_eq!(said.token, None);
    }

    #[test]
    fn a_token_travels_when_one_was_given() {
        let channels::ClientFrame::Hello(said) = hello(Some("pair-me")) else {
            panic!("a greeting is a Hello");
        };
        assert_eq!(said.token.as_deref(), Some("pair-me"));
    }
}
