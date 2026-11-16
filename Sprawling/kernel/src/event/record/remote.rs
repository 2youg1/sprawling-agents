// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The remote door's five lines: it opened, it closed, a device paired,
//! a device was revoked, a device started a remote session
//! (`crates/kernel/spec/Event/Record.lean` §8-81).
//!
//! **No line here carries a key, a pairing code or a session id.** The
//! history says who was let in and when; what a device proves itself
//! with stays in the device table, and a ledger anybody can replay
//! would otherwise hand a reader the means to be that device.
//!
//! A device is spelled by the text of its id and a person's name for
//! it, and its authority by the word the remote door gives it: the
//! types that decide those live in `remote_access`, which the kernel
//! does not depend on.

use serde::{Deserialize, Serialize};

use crate::event::TimeMs;

/// `remote_opened`: the door is open until `closes_at`, reached at `url`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RemoteOpened {
    /// When the door closes by itself.
    pub closes_at: TimeMs,
    /// The `https://` address the route answered, which a device opens.
    pub url: String,
}

/// `remote_closed`: the door closed, and why.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RemoteClosed {
    pub why: RemoteClosing,
}

/// Who closed the remote door.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum RemoteClosing {
    /// The person, at the city's console.
    Console,
    /// A device holding a session, which may lock the door behind it.
    Locked,
    /// The time the door was opened for ran out.
    Expired,
}

/// `device_paired`: a device presented a good code and the city keeps it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DevicePaired {
    /// The device id's text, as the device table spells it.
    pub device: String,
    /// What the person called the device.
    pub name: String,
    /// `watch` or `act`.
    pub authority: String,
}

/// `device_revoked`: the person took a device's pairing away.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DeviceRevoked {
    pub device: String,
    pub name: String,
}

/// `remote_session_started`: a paired device proved its key and holds a
/// session until `expires`, which is never later than the door closes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RemoteSessionStarted {
    pub device: String,
    pub expires: TimeMs,
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, reason = "test code")]
mod tests {
    use super::*;
    use crate::event::Payload;

    /// Each line's bytes, so a ledger written by this build reads back in
    /// the next one.
    #[test]
    fn each_remote_door_line_spells_its_keys_once() {
        let device = "aaaqeayeaudaocajbifqydiob4".to_owned();
        let cases: [(Payload, &str); 5] = [
            (
                Payload::of(&RemoteOpened {
                    closes_at: TimeMs::new(9),
                    url: "https://city.example".to_owned(),
                })
                .unwrap(),
                "{\"closes_at\":9,\"url\":\"https://city.example\"}",
            ),
            (
                Payload::of(&RemoteClosed {
                    why: RemoteClosing::Locked,
                })
                .unwrap(),
                "{\"why\":\"locked\"}",
            ),
            (
                Payload::of(&DevicePaired {
                    device: device.clone(),
                    name: "phone".to_owned(),
                    authority: "watch".to_owned(),
                })
                .unwrap(),
                "{\"authority\":\"watch\",\"device\":\"aaaqeayeaudaocajbifqydiob4\",\"name\":\"phone\"}",
            ),
            (
                Payload::of(&DeviceRevoked {
                    device: device.clone(),
                    name: "phone".to_owned(),
                })
                .unwrap(),
                "{\"device\":\"aaaqeayeaudaocajbifqydiob4\",\"name\":\"phone\"}",
            ),
            (
                Payload::of(&RemoteSessionStarted {
                    device,
                    expires: TimeMs::new(7),
                })
                .unwrap(),
                "{\"device\":\"aaaqeayeaudaocajbifqydiob4\",\"expires\":7}",
            ),
        ];
        for (payload, wire) in cases {
            assert_eq!(serde_json::to_string(&payload).unwrap(), wire);
            let back: Payload = serde_json::from_str(wire).unwrap();
            assert_eq!(back, payload);
        }
    }
}
