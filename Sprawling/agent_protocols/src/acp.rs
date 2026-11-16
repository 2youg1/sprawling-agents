// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{Address, AxCode, AxError};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Incoming {
    addr: Address,
    task: String,
    goal: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Admitted {
    Dispatch {
        addr: Address,
        task: String,
        goal: String,
    },
}

impl Incoming {
    pub fn parse(body: &Value) -> Result<Incoming, AxError> {
        let text = |key: &str| -> Result<String, AxError> {
            body.get(key)
                .and_then(Value::as_str)
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
                .ok_or_else(|| {
                    AxError::failure(
                        AxCode::InvalidArgs,
                        "read an external request",
                        format!("missing `{key}`"),
                    )
                    .with_recovery("send `addr`, `task` and `goal`, all non-empty")
                })
        };
        Ok(Incoming {
            addr: Address::parse(&text("addr")?)?,
            task: text("task")?,
            goal: text("goal")?,
        })
    }
}

pub fn admit(request: Incoming) -> Result<Admitted, AxError> {
    if request.addr.is_reserved() {
        return Err(AxError::failure(
            AxCode::OutsideWriteDomain,
            "admit an external request",
            request.addr.as_str().to_owned(),
        )
        .with_recovery("name a room in a building; the city's own subtree is not one"));
    }
    Ok(Admitted::Dispatch {
        addr: request.addr,
        task: request.task,
        goal: request.goal,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Progress {
    pub run: String,
    pub turns: u32,
    pub finished: bool,
}

impl Progress {
    #[must_use]
    pub fn to_body(&self) -> Value {
        serde_json::json!({
            "run": self.run,
            "turns": self.turns,
            "finished": self.finished,
        })
    }
}

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "test code"
)]
mod tests {
    use super::*;
    use serde_json::json;

    fn body() -> Value {
        json!({
            "token": "pair-1234",
            "addr": "lab/room1",
            "task": "fix the kiln timer",
            "goal": "the timer test passes",
        })
    }

    #[test]
    fn a_paired_request_becomes_an_ordinary_dispatch() {
        let request = Incoming::parse(&body()).unwrap();
        let Admitted::Dispatch { addr, task, goal } = admit(request).unwrap();
        assert_eq!(addr.as_str(), "lab/room1");
        assert_eq!(task, "fix the kiln timer");
        assert_eq!(goal, "the timer test passes");
    }

    #[test]
    fn the_pairing_token_does_not_enter_the_request() {
        let request = Incoming::parse(&body()).unwrap();
        let printed = format!("{request:?}");
        assert!(!printed.contains("pair-1234"), "{printed}");
    }

    #[test]
    fn a_valid_token_still_cannot_reach_the_citys_own_subtree() {
        let mut raw = body();
        raw["addr"] = json!(".sprawling/ledger");
        let request = Incoming::parse(&raw).unwrap();
        let err = admit(request).unwrap_err();
        assert_eq!(err.code(), &AxCode::OutsideWriteDomain);
        assert!(err.recovery().contains("room in a building"));
    }

    #[test]
    fn every_field_is_required_and_the_refusal_names_the_missing_one() {
        for key in ["addr", "task", "goal"] {
            let mut raw = body();
            raw[key] = json!("");
            let err = Incoming::parse(&raw).unwrap_err();
            assert!(err.subject().contains(key), "{key}: {}", err.subject());
            assert_eq!(err.code(), &AxCode::InvalidArgs);
        }
        assert!(Incoming::parse(&json!("a string")).is_err());
    }

    #[test]
    fn what_an_editor_gets_back_is_progress_and_nothing_else() {
        let body = Progress {
            run: "r-1".to_owned(),
            turns: 3,
            finished: false,
        }
        .to_body();
        let keys: Vec<&String> = body.as_object().unwrap().keys().collect();
        assert_eq!(keys, ["finished", "run", "turns"]);
    }
}
