// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use futures_util::{SinkExt, StreamExt};
use kernel::{AxCode, AxError};
use std::time::Duration;
use tokio_tungstenite::tungstenite::Message;

mod ending;

mod enrolment;

mod watching;

use ending::{Awaited, Echo, Ending, Heard, Reply};
pub(crate) use ending::{Milestone, Spoken, Until};
pub(crate) use enrolment::{enrol, split_reference};
pub(crate) use watching::top;

pub(crate) struct Listen {
    pub(crate) quiet: Duration,
    pub(crate) until: Until,
}

#[derive(Debug)]
pub(crate) enum Unheard {

    Unreadable(AxError),

    Unkeyed(AxError),

    NoCity(AxError),

    Broken(AxError),
}

#[derive(Debug, PartialEq, Eq)]
enum Reach {

    ThisMachine(u16),

    Portless,

    Elsewhere,
}

fn reach(at: &str) -> Reach {
    let (host, port) = at.rsplit_once(':').unwrap_or((at, ""));
    match kernel::gate::target_of(host) {
        kernel::gate::EgressTarget::Loopback => port
            .parse::<u16>()
            .map_or(Reach::Portless, Reach::ThisMachine),
        kernel::gate::EgressTarget::Private
        | kernel::gate::EgressTarget::Public { .. }
        | kernel::gate::EgressTarget::Connector { .. } => Reach::Elsewhere,
    }
}

fn credential(at: &str, given: Option<&str>) -> Result<Option<String>, AxError> {
    if let Some(given) = given {
        return Ok(Some(given.to_owned()));
    }
    match reach(at) {
        Reach::ThisMachine(port) => Ok(match sprawling::serving::key_file::read_key(port) {
            Ok(key) => key,
            Err(unread) => {
                eprintln!("{}: {}", unread.action(), unread.recovery());
                None
            }
        }),
        Reach::Portless => Ok(None),
        Reach::Elsewhere => Err(AxError::failure(
            AxCode::CredentialMissing,
            "find a credential for the city",
            format!("{at} is not on this machine, and no --token was given"),
        )
        .with_recovery(
            "pass --token <key>: the city's pairing token, or the key its banner showed; \
             this machine's key file answers only for 127.0.0.1, [::1] or localhost",
        )),
    }
}

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

pub(crate) fn call(
    at: &str,
    frame: &str,
    token: Option<&str>,
    listen: Listen,
) -> Result<Heard, Unheard> {

    let outgoing: wire::ClientFrame = serde_json::from_str(frame).map_err(|err| {
        Unheard::Unreadable(malformed("read the frame to send", &err.to_string()))
    })?;
    send(at, &outgoing, token, listen)
}

pub(crate) fn send(
    at: &str,
    outgoing: &wire::ClientFrame,
    token: Option<&str>,
    listen: Listen,
) -> Result<Heard, Unheard> {
    let body = serde_json::to_string(outgoing).map_err(|err| {
        Unheard::Unreadable(malformed("encode the frame to send", &err.to_string()))
    })?;
    let key = credential(at, token).map_err(Unheard::Unkeyed)?;
    let greeting = serde_json::to_string(&hello(key.as_deref()))
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

    let _closed = socket.close(None).await;
    Ok(heard)
}

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

            Some(Ok(Message::Close(_))) | None => return Ok(None),

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
    use super::{
        Duration, Listen, Message, Reach, SinkExt, Spoken, StreamExt, Until, hello, reach,
    };
    use kernel::{EventDraft, EventKind, EventRecord, GENESIS_PREV, Payload, RunId, Seq, TimeMs};

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

                let mut socket = tokio_tungstenite::accept_async(stream).await.unwrap();

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

    fn cancelling() -> String {
        let run = serde_json::to_string(&RunId::CITY).unwrap();
        format!(
            "{{\"command\":{{\"cancel\":{{\"run\":{run},             \"idem\":\"idem1-00000000000000000000000000000002\"}}}}}}"
        )
    }

    #[test]
    fn only_a_host_on_this_machine_is_answered_from_the_key_file() {
        for at in [
            "127.0.0.1:2501",
            "127.8.9.1:2501",
            "[::1]:2501",
            "localhost:2501",
            "LocalHost:2501",
        ] {
            assert_eq!(reach(at), Reach::ThisMachine(2501), "{at}");
        }
        for at in [
            "other-host:2501",
            "192.168.1.5:2501",
            "10.0.0.2:2501",
            "0.0.0.0:2501",
            "example.com:2501",
            "localhost.example.com:2501",
            "127.0.0.1.example.com:2501",
            "[2001:db8::1]:2501",
        ] {
            assert_eq!(reach(at), Reach::Elsewhere, "{at}");
        }
        for at in ["127.0.0.1", "localhost:", "localhost:http"] {
            assert_eq!(reach(at), Reach::Portless, "{at}");
        }
    }

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
