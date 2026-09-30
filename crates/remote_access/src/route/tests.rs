// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! Which addresses a route may answer; that a command route, run against
//! a scripted child, answers what the scripted route answers; that a
//! command which ends early, prints its address wrong or prints nothing is
//! refused; and that closing is always safe.
//!
//! The child is a script written for each test (a `.cmd` file on Windows,
//! an `sh` script elsewhere), because a command line quoted for Windows is
//! not read back the same way by `cmd` itself.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "test code"
)]

use std::net::SocketAddr;
use std::path::PathBuf;

use kernel::{AxCode, AxError, TimeoutMs};

use super::cloudflare::{NamedTunnel, Tunnel, TunnelName};
use super::command::{CommandRoute, LOCAL_ENV, RouteCommand};
use super::scripted::ScriptedRoute;
use super::{Opened, Permanence, PublicUrl, Route};

fn local() -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], 4000))
}

fn patience() -> TimeoutMs {
    TimeoutMs(20_000)
}

fn code(refusal: AxError) -> AxCode {
    *refusal.code()
}

/// What a scripted child does once its lines are printed.
enum Then {
    Exit,
    Wait,
}

/// A script file that lives as long as the test holding it.
struct Script(PathBuf);

impl Drop for Script {
    fn drop(&mut self) {
        match std::fs::remove_file(&self.0) {
            Ok(()) | Err(_) => {}
        }
    }
}

/// A route command that prints `lines`, in which `{local}` stands for the
/// address the route hands the command.
fn script(name: &str, lines: &[&str], then: Then) -> (Script, RouteCommand) {
    let stem = format!("sprawling-route-{}-{name}", std::process::id());
    if cfg!(windows) {
        let path = std::env::temp_dir().join(format!("{stem}.cmd"));
        let printed: String = lines
            .iter()
            .map(|line| {
                format!(
                    "echo {}\r\n",
                    line.replace("{local}", &format!("%{LOCAL_ENV}%"))
                )
            })
            .collect();
        let waited = match then {
            Then::Exit => "",
            Then::Wait => "ping -n 6 127.0.0.1 >NUL\r\n",
        };
        std::fs::write(&path, format!("@echo off\r\n{printed}{waited}")).unwrap();
        let command = RouteCommand {
            program: path.clone(),
            args: Vec::new(),
        };
        (Script(path), command)
    } else {
        let path = std::env::temp_dir().join(format!("{stem}.sh"));
        let printed = lines
            .join("\n")
            .replace("{local}", &format!("${LOCAL_ENV}"));
        let waited = match then {
            Then::Exit => "",
            Then::Wait => "sleep 5\n",
        };
        std::fs::write(&path, format!("cat <<EOF\n{printed}\nEOF\n{waited}")).unwrap();
        let command = RouteCommand {
            program: PathBuf::from("sh"),
            args: vec![path.to_string_lossy().into_owned()],
        };
        (Script(path), command)
    }
}

#[test]
fn an_address_a_page_can_use_is_https_with_a_host_and_nothing_else() {
    let read = |text: &str| {
        PublicUrl::parse(text)
            .map(|url| url.as_str().to_owned())
            .map_err(code)
    };
    assert_eq!(
        [
            "https://city.example",
            "HTTPS://city.example/remote?x=1",
            "https://[::1]:8443",
        ]
        .map(read),
        [
            Ok("https://city.example".to_owned()),
            Ok("https://city.example/remote?x=1".to_owned()),
            Ok("https://[::1]:8443".to_owned()),
        ]
    );
    assert_eq!(
        [
            "http://city.example",
            "wss://city.example",
            "city.example",
            "https://",
            "https:///remote",
            "https://:443",
            "https://someone@city.example",
            "https://city.example/#pair",
            "https://city example",
            "https://city.example/\n",
        ]
        .map(read),
        [const { Err(AxCode::ConfigInvalid) }; 10]
    );
}

#[test]
fn a_command_route_answers_what_the_scripted_route_answers() {
    let (_script, command) = script(
        "answers",
        &["starting", r#"{"url": "https://{local}"}"#],
        Then::Exit,
    );
    let mut scripted = ScriptedRoute::new(Opened {
        url: PublicUrl("https://127.0.0.1:4000".to_owned()),
        permanence: Permanence::Fixed,
    });
    let mut commanded = CommandRoute::new(command, Permanence::Fixed, patience());
    assert_eq!(
        (
            commanded.open(local()).map_err(code),
            commanded.close().map_err(code)
        ),
        (
            scripted.open(local()).map_err(code),
            scripted.close().map_err(code)
        )
    );
}

#[test]
fn a_command_that_ends_early_or_prints_its_address_wrong_is_refused() {
    let opened = |name: &str, lines: &[&str]| {
        let (_script, command) = script(name, lines, Then::Exit);
        CommandRoute::new(command, Permanence::Fixed, patience())
            .open(local())
            .map_err(code)
    };
    assert_eq!(
        [
            opened("ends", &["starting"]),
            opened("plain", &[r#"{"url": "http://{local}"}"#]),
            opened("escaped", &[r#"{"url": "https:\/\/{local}"}"#]),
        ],
        [
            Err(AxCode::ToolUnavailable),
            Err(AxCode::ConfigInvalid),
            Err(AxCode::ConfigInvalid),
        ]
    );
}

#[test]
fn a_command_that_prints_no_address_in_time_is_stopped() {
    let (_script, command) = script("silent", &["starting"], Then::Wait);
    assert_eq!(
        CommandRoute::new(command, Permanence::Fixed, TimeoutMs(300))
            .open(local())
            .map_err(code),
        Err(AxCode::Timeout)
    );
}

#[test]
fn closing_a_route_that_is_not_open_succeeds() {
    let tunnel = TunnelName::parse("city").and_then(|name| {
        NamedTunnel::new(
            Tunnel {
                program: PathBuf::from("cloudflared"),
                name,
                url: PublicUrl("https://city.example".to_owned()),
            },
            patience(),
        )
        .close()
    });
    let command = CommandRoute::new(
        RouteCommand {
            program: PathBuf::from("route"),
            args: Vec::new(),
        },
        Permanence::PerStart,
        patience(),
    )
    .close();
    let scripted = ScriptedRoute::new(Opened {
        url: PublicUrl("https://city.example".to_owned()),
        permanence: Permanence::Fixed,
    })
    .close();
    assert_eq!(
        [tunnel, command, scripted].map(|closed| closed.map_err(code)),
        [Ok(()), Ok(()), Ok(())]
    );
}

#[test]
fn a_tunnel_name_cloudflared_would_read_as_an_option_is_refused() {
    let read = |text: &str| {
        TunnelName::parse(text)
            .map(|name| name.as_str().to_owned())
            .map_err(code)
    };
    assert_eq!(
        ["sprawling.home_1", "-city", "", "a b", &"x".repeat(65)].map(read),
        [
            Ok("sprawling.home_1".to_owned()),
            Err(AxCode::ConfigInvalid),
            Err(AxCode::ConfigInvalid),
            Err(AxCode::ConfigInvalid),
            Err(AxCode::ConfigInvalid),
        ]
    );
}
