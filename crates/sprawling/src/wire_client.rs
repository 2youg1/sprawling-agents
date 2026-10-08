// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The second client of `wire::frames` (`crates/sprawling/Spec.lean` section
//! 8-10).
//!
//! ARCHITECTURE section 8 says the wire is the whole API and that a
//! second client writes against it. Until this existed there was one
//! client, which by the repository's own test (section 4: one adapter is
//! a hypothetical seam, two make it real) left `wire::frames` a
//! hypothetical seam.
//!
//! The handshake is computed here from `wire::WIRE_V` and
//! `wire::schema_hash()` rather than copied, so a command renamed in
//! `frames.rs` cannot leave this client behind.

use futures_util::{SinkExt, StreamExt};
use kernel::{AxCode, AxError};
use std::time::Duration;
use tokio_tungstenite::tungstenite::Message;

/// When a call stops listening is a decision about the wire's frames,
/// with no socket in it, so it has its own file.
mod ending;
/// Enrolment is a credential handed to an HTTP route rather than a
/// frame spoken on the socket, so it has its own file.
mod enrolment;
/// `sprawling gauge --at` holds its socket open until the city stops.
mod watching;

use ending::{Awaited, Echo, Ending, Heard, Reply};
pub(crate) use ending::{Milestone, Spoken, Until};
pub(crate) use enrolment::{enrol, split_reference};
pub(crate) use watching::top;

/// How long a call listens: the silence that ends it, and what it waits
/// for before then. They are read together at every frame, so they
/// travel as one value.
pub(crate) struct Listen {
    pub(crate) quiet: Duration,
    pub(crate) until: Until,
}

/// Why nothing was heard: the three failures `call` tells apart,
/// because each asks the caller for a different next step.
#[derive(Debug)]
pub(crate) enum Unheard {
    /// The frame is not one this wire can carry; nothing was sent.
    Unreadable(AxError),
    /// Nothing at the address completed the greeting, so no city heard
    /// the frame.
    NoCity(AxError),
    /// The conversation started and then broke, or this process could
    /// not start one.
    Broken(AxError),
}

/// The credential a call presents: the token given with `--token`, or
/// else the key the city at `at` keeps in its key file on this machine
/// (`crates/sprawling/spec/Keying.lean` §8-22). A key file that cannot be
/// read is said once and the greeting goes without; the city's refusal
/// then says how to get in.
fn credential(at: &str, given: Option<&str>) -> Option<String> {
    if let Some(given) = given {
        return Some(given.to_owned());
    }
    let port = at
        .rsplit_once(':')
        .and_then(|(_, port)| port.parse::<u16>().ok())?;
    match sprawling::serving::key_file::read_key(port) {
        Ok(key) => key,
        Err(unread) => {
            eprintln!("{}: {}", unread.action(), unread.recovery());
            None
        }
    }
}

/// The greeting this build sends, computed rather than transcribed.
fn hello(token: Option<&str>) -> wire::ClientFrame {
    wire::ClientFrame::Hello(wire::Hello {
        wire_v: wire::WIRE_V,
        schema: wire::schema_hash(),
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
         {\"ask\":{\"ask_id\":1,\"query\":\"city_view\"}}; `sprawling call` with no frame lists every name",
    )
}

/// Sends one frame and prints every frame that comes back, as one JSON
/// object per line, until the frame's [`Ending`]: a query stops on its
/// answer or refusal, a command on what `listen.until` names or once
/// nothing has arrived for `listen.quiet`.
///
/// Quiet is a duration rather than a frame count because how many events
/// one dispatch produces is the city's business, not this client's; for
/// a query it only bounds an answer that never comes.
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
    listen: Listen,
) -> Result<Heard, Unheard> {
    // The frame is parsed before the socket is opened: a typo should
    // cost nothing and should be reported against the text a person
    // wrote, not against whatever the server made of it.
    let outgoing: wire::ClientFrame = serde_json::from_str(frame).map_err(|err| {
        Unheard::Unreadable(malformed("read the frame to send", &err.to_string()))
    })?;
    send(at, &outgoing, token, listen)
}

/// Sends a frame this process built and prints what comes back until
/// the [`Ending`] the frame's kind and `listen.until` decide; [`call`]
/// is this for a frame a person wrote.
///
/// # Errors
/// As [`call`], less the frame this process could not read.
pub(crate) fn send(
    at: &str,
    outgoing: &wire::ClientFrame,
    token: Option<&str>,
    listen: Listen,
) -> Result<Heard, Unheard> {
    let body = serde_json::to_string(outgoing).map_err(|err| {
        Unheard::Unreadable(malformed("encode the frame to send", &err.to_string()))
    })?;
    let greeting = serde_json::to_string(&hello(credential(at, token).as_deref()))
        .map_err(|err| Unheard::Broken(malformed("encode the greeting", &err.to_string())))?;

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|err| {
            Unheard::Broken(
                AxError::failure(
                    AxCode::StorageFatal,
                    "start the async runtime",
                    err.to_string(),
                )
                .with_recovery(
                    "close some programs and try again: this machine would not give the \
                 process the thread the connection needs",
                ),
            )
        })?;
    let mut sending = Sending {
        body,
        ending: Ending::of(outgoing, listen.until),
    };
    runtime.block_on(converse(at, &greeting, &mut sending, listen.quiet))
}

/// The frame on its way out, and when to stop listening for what it
/// brings back; the second is read off the first, so they travel as one.
struct Sending {
    body: String,
    ending: Ending,
}

async fn converse(
    at: &str,
    greeting: &str,
    sending: &mut Sending,
    quiet: Duration,
) -> Result<Heard, Unheard> {
    let url = format!("ws://{at}/ws");
    let no_city = |why: String| Unheard::NoCity(unreachable_city(at, &why));
    let (mut socket, _) = tokio_tungstenite::connect_async(&url)
        .await
        .map_err(|err| no_city(err.to_string()))?;
    socket
        .send(Message::Text(greeting.into()))
        .await
        .map_err(|err| no_city(err.to_string()))?;

    let mut heard = Heard {
        frames: 0,
        refusals: 0,
        answers: 0,
        awaited: sending.ending.not_yet(),
        run: None,
    };
    // The greeting is answered before anything else is sent: a client
    // that shouted its command at a server which then refused the
    // handshake would have to guess whether the command was seen.
    let welcome = next_frame(&mut socket, quiet)
        .await
        .map_err(Unheard::NoCity)?;
    match welcome {
        Some(text) => {
            report(&text, &Reply::of(&text), &sending.ending.echo(), &mut heard);
            if heard.refusals > 0 {
                return Ok(heard);
            }
        }
        None => return Err(no_city("no answer to the greeting".to_owned())),
    }

    socket
        .send(Message::Text(sending.body.as_str().into()))
        .await
        .map_err(|err| Unheard::Broken(unreachable_city(at, &err.to_string())))?;
    while let Some(text) = next_frame(&mut socket, quiet)
        .await
        .map_err(Unheard::Broken)?
    {
        let reply = Reply::of(&text);
        report(&text, &reply, &sending.ending.echo(), &mut heard);
        heard.answers = heard.answers.saturating_add(1);
        if sending.ending.ends_on(&reply) {
            heard.awaited = Awaited::Arrived;
            break;
        }
    }
    heard.run = sending.ending.run();
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
fn report(text: &str, reply: &Reply, echo: &Echo, heard: &mut Heard) {
    match echo {
        Echo::Stdout => println!("{text}"),
        Echo::Stderr => eprintln!("{text}"),
    }
    heard.frames = heard.frames.saturating_add(1);
    match reply {
        Reply::Refusal => heard.refusals = heard.refusals.saturating_add(1),
        Reply::Answer | Reply::Event { .. } | Reply::Other => {}
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::disallowed_methods,
    reason = "test code: how long a call waited is read off the wall clock"
)]
mod tests {
    use super::{Duration, Listen, Message, SinkExt, Spoken, StreamExt, Until, hello};
    use kernel::{EventDraft, EventKind, EventRecord, GENESIS_PREV, Payload, RunId, Seq, TimeMs};

    /// A city double at a fresh port: it answers the greeting with a
    /// `Welcome`, says `said` in order, and then holds the session open
    /// for `linger`, as a served city does, so only the client under
    /// test can end the call early.
    fn city_saying(
        said: Vec<wire::ServerFrame>,
        linger: Duration,
    ) -> (String, std::thread::JoinHandle<()>) {
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
                let welcome = wire::ServerFrame::Welcome(wire::Welcome {
                    wire_v: wire::WIRE_V,
                    schema: wire::schema_hash(),
                    resume_from: None,
                    city: None,
                    epoch: None,
                });
                for frame in std::iter::once(welcome).chain(said) {
                    let text = serde_json::to_string(&frame).unwrap();
                    socket.send(Message::Text(text.into())).await.unwrap();
                }
                tokio::time::sleep(linger).await;
            });
        });
        (format!("127.0.0.1:{}", port.recv().unwrap()), scripted)
    }

    fn event(kind: EventKind) -> wire::ServerFrame {
        let draft = EventDraft {
            run: RunId::CITY,
            t: TimeMs::new(0),
            who: "mason@lab.1".into(),
            addr: None,
            kind,
            data: Payload::new(serde_json::Map::new()).unwrap(),
            ig: false,
        };
        wire::ServerFrame::Event(Box::new(EventRecord::from_draft(
            draft,
            Seq::FIRST,
            GENESIS_PREV,
        )))
    }

    /// A command frame whose consequences are events.
    fn cancelling() -> String {
        let run = serde_json::to_string(&RunId::CITY).unwrap();
        format!(
            "{{\"command\":{{\"cancel\":{{\"run\":{run},             \"idem\":\"idem1-00000000000000000000000000000002\"}}}}}}"
        )
    }

    /// A city that answers the greeting and then says nothing is the
    /// third answer, and it must not read as the first.
    ///
    /// The defect this pins: `call` exited 0 whenever no refusal
    /// arrived inside the quiet window, while its own documentation
    /// promised 1 means "the city refused". An agent branching on the
    /// exit code read a refusal it never received as a success - which
    /// the out-of-tree checker measured against a real city
    /// (`tools/adversary/Spec.lean` section 4).
    #[test]
    fn a_city_that_says_nothing_inside_the_window_is_not_a_success() {
        let (at, scripted) = city_saying(Vec::new(), Duration::from_millis(1_500));
        let listen = Listen {
            quiet: Duration::from_millis(200),
            until: Until::Quiet,
        };
        let heard = super::call(
            &at,
            "{\"ask\":{\"ask_id\":1,\"query\":\"city_view\"}}",
            None,
            listen,
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

    /// A query has exactly one reply, so `call` ends on it rather than
    /// holding the process open for a quiet window that can bring
    /// nothing more: the city's answer arrives in milliseconds, and
    /// waiting out the window would add whole seconds to every query.
    #[test]
    fn a_query_returns_on_its_answer_before_the_quiet_window_ends() {
        let quiet = Duration::from_millis(1_000);
        let answer = wire::ServerFrame::Answered(Box::new(wire::Answered {
            ask_id: wire::AskId(1),
            as_of: kernel::Seq::FIRST,
            outcome: wire::AskOutcome::Answer(wire::Answer::Run(None)),
        }));
        let (at, scripted) = city_saying(vec![answer], Duration::from_millis(1_500));
        let began = std::time::Instant::now();
        let listen = Listen {
            quiet,
            until: Until::Quiet,
        };
        let heard = super::call(
            &at,
            "{\"ask\":{\"ask_id\":1,\"query\":\"city_view\"}}",
            None,
            listen,
        )
        .unwrap();
        let waited = began.elapsed();
        assert!(matches!(heard.spoken(), Spoken::Answered));
        assert!(
            waited < quiet,
            "the answer ends the call; it took {waited:?} against a {quiet:?} window"
        );
        let _joined = scripted.join();
    }

    /// A command told what it waits for ends on that event, after
    /// printing the events before it, instead of on the window.
    #[test]
    fn a_command_returns_on_the_event_it_waits_for_before_the_quiet_window_ends() {
        let quiet = Duration::from_millis(1_000);
        let said = vec![event(EventKind::RunStarted), event(EventKind::RunFrozen)];
        let (at, scripted) = city_saying(said, Duration::from_millis(1_500));
        let began = std::time::Instant::now();
        let listen = Listen {
            quiet,
            until: Until::Event(EventKind::RunFrozen),
        };
        let heard = super::call(&at, &cancelling(), None, listen).unwrap();
        let waited = began.elapsed();
        assert_eq!(heard.answers, 2, "the event before it is printed too");
        assert!(matches!(heard.spoken(), Spoken::Answered));
        assert!(
            waited < quiet,
            "the awaited event ends the call; it took {waited:?} against a {quiet:?} window"
        );
        let _joined = scripted.join();
    }

    /// Events that arrive while the awaited one never does are not the
    /// answer the caller asked for, so the call reads as unfinished: the
    /// work may still be running, and exit 0 would call it done.
    #[test]
    fn a_command_whose_awaited_event_never_comes_is_unfinished_not_answered() {
        let said = vec![event(EventKind::RunStarted)];
        let (at, scripted) = city_saying(said, Duration::from_millis(800));
        let listen = Listen {
            quiet: Duration::from_millis(200),
            until: Until::Event(EventKind::RunFrozen),
        };
        let heard = super::call(&at, &cancelling(), None, listen).unwrap();
        assert_eq!(heard.answers, 1);
        assert!(
            matches!(heard.spoken(), Spoken::Unfinished),
            "the awaited event did not come, so the call is not a success"
        );
        let _joined = scripted.join();
    }

    /// The whole reason this module exists: the greeting is derived from
    /// `wire`, so there is no second place where the wire version or
    /// the schema hash is written down. The probe this replaces had both
    /// copied out into a file outside the workspace.
    #[test]
    fn the_greeting_is_this_build_s_own_and_not_a_transcription() {
        let wire::ClientFrame::Hello(said) = hello(None) else {
            panic!("a greeting is a Hello");
        };
        assert_eq!(said.wire_v, wire::WIRE_V);
        assert_eq!(said.schema, wire::schema_hash());
        assert_eq!(said.token, None);
    }

    #[test]
    fn a_token_travels_when_one_was_given() {
        let wire::ClientFrame::Hello(said) = hello(Some("pair-me")) else {
            panic!("a greeting is a Hello");
        };
        assert_eq!(said.token.as_deref(), Some("pair-me"));
    }
}
