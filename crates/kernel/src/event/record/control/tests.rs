// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The golden bytes: each typed payload encodes to the exact string the
//! hand-written writer before it put on the ledger, and reads back as
//! itself, so a replay over an older history re-hashes identically.

use super::*;
use crate::event::Payload;

fn golden<T>(record: &T, bytes: &str)
where
    T: Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug,
{
    let payload = Payload::of(record).unwrap();
    assert_eq!(serde_json::to_string(&payload).unwrap(), bytes);
    let parsed: Payload = serde_json::from_str(bytes).unwrap();
    assert_eq!(&parsed.read::<T>().unwrap(), record);
}

fn lab() -> Address {
    Address::parse("lab").unwrap()
}

#[test]
fn genesis_lines_keep_their_bytes() {
    golden(&CityInitialized {}, "{}");
    golden(
        &BuildingCreated {
            addr: lab(),
            template: "minimal".to_owned(),
            adopted: false,
        },
        r#"{"addr":"lab","template":"minimal"}"#,
    );
    golden(
        &BuildingCreated {
            addr: lab(),
            template: "minimal".to_owned(),
            adopted: true,
        },
        r#"{"addr":"lab","adopted":true,"template":"minimal"}"#,
    );
    golden(
        &BuildingConfigured {
            addr: lab(),
            sandbox: true,
            mcp: false,
            desktop: false,
            context: true,
        },
        r#"{"addr":"lab","context":true,"desktop":false,"mcp":false,"sandbox":true}"#,
    );
}

#[test]
fn run_control_lines_keep_their_bytes() {
    golden(&CancelReceived {}, "{}");
    let must_read = Locator::parse(&format!("cas:b3-{}", "ab".repeat(32))).unwrap();
    golden(
        &HandoffWritten {
            must_read: vec![must_read.clone()],
            overview: "o".to_owned(),
            progress: "p".to_owned(),
            context: "c".to_owned(),
            next_step: "n".to_owned(),
        },
        &format!(
            r#"{{"context":"c","must_read":["{must_read}"],"next_step":"n","overview":"o","progress":"p"}}"#
        ),
    );
    golden(
        &WatchdogFired {
            action: FiredAction::BackOff {
                until_ms: 5000,
                code: "E_PROVIDER".to_owned(),
                subject: "503".to_owned(),
            },
            corrections: 0,
            provider_failures: 1,
        },
        r#"{"action":"back_off","code":"E_PROVIDER","corrections":0,"provider_failures":1,"subject":"503","until_ms":5000}"#,
    );
    golden(
        &WatchdogFired {
            action: FiredAction::Switch {
                to: crate::ServerLabel::parse("b").unwrap(),
                code: "E_PROVIDER".to_owned(),
                subject: "401 Unauthorized".to_owned(),
            },
            corrections: 0,
            provider_failures: 1,
        },
        r#"{"action":"switch","code":"E_PROVIDER","corrections":0,"provider_failures":1,"subject":"401 Unauthorized","to":"b"}"#,
    );
    golden(
        &WatchdogFired {
            action: FiredAction::Freeze {
                reason: "stall".to_owned(),
            },
            corrections: 2,
            provider_failures: 0,
        },
        r#"{"action":"freeze","corrections":2,"provider_failures":0,"reason":"stall"}"#,
    );
}
