// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use gateway::Persistence;
use kernel::AxError;
use kernel::event::record::RemoteClosing;
use qrcodegen::{QrCode, QrCodeEcc};
use remote_access::door::Authority;
use reqwest::Url;

use super::keeper::{Doorway, Inviting, Revoking};
use super::listener::Reaching;

const DEFAULT_HOURS: u64 = 12;

const LONGEST_HOURS: u64 = 7 * 24;
const MS_PER_MINUTE: u64 = 60 * 1000;

const QUIET: i32 = 2;

const USAGE: &str = "/remote open [--for <30m|12h|2d>], /remote pair <name> [--watch], \
                     /remote close, /remote devices, /remote revoke <name>|--all, \
                     /remote replace-key";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RemoteLine {
    Open(Lasting),
    Pair { name: String, authority: Authority },
    Close,
    Devices,
    Revoke(Revoking),

    ReplaceKey,

    Unreadable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Lasting(u64);

impl Lasting {
    pub(crate) fn ms(self) -> u64 {
        self.0
    }

    pub(crate) fn of_ms(ms: u64) -> Option<Lasting> {
        (MS_PER_MINUTE
            ..=LONGEST_HOURS
                .saturating_mul(60)
                .saturating_mul(MS_PER_MINUTE))
            .contains(&ms)
            .then_some(Lasting(ms))
    }

    pub(super) fn read(text: &str) -> Option<Lasting> {
        let unit = text.chars().last()?;
        let count: u64 = text.strip_suffix(unit)?.parse().ok()?;
        let minutes = match unit {
            'm' => Some(count),
            'h' => count.checked_mul(60),
            'd' => count.checked_mul(24 * 60),
            _ => None,
        }?;
        (minutes > 0 && minutes <= LONGEST_HOURS.saturating_mul(60))
            .then(|| Lasting(minutes.saturating_mul(MS_PER_MINUTE)))
    }
}

#[derive(Clone)]
pub(crate) struct Remote {
    pub(crate) doorway: Doorway,
    pub(crate) reaching: Reaching,
}

pub(crate) fn parse(tail: &str) -> RemoteLine {
    let tail = tail.trim();
    let (verb, rest) = tail.split_once(char::is_whitespace).unwrap_or((tail, ""));
    let words: Vec<&str> = rest.split_whitespace().collect();
    match (verb, words.as_slice()) {
        ("open", []) => RemoteLine::Open(Lasting(
            DEFAULT_HOURS
                .saturating_mul(60)
                .saturating_mul(MS_PER_MINUTE),
        )),
        ("open", ["--for", length]) => {
            Lasting::read(length).map_or(RemoteLine::Unreadable, RemoteLine::Open)
        }
        ("pair", _) => {
            let authority = if words.contains(&"--watch") {
                Authority::Watch
            } else {
                Authority::Act
            };
            let name: Vec<&str> = words
                .iter()
                .copied()
                .filter(|word| *word != "--watch")
                .collect();
            if name.is_empty() {
                RemoteLine::Unreadable
            } else {
                RemoteLine::Pair {
                    name: name.join(" "),
                    authority,
                }
            }
        }
        ("close", []) => RemoteLine::Close,
        ("devices", []) => RemoteLine::Devices,
        ("revoke", ["--all"]) => RemoteLine::Revoke(Revoking::All),
        ("revoke", [_, ..]) => RemoteLine::Revoke(Revoking::Named(words.join(" "))),
        ("replace-key", []) => RemoteLine::ReplaceKey,
        _ => RemoteLine::Unreadable,
    }
}

pub(crate) fn carry(remote: &Result<Remote, AxError>, line: RemoteLine) -> String {
    perform(remote, line).unwrap_or_else(|refused| refusal(&refused))
}

pub(crate) fn perform(
    remote: &Result<Remote, AxError>,
    line: RemoteLine,
) -> Result<String, AxError> {
    let remote = remote.as_ref().map_err(Clone::clone)?;
    let doorway = &remote.doorway;
    match line {
        RemoteLine::Open(lasting) => super::listener::open(doorway, &remote.reaching, lasting)
            .and_then(|opened| {
                doorway
                    .key_lasting()
                    .map(|keeps| opening(&opened, lasting, keeps))
            }),
        RemoteLine::Pair { name, authority } => doorway
            .invite(&name, authority)
            .and_then(|inviting| invitation(&inviting)),
        RemoteLine::Close => doorway
            .close(RemoteClosing::Console)
            .map(|_| "  the remote door is closed; paired devices stay paired".to_owned()),
        RemoteLine::Devices => doorway.devices().map(|devices| {
            if devices.is_empty() {
                return "  no device is paired".to_owned();
            }
            devices
                .iter()
                .map(|device| {
                    format!(
                        "  {}  {}  {}",
                        device.name.as_str(),
                        device.authority.word(),
                        device.id.text()
                    )
                })
                .collect::<Vec<String>>()
                .join("\n")
        }),
        RemoteLine::Revoke(which) => doorway.revoke(&which).map(|revoked| {
            if revoked.is_empty() {
                return "  no device is paired".to_owned();
            }
            let names: Vec<&str> = revoked.iter().map(|device| device.name.as_str()).collect();
            format!("  revoked {}; their sessions ended", names.join(", "))
        }),
        RemoteLine::ReplaceKey => doorway.replace_key().and_then(|revoked| {
            let names: Vec<&str> = revoked.iter().map(|device| device.name.as_str()).collect();
            let who = if names.is_empty() {
                "no device was paired".to_owned()
            } else {
                format!("{} pair again", names.join(", "))
            };
            doorway
                .key_lasting()
                .map(|keeps| format!("  the city has a new key; {who}\n  {}", kept_for(keeps)))
        }),
        RemoteLine::Unreadable => Ok(format!("  {USAGE}")),
    }
}

fn opening(origin: &Url, lasting: Lasting, keeps: Persistence) -> String {
    let minutes = lasting.ms().checked_div(MS_PER_MINUTE).unwrap_or_default();
    format!(
        "  the remote door is open for {}h {}m at {}\n  {}",
        minutes.checked_div(60).unwrap_or_default(),
        minutes.checked_rem(60).unwrap_or_default(),
        origin.as_str(),
        kept_for(keeps)
    )
}

fn kept_for(keeps: Persistence) -> &'static str {
    match keeps {
        Persistence::AcrossReboots => {
            "the city's key is kept in this computer's credential store: paired devices stay \
             paired when the city or the computer restarts"
        }
        Persistence::AcrossRebootsWithPassphrase => {
            "the city's key is kept in the encrypted vault file: paired devices stay paired when \
             the city or the computer restarts"
        }
        Persistence::ThisBoot => {
            "the city's key is kept in the kernel keyring until this computer restarts: devices \
             pair again after that"
        }
        Persistence::ThisProcess => {
            "the city's key lives in this process only: devices pair again after the city restarts"
        }
    }
}

fn invitation(inviting: &Inviting) -> Result<String, AxError> {
    Ok(format!(
        "{}\n  {}\n  pairing code {} - good for ten minutes, once",
        qr(&inviting.link)?,
        inviting.link,
        inviting.shown
    ))
}

fn qr(link: &str) -> Result<String, AxError> {
    let code = QrCode::encode_text(link, QrCodeEcc::Low).map_err(|too_long| {
        AxError::failure(
            kernel::AxCode::InvalidArgs,
            "draw the pairing link as a QR code",
            too_long.to_string(),
        )
        .with_recovery("type the link printed below the code into the device instead")
    })?;
    let light = |x: i32, y: i32| !code.get_module(x, y);
    let from = QUIET.saturating_neg();
    let to = code.size().saturating_add(QUIET);
    let lines: Vec<String> = (from..to)
        .step_by(2)
        .map(|y| {
            (from..to)
                .map(|x| match (light(x, y), light(x, y.saturating_add(1))) {
                    (true, true) => '\u{2588}',
                    (true, false) => '\u{2580}',
                    (false, true) => '\u{2584}',
                    (false, false) => ' ',
                })
                .collect::<String>()
        })
        .map(|row| format!("  {row}"))
        .collect();
    Ok(lines.join("\n"))
}

fn refusal(refused: &AxError) -> String {
    format!("  {refused}\n  {}", refused.recovery())
}
