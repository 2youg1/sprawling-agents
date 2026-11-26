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

use std::time::Duration;

use futures_util::StreamExt;
use kernel::{AxCode, EventKind, EventRecord, IdemKey, RunId, Seq};
use remote_access::handshake::{CityFingerprint, Invitation};
use remote_access::route::command::LOCAL_ENV;

#[expect(
    clippy::disallowed_methods,
    reason = "each remote door and its client inherit the same isolated home"
)]
fn serve_workspace() -> Option<(accounting::worker::InitReport, Served)> {
    const CASE: &str = "SPRAWLING_REMOTE_DOOR_TEST_CASE";
    let thread = std::thread::current();
    let case = thread.name().unwrap();
    if std::env::var(CASE).as_deref() != Ok(case) {
        let home = tempfile::tempdir().unwrap();
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", case, "--nocapture"])
            .env(CASE, case)
            .env("USERPROFILE", home.path())
            .env("HOME", home.path())
            .status()
            .unwrap();
        assert!(
            status.success(),
            "{case} failed in its isolated home: {status}"
        );
        return None;
    }
    let home = accounting::home::Home::detect().unwrap();
    let workspace = home.path().join("workspace");
    let private = home.workspace(&workspace).unwrap();
    let raised = sprawling::assembly::init_city(&private).unwrap();
    choose_this_binary_as_the_route(&private);
    Some((raised, Served::start(&workspace)))
}

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
    let Some((raised, mut city)) = serve_workspace() else {
        return;
    };

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

    let answered = visit(&at, &invitation);
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

fn confirm_for(code: &str, n: &[u8], approval: Option<IdemKey>) -> String {
    door_frame(wire::WireCommand::ConfirmRemoteDoor(wire::DoorAnswer {
        approval,
        code: code.to_owned(),
        idem: IdemKey::derive(&RunId::CITY, Seq::FIRST, n),
    }))
}

async fn accepted(page: &mut Socket) -> Option<IdemKey> {
    let deadline = tokio::time::Instant::now().checked_add(PATIENCE)?;
    loop {
        let message = tokio::time::timeout_at(deadline, page.next())
            .await
            .ok()??;
        let Ok(text) = message.ok()?.into_text() else {
            continue;
        };
        match serde_json::from_str(&text) {
            Ok(wire::ServerFrame::Accepted(idem)) => return Some(idem),
            Ok(wire::ServerFrame::Refusal(error)) => {
                panic!("the expected acceptance was refused: {error}")
            }
            Ok(_) | Err(_) => {}
        }
    }
}

const SPACED: Duration = Duration::from_millis(remote_access::confirm::CONFIRM_SPACING_MS + 50);

#[test]
fn a_page_opens_the_door_only_with_the_code_the_console_printed() {
    let Some((raised, mut city)) = serve_workspace() else {
        return;
    };
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
        let first = between(asked.lines().last().unwrap(), "type ", char::is_whitespace);
        refusals.push(refused(&mut page, &confirm("aaaa-aaaa", b"wrong")).await);

        tokio::time::sleep(SPACED).await;
        refusals.push(refused(&mut page, &confirm(&first, b"late")).await);
        refusals.push(refused(&mut page, &open(b"open 2")).await);
        let asked = read_until(&city.printed, &["within two minutes"]).0;
        let second = between(asked.lines().last().unwrap(), "type ", char::is_whitespace);
        tokio::time::sleep(SPACED).await;
        let wrong = IdemKey::derive(&RunId::CITY, Seq::FIRST, b"open 1");
        refusals.push(refused(&mut page, &confirm_for(&second, b"foreign", Some(wrong))).await);

        refusals.push(refused(&mut page, &open(b"open 3")).await);
        let asked = read_until(&city.printed, &["within two minutes"]).0;
        let third = between(asked.lines().last().unwrap(), "type ", char::is_whitespace);
        tokio::time::sleep(SPACED).await;
        let approval = IdemKey::derive(&RunId::CITY, Seq::FIRST, b"open 3");
        send_text(
            &mut page,
            &confirm_for(&third.to_uppercase(), b"right", Some(approval)),
        )
        .await;
        assert_eq!(
            accepted(&mut page).await,
            Some(IdemKey::derive(&RunId::CITY, Seq::FIRST, b"right"))
        );
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
        assert_eq!(
            accepted(&mut page).await,
            Some(IdemKey::derive(&RunId::CITY, Seq::FIRST, b"close"))
        );
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
    let Some((raised, mut city)) = serve_workspace() else {
        return;
    };
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
