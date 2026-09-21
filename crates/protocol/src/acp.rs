// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The other direction: an outside editor driving this city.
//!
//! `mcp` reaches out; this reaches in. The asymmetry is deliberate and
//! it is the whole security story of the module. An outbound call is
//! something a resident chose; an inbound request is something a
//! stranger sent, so nothing here produces work until the request has
//! passed the admission middleware and been turned into an ordinary
//! dispatch that lands in an ordinary room under an ordinary write
//! domain.
//!
//! **Pairing is decided before this module is reached.** Whether a
//! caller holds this city's token is one judgement on one middleware
//! that every inbound route passes through (`channels::reception`), so
//! no request arrives here unpaired and no token is carried in a value
//! this module could print. A second pairing check here would be a
//! second authority for the same rule, and two authorities disagree the
//! day one of them is edited.
//!
//! What comes back is progress, not a conversation. An editor watching a
//! run wants to know where it got to; giving it a channel into the run
//! would be a second control surface, and the city already has one.

use kernel::{Address, AxCode, AxError};
use serde_json::Value;

/// A request from outside, already read but not yet admitted.
///
/// The fields are private and [`Incoming::parse`] is the only way to
/// make one, which is what makes the grammar single-homed: a caller
/// cannot assemble a request that skipped the checks `parse` performs,
/// because there is no syntax for it outside this module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Incoming {
    addr: Address,
    task: String,
    goal: String,
}

/// What the city does about it. Exhaustive: an inbound request either
/// becomes work or is refused, and there is no third thing that quietly
/// happens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Admitted {
    /// Start a run. The fields are exactly a `Dispatch`'s, because an
    /// external request must not be able to ask for anything a person at
    /// the control surface could not ask for.
    Dispatch {
        addr: Address,
        task: String,
        goal: String,
    },
}

impl Incoming {
    /// Reads one request.
    ///
    /// Keys this version does not read are ignored, which is what lets
    /// the pairing token travel beside the request without entering it.
    ///
    /// # Errors
    /// Refuses a body this version does not read, an address that is not
    /// one, and an empty task or goal. A goal is required for the same
    /// reason it is required at the control surface: a run with no
    /// definition of done cannot report that it is done.
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

/// Decides whether a request becomes work.
///
/// Takes the request by value: what comes out carries the same three
/// fields, so a copy of them would be a second version of one request.
///
/// # Errors
/// Refuses an address inside the reserved subtree: an outside caller
/// must not be able to start work on the city's own files, and holding
/// the pairing token does not change that.
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

/// What an editor is told while a run it asked for is working. Progress
/// only: an editor watching a run is a reader, and the city's one
/// control surface stays the city's one control surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Progress {
    pub run: String,
    pub turns: u32,
    pub finished: bool,
}

impl Progress {
    /// The body an editor receives.
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

    /// The pairing token belongs to the middleware that judges it. A
    /// request that carries one beside it is read without it, so no
    /// log line, panic message or ledger payload made from this value
    /// can publish the secret.
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
