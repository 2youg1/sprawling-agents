// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]

mod harness;
use harness::*;

use std::path::Path;
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use kernel::{Address, AxCode, EventKind, EventRecord, IdemKey, RunId, Seq};
use remote_access::door::DeviceId;
use remote_access::handshake::{self, CityFingerprint, Invitation, PairReply, Reply, Session};
use remote_access::keys::{SigningKey, VerifyingKey};
use remote_access::route::command::LOCAL_ENV;
use remote_access::seal::Payload;
use tokio_tungstenite::tungstenite::Message;

#[test]
#[ignore = "the route command the end-to-end test starts; it does nothing on its own"]
fn route_stand_in() {
    let Ok(local) = std::env::var(LOCAL_ENV) else {
        return;
    };

    println!("\n{{\"url\": \"https://{local}\"}}");

    std::thread::sleep(PATIENCE.saturating_mul(2));
}

#[test]
fn a_device_reaches_the_city_through_the_door_over_real_sockets() {
    let dir = tempfile::tempdir().unwrap();
    let raised = sprawling::assembly::init_city(dir.path()).unwrap();
    choose_this_binary_as_the_route(dir.path());
    let mut city = Served::start(dir.path());

    let opened = city.type_line("/remote open --for 1h", "the remote door is open");
    let at = between(&opened, "at https://", char::is_whitespace);
    let home = between(&opened, "WebUI    http://", char::is_whitespace);
    let page = fetched(&at, "/");
    let served_at_home = fetched(&home, "/");
    let invited = city.type_line(&format!("/remote pair {PHONE}"), "pairing code");
    let invitation = Invitation {
        city: CityFingerprint::read(&between(&invited, "&city=", char::is_whitespace)).unwrap(),
        code: between(&invited, "#pair=", |each| each == '&'),
    };

    let runtime = tokio::runtime::Runtime::new().unwrap();
    let answered = runtime.block_on(visit(&at, &invitation));
    let closed = city.type_line("/remote close", "the remote door is closed");
    drop(city);

    let verified = runtime::replay::verify_ledger_dir(&raised.ledger_dir).unwrap();
    let door: Vec<EventKind> = verified
        .raw_lines()
        .iter()
        .map(|line| EventRecord::parse_line(line).unwrap().kind())
        .filter(|kind| {
            matches!(
                kind,
                EventKind::RemoteOpened
                    | EventKind::DevicePaired
                    | EventKind::RemoteSessionStarted
                    | EventKind::CityHalted
                    | EventKind::RemoteClosed
            )
        })
        .collect();
    assert_eq!(
        (page.0.contains(" 200 "), page, answered, door),
        (
            true,
            served_at_home,
            Visit {
                welcomed: true,
                read_answered: true,
                act_carried_out: true,
                local_only_refused: Some(AxCode::GateDenied),
            },
            vec![
                EventKind::RemoteOpened,
                EventKind::DevicePaired,
                EventKind::RemoteSessionStarted,
                EventKind::CityHalted,
                EventKind::RemoteClosed,
            ]
        ),
        "the console printed, closing:\n{closed}"
    );
}

const SPACED: Duration = Duration::from_millis(remote_access::confirm::CONFIRM_SPACING_MS + 50);

#[test]
fn a_page_opens_the_door_only_with_the_code_the_console_printed() {
    let dir = tempfile::tempdir().unwrap();
    let raised = sprawling::assembly::init_city(dir.path()).unwrap();
    choose_this_binary_as_the_route(dir.path());
    let mut city = Served::start(dir.path());
    let banner = city.type_line("/remote devices", "no device is paired");
    let home = between(&banner, "WebUI    http://", char::is_whitespace);
    let home = home.trim_end_matches('/').to_owned();

    let runtime = tokio::runtime::Runtime::new().unwrap();
    let (refusals, printed) = runtime.block_on(async {
        let mut page = page_at(&home).await;
        let mut refusals = Vec::new();
        let mut printed = Vec::new();
        refusals.push(refused(&mut page, &open(b"open 1")).await);
        let asked = read_until(&city.printed, &["within two minutes"]).0;
        let first = between(&asked, "type ", char::is_whitespace);
        refusals.push(refused(&mut page, &confirm("aaaa-aaaa", b"wrong")).await);

        tokio::time::sleep(SPACED).await;
        refusals.push(refused(&mut page, &confirm(&first, b"late")).await);
        refusals.push(refused(&mut page, &open(b"open 2")).await);
        let asked = read_until(&city.printed, &["within two minutes"]).0;
        let second = between(&asked, "type ", char::is_whitespace);
        tokio::time::sleep(SPACED).await;
        send_text(&mut page, &confirm(&second.to_uppercase(), b"right")).await;
        printed.push(
            read_until(&city.printed, &["the remote door is open"])
                .1
                .is_some(),
        );
        send_text(
            &mut page,
            &door_frame(wire::WireCommand::CloseRemoteDoor(wire::DoorStep {
                idem: IdemKey::derive(&RunId::CITY, Seq::FIRST, b"close"),
            })),
        )
        .await;
        printed.push(
            read_until(&city.printed, &["the remote door is closed"])
                .1
                .is_some(),
        );
        (refusals, printed)
    });
    drop(city);

    let verified = runtime::replay::verify_ledger_dir(&raised.ledger_dir).unwrap();
    let door: Vec<EventKind> = verified
        .raw_lines()
        .iter()
        .map(|line| EventRecord::parse_line(line).unwrap().kind())
        .filter(|kind| matches!(kind, EventKind::RemoteOpened | EventKind::RemoteClosed))
        .collect();
    assert_eq!(
        (refusals, printed, door),
        (
            vec![
                Some(AxCode::ApprovalPending),
                Some(AxCode::GateDenied),
                Some(AxCode::GateDenied),
                Some(AxCode::ApprovalPending),
            ],
            vec![true, true],
            vec![EventKind::RemoteOpened, EventKind::RemoteClosed],
        )
    );
}

#[test]
fn ten_wrong_codes_lock_the_pages_door_verbs_and_close_the_door() {
    let dir = tempfile::tempdir().unwrap();
    let raised = sprawling::assembly::init_city(dir.path()).unwrap();
    choose_this_binary_as_the_route(dir.path());
    let mut city = Served::start(dir.path());
    let banner = city.type_line("/remote devices", "no device is paired");
    let home = between(&banner, "WebUI    http://", char::is_whitespace);
    let home = home.trim_end_matches('/').to_owned();
    city.type_line("/remote open --for 1h", "the remote door is open");

    let runtime = tokio::runtime::Runtime::new().unwrap();
    let (rounds, said, after) = runtime.block_on(async {
        let mut page = page_at(&home).await;
        let mut rounds = Vec::new();
        for round in 0..remote_access::confirm::CONFIRM_MISSES_MAX {
            let asked = refused(&mut page, &open(format!("open {round}").as_bytes())).await;
            tokio::time::sleep(SPACED).await;
            let answered = confirm("aaaa-aaaa", format!("wrong {round}").as_bytes());
            rounds.push((asked, refused(&mut page, &answered).await));
        }
        let said = read_until(&city.printed, &["the remote door is closed"]);
        let after = refused(&mut page, &open(b"after")).await;
        (
            rounds,
            said.0.contains("remote-door controls are off"),
            after,
        )
    });
    drop(city);

    let verified = runtime::replay::verify_ledger_dir(&raised.ledger_dir).unwrap();
    let door: Vec<EventKind> = verified
        .raw_lines()
        .iter()
        .map(|line| EventRecord::parse_line(line).unwrap().kind())
        .filter(|kind| matches!(kind, EventKind::RemoteOpened | EventKind::RemoteClosed))
        .collect();
    let round = (Some(AxCode::ApprovalPending), Some(AxCode::GateDenied));
    assert_eq!(
        (
            rounds.len(),
            rounds.iter().all(|each| *each == round),
            said,
            after,
            door
        ),
        (
            usize::try_from(remote_access::confirm::CONFIRM_MISSES_MAX).unwrap(),
            true,
            true,
            Some(AxCode::GateDenied),
            vec![EventKind::RemoteOpened, EventKind::RemoteClosed],
        )
    );
}
