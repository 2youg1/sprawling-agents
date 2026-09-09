// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::float_arithmetic,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::string_slice,
    clippy::arithmetic_side_effects,
    reason = "test code"
)]

use super::super::*;
use crate::assembly::fixture::*;
use crate::assembly::*;
use crate::serving::CommandDesk;

#[test]
fn a_socket_that_carried_no_request_spends_no_scripted_reply() {
    let (base_url, provider) = fake_openai(
        &["m-local"],
        vec![
            completion("the first turn", None),
            completion("the second turn", None),
        ],
    );
    let addr = base_url
        .trim_start_matches("http://")
        .trim_end_matches("/v1")
        .to_owned();

    // Opened and abandoned: a pooled socket nobody wrote to.
    drop(std::net::TcpStream::connect(&addr).unwrap());

    // Opened and cut short: a body that stops before content-length.
    let mut cut = std::net::TcpStream::connect(&addr).unwrap();
    std::io::Write::write_all(
        &mut cut,
        b"POST /v1/chat/completions HTTP/1.1\r\ncontent-length: 64\r\n\r\n{\"cut\":",
    )
    .unwrap();
    drop(cut);

    // The first socket that actually asked gets the first reply.
    let body = "{\"model\":\"m-local\"}";
    let mut asking = std::net::TcpStream::connect(&addr).unwrap();
    std::io::Write::write_all(
        &mut asking,
        format!(
            "POST /v1/chat/completions HTTP/1.1\r\ncontent-length: {}\r\n\r\n{body}",
            body.len()
        )
        .as_bytes(),
    )
    .unwrap();
    let mut answered = String::new();
    std::io::Read::read_to_string(&mut asking, &mut answered).unwrap();

    assert!(
        answered.contains("the first turn"),
        "a socket that asked nothing spent the first reply: {answered}"
    );
    assert_eq!(
        provider.exchanges().len(),
        1,
        "only what arrived whole is on the record: {:?}",
        provider.exchanges()
    );
}
#[test]
fn a_dispatch_runs_a_whole_turn_loop_and_the_chain_still_verifies() {
    let dir = tempfile::tempdir().unwrap();
    let report = init_city(dir.path()).unwrap();

    let (base_url, _provider) = fake_openai(
        &["m-local"],
        vec![
            completion("editing", Some(("tu_1", "lab/room1/notes.md"))),
            completion("done", None),
        ],
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let sink = std::sync::Arc::clone(&seen);
    worker.observe(Box::new(move |record: &EventRecord| {
        sink.lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(record.kind());
    }));

    worker
        .handle(channels::Command::Dispatch {
            addr: Address::parse("lab/room1").unwrap(),
            task: "say hello".to_owned(),
            goal: "one turn is enough".to_owned(),
            mode: channels::ModeTag::parse("plan").unwrap(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            effort: None,
        })
        .unwrap();

    let kinds = seen
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone();
    assert!(kinds.contains(&EventKind::RunStarted), "{kinds:?}");
    assert!(kinds.contains(&EventKind::ToolCalled), "{kinds:?}");
    assert!(kinds.contains(&EventKind::HandoffWritten), "{kinds:?}");
    assert!(kinds.contains(&EventKind::RunFrozen), "{kinds:?}");

    // The effect, not only the freeze: the canary edit really landed.
    // Asserting kinds alone once passed while every edit was refused.
    assert_eq!(
        std::fs::read_to_string(dir.path().join("lab/room1/notes.md")).unwrap(),
        "noted\n"
    );

    // The whole thing is one verifiable chain, genesis included.
    let verified = runtime::replay::verify_ledger_dir(&report.ledger_dir).unwrap();
    assert!(verified.raw_lines().len() > kinds.len());
}
#[test]
fn a_cancel_reaches_the_run_it_cancels_without_waiting_for_it_to_end() {
    let desk = CommandDesk::new();
    let mine = RunId::CITY;
    let other = kernel::RunId::from_bytes([7u8; 16]);
    // One key per ask, the way every client mints them: the desk
    // now reads the key, so three commands sharing one would be one
    // command repeated twice (section 8-33).
    let key = |material: &[u8]| kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, material);

    let nobody = || channels::Reply::nowhere();
    desk.post(
        channels::Command::Steer {
            run: other,
            text: "not for me".to_owned(),
            idem: key(b"not for me"),
        },
        nobody(),
    );
    desk.post(
        channels::Command::Steer {
            run: mine,
            text: "  measure it in metres  ".to_owned(),
            idem: key(b"measure it in metres"),
        },
        nobody(),
    );
    desk.post(
        channels::Command::Cancel {
            run: mine,
            idem: key(b"cancel"),
        },
        nobody(),
    );

    // Cancel outranks a steer that arrived first: stopping and
    // changing course are exclusive, and stopping cannot be undone.
    assert!(matches!(desk.interrupt_for(mine), Interrupt::Cancel));
    let Interrupt::Steer { source, text } = desk.interrupt_for(mine) else {
        panic!("the steer for this run is still waiting");
    };
    assert_eq!(source, "user");
    assert_eq!(text, "measure it in metres");
    assert!(matches!(desk.interrupt_for(mine), Interrupt::None));

    // And the other run's command kept its place in line.
    let Some(channels::Command::Steer { run, .. }) = desk.take() else {
        panic!("a command for another run is not consumed by this one");
    };
    assert_eq!(run, other);
}
#[test]
fn a_steer_lands_at_the_end_of_the_next_tool_result() {
    let dir = tempfile::tempdir().unwrap();
    let report = init_city(dir.path()).unwrap();
    let (base_url, provider) = fake_openai(
        &["m-local"],
        vec![
            completion("editing", Some(("tu_1", "lab/room1/notes.md"))),
            completion("done", None),
        ],
    );
    let mut worker = worker_with_provider(dir.path(), &base_url, "m-local").unwrap();
    // One steer, delivered at the first safe point that asks.
    // A count the source can spend without owning it: N runs may ask
    // this one source at once, so it answers by `Fn` rather than by
    // `FnMut` (sprawling-SPEC 8-46-1).
    let left = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(1));
    worker.attach_interrupts(std::sync::Arc::new(move |_| {
        if left
            .fetch_update(
                std::sync::atomic::Ordering::SeqCst,
                std::sync::atomic::Ordering::SeqCst,
                |held| held.checked_sub(1),
            )
            .is_ok()
        {
            return Interrupt::Steer {
                source: "user".to_owned(),
                text: "measure it in metres".to_owned(),
            };
        }
        Interrupt::None
    }));
    worker
        .handle(channels::Command::Dispatch {
            addr: Address::parse("lab/room1").unwrap(),
            task: "measure the thing".to_owned(),
            goal: "a number, then stop".to_owned(),
            mode: channels::ModeTag::parse("plan").unwrap(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            effort: None,
        })
        .unwrap();

    let verified = runtime::replay::verify_ledger_dir(&report.ledger_dir).unwrap();
    let history: String = verified
        .raw_lines()
        .iter()
        .map(|line| String::from_utf8_lossy(line).into_owned())
        .collect::<Vec<String>>()
        .join("\n");
    assert!(history.contains("steer_received"));

    let asked = provider.bodies().join("\n");
    assert!(
        asked.contains("measure it in metres"),
        "the steer reaches the model in the window, not only the ledger"
    );
    assert!(
        history.contains("run_frozen"),
        "a steer changes course; it does not end the run"
    );
}

/// Deleting one file, in this platform's own words. The point of the
/// test is the sweep, and the sweep does not care which command did
/// it — which is exactly why the sweep is the defence.
#[test]
fn a_provider_failure_freezes_the_run_instead_of_hanging_it() {
    let dir = tempfile::tempdir().unwrap();
    init_city(dir.path()).unwrap();
    // A provider that lists a model, then answers 401 to every chat.
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { return };
            let mut buf = [0u8; 8192];
            let mut head = String::new();
            loop {
                let Ok(n) = std::io::Read::read(&mut stream, &mut buf) else {
                    return;
                };
                if n == 0 {
                    break;
                }
                head.push_str(&String::from_utf8_lossy(&buf[..n]));
                if head.contains("\r\n\r\n") && (head.starts_with("GET") || head.ends_with('}')) {
                    break;
                }
            }
            let body = if head.starts_with("GET") {
                r#"{"data":[{"id":"m-503"}]}"#.to_owned()
            } else {
                r#"{"error":{"message":"Authentication failed"}}"#.to_owned()
            };
            let status = if head.starts_with("GET") {
                "200 OK"
            } else {
                "401 Unauthorized"
            };
            let response = format!(
                "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = std::io::Write::write_all(&mut stream, response.as_bytes());
        }
    });
    let mut worker =
        worker_with_provider(dir.path(), &format!("http://{addr}/v1"), "m-503").unwrap();
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let sink = std::sync::Arc::clone(&seen);
    worker.observe(Box::new(move |record: &EventRecord| {
        sink.lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(record.kind());
    }));
    // The command surface must not error out: the failure belongs to
    // the run's own account, not to the person's keystroke.
    worker
        .handle(channels::Command::Dispatch {
            addr: Address::parse("lab/room1").unwrap(),
            task: "anything".to_owned(),
            goal: "an honest ending".to_owned(),
            mode: channels::ModeTag::parse("plan").unwrap(),
            idem: kernel::IdemKey::derive(&RunId::CITY, kernel::Seq::FIRST, b"dispatch"),
            session: None,
            effort: None,
        })
        .unwrap();
    let kinds = seen
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone();
    // Before the drive backstop: events stopped at model_called and
    // the run never froze - a browser watching the stream saw it go
    // quiet forever.
    assert!(
        kinds.contains(&EventKind::ProviderDegraded),
        "the failure is written under its carrier: {kinds:?}"
    );
    assert!(
        kinds.contains(&EventKind::RunFrozen),
        "a run always ends: {kinds:?}"
    );
}

/// What one drive is handed can leave the thread that built it
/// (sprawling-SPEC 8-44), and owns everything it runs on, so a lane can
/// be handed one (sprawling-SPEC 8-46-1). A bound rather than a run: a
/// value that cannot be sent is a compile error here before it is a
/// design error in the pool.
#[test]
fn a_drive_can_be_handed_to_another_thread() {
    fn crosses_threads<T: Send + 'static>() {}
    crosses_threads::<Driving>();
}
