// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use serde::{Deserialize, Serialize};

use crate::event::TimeMs;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RemoteOpened {

    pub closes_at: TimeMs,

    pub url: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RemoteClosed {
    pub why: RemoteClosing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub enum RemoteClosing {

    Console,

    Locked,

    Expired,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DevicePaired {

    pub device: String,

    pub name: String,

    pub authority: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DeviceRevoked {
    pub device: String,
    pub name: String,
}

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
