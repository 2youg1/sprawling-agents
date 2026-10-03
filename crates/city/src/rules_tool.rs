// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The face a building's rules show a model: what this building's runs
//! are judged by, read as it stands.
//!
//! **Why a run only reads.** `RULES.toml` lives in the building's
//! reserved subtree, and no write domain reaches there, which is not an
//! oversight to work around but the rule itself: a run may not change
//! what governs it (`crates/city/spec/RulesTool.lean` §8-2b, city D1).
//! The tool is declared `Effect::Govern`; a `read` call answers
//! `Effect::Read`, so a run sees what it is judged by. A model that asks
//! to `propose` new rules is answered by the effect layer's Govern
//! refusal, which names the file the User edits, and the tool offers no
//! such operation (city D23).

use std::path::{Path, PathBuf};

use kernel::event::Scope;
use kernel::{
    Address, AxCode, AxError, CostTier, Effect, GateSubject, Payload, RenderIntent, Temporal, Tool,
    ToolCall, ToolMeta, ToolName, ToolOutcome,
};
use serde_json::{Map, Value};

use crate::policy::{RULES_FILE, rules_path};

/// The tool. It holds where the city is and which building the calling
/// run belongs to, so a run cannot rewrite another building's rules by
/// naming one.
pub struct RulesTool {
    city_root: PathBuf,
    building: Address,
    meta: ToolMeta,
}

impl RulesTool {
    /// # Errors
    /// Propagates a malformed parameter schema, which is a build-time
    /// defect rather than a runtime one.
    pub fn new(city_root: &Path, building: Address) -> Result<RulesTool, AxError> {
        let mut op = Map::new();
        op.insert("type".to_owned(), Value::String("string".to_owned()));
        op.insert(
            "description".to_owned(),
            Value::String("`read`: the rules as they stand".to_owned()),
        );
        op.insert(
            "enum".to_owned(),
            Value::Array(vec![Value::String("read".to_owned())]),
        );
        let mut properties = Map::new();
        properties.insert("op".to_owned(), Value::Object(op));
        let mut params = Map::new();
        params.insert("type".to_owned(), Value::String("object".to_owned()));
        params.insert("properties".to_owned(), Value::Object(properties));
        params.insert(
            "required".to_owned(),
            Value::Array(vec![Value::String("op".to_owned())]),
        );
        Ok(RulesTool {
            city_root: city_root.to_path_buf(),
            building,
            meta: ToolMeta {
                name: ToolName::parse("rules")?,
                disclosure: format!(
                    "What this building's runs are judged by. `read` shows them. No run may \
                     change them: the User edits the {RULES_FILE}."
                ),
                params: Payload::new(params)?,
                effect: Effect::Govern,
                cost_tier: CostTier::Light,
                timeout: None,
                render: RenderIntent::Generic,
                temporal: Temporal::Timeless,
            },
        })
    }

    /// The rules as they stand. A building nobody has written rules for
    /// has none to show, which is an answer rather than a failure.
    fn standing(&self) -> Result<String, AxError> {
        let path = rules_path(&self.city_root, &self.building);
        match std::fs::read_to_string(&path) {
            Ok(text) => Ok(text),
            // Linux says `NotADirectory` where Windows says `NotFound`
            // for a path through a file, as `policy::load` reads it.
            Err(err)
                if matches!(
                    err.kind(),
                    std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory
                ) =>
            {
                Ok(String::new())
            }
            Err(err) => Err(AxError::failure(
                AxCode::StorageFatal,
                "read a building's rules",
                format!("{}: {err}", path.display()),
            )
            .with_recovery(format!(
                "make {} readable by the city, or remove it so the building has no rules",
                path.display()
            ))),
        }
    }
}

/// What a call asks of the rules. Exhaustive: an unknown verb is refused
/// rather than read as the harmless one. `Rewrite` is not offered to the
/// model; it is read so that a model which asks to `propose` new rules
/// meets the effect layer's Govern refusal, which says who changes them,
/// rather than an unknown-verb refusal (city D23).
enum Op {
    Read,
    Rewrite,
}

impl Op {
    /// The one reading of a call's arguments, shared by `subject`,
    /// `effect_of` and `invoke` so each refuses an unreadable call in the
    /// same words.
    fn read(args: &Map<String, Value>) -> Result<Op, AxError> {
        match arg(args, "op")? {
            "read" => Ok(Op::Read),
            "propose" => Ok(Op::Rewrite),
            other => Err(AxError::failure(
                AxCode::InvalidArgs,
                "read a building's rules",
                format!("no such operation: {other}"),
            )
            .with_recovery("call with `op` set to `read`")),
        }
    }
}

fn arg<'a>(args: &'a Map<String, Value>, key: &str) -> Result<&'a str, AxError> {
    args.get(key).and_then(Value::as_str).ok_or_else(|| {
        AxError::failure(
            AxCode::InvalidArgs,
            "read a building's rules",
            format!("missing string argument `{key}`"),
        )
        .with_recovery("call with `op` set to `read`")
    })
}

impl Tool for RulesTool {
    fn meta(&self) -> &ToolMeta {
        &self.meta
    }

    /// This building's rules are what every call is about, read or asked
    /// to be rewritten (`crates/city/spec/RulesTool.lean` §8-36).
    fn subject(&self, call: &ToolCall) -> Result<GateSubject, AxError> {
        Op::read(call.args.as_map())?;
        Ok(GateSubject::Scope(
            Scope::Building(self.building.clone()).to_string(),
        ))
    }

    /// Reading the rules changes nothing they govern, so a run may read
    /// them; a rewrite still meets the Govern refusal
    /// (`crates/kernel/spec/Gate.lean` D26).
    fn effect_of(&self, call: &ToolCall) -> Result<Effect, AxError> {
        Ok(match Op::read(call.args.as_map())? {
            Op::Read => Effect::Read,
            Op::Rewrite => Effect::Govern,
        })
    }

    fn invoke(&self, call: &ToolCall) -> Result<ToolOutcome, AxError> {
        if call.name != self.meta.name {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "read a building's rules",
                format!("call routed to the wrong tool: {}", call.name.as_str()),
            )
            .with_recovery(format!(
                "call `{}`, the name this tool answers to",
                self.meta.name.as_str()
            )));
        }
        let args = call.args.as_map();
        let mut out = Map::new();
        out.insert(
            "scope".to_owned(),
            Value::String(self.building.as_str().to_owned()),
        );
        match Op::read(args)? {
            Op::Read => {
                out.insert("text".to_owned(), Value::String(self.standing()?));
            }
            // The bench refuses a Govern call before `invoke`; a caller
            // that reaches here without it is refused the same way.
            Op::Rewrite => {
                return Err(AxError::failure(
                    AxCode::GateDenied,
                    "change a building's rules",
                    self.building.as_str().to_owned(),
                )
                .with_recovery(format!(
                    "no run may change the rules it is judged by; the User edits {}",
                    rules_path(&self.city_root, &self.building).display()
                )));
            }
        }
        Ok(ToolOutcome {
            result: Payload::new(out)?,
            attachments: Vec::new(),
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
    use kernel::GateSubject;

    fn call(op: &str, text: Option<&str>) -> ToolCall {
        let mut args = Map::new();
        args.insert("op".to_owned(), Value::String(op.to_owned()));
        if let Some(text) = text {
            args.insert("text".to_owned(), Value::String(text.to_owned()));
        }
        ToolCall {
            id: "c1".to_owned(),
            name: ToolName::parse("rules").unwrap(),
            args: Payload::new(args).unwrap(),
        }
    }

    fn tool(root: &Path) -> RulesTool {
        RulesTool::new(root, Address::parse("lab").unwrap()).unwrap()
    }

    fn lay_rules(root: &Path, text: &str) -> PathBuf {
        let path = rules_path(root, &Address::parse("lab").unwrap());
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, text).unwrap();
        path
    }

    #[test]
    fn a_read_shows_the_rules_as_the_user_wrote_them() {
        let dir = tempfile::tempdir().unwrap();
        let text = "confidential = false\nwrite = \"everything\"\nreview = true\n";
        lay_rules(dir.path(), text);
        let read = tool(dir.path()).invoke(&call("read", None)).unwrap();
        assert_eq!(read.result.as_map()["text"], text);
    }

    /// city D23: the bench refuses a rewrite before `invoke`; a caller
    /// that reaches the tool without the bench is refused the same way,
    /// and the rules stand as the User wrote them.
    #[test]
    fn a_rewrite_that_reaches_the_tool_is_refused_and_the_rules_stand() {
        let dir = tempfile::tempdir().unwrap();
        let text = "confidential = false\nwrite = \"everything\"\n";
        let path = lay_rules(dir.path(), text);
        let err = tool(dir.path())
            .invoke(&call(
                "propose",
                Some("confidential = true\nwrite = \"everything\"\n"),
            ))
            .unwrap_err();
        assert_eq!(
            (err.code(), std::fs::read_to_string(&path).unwrap()),
            (&AxCode::GateDenied, text.to_owned())
        );
    }

    /// city D23: what the model reads of this tool offers no operation
    /// the tool always refuses.
    #[test]
    fn the_model_is_offered_the_read_alone() {
        let dir = tempfile::tempdir().unwrap();
        let tool = tool(dir.path());
        let offered = format!(
            "{} {}",
            tool.meta().disclosure,
            Value::Object(tool.meta().params.as_map().clone())
        );
        assert!(!offered.contains("propose"), "{offered}");
    }

    /// The effect layer refuses a Govern call by the scope it names; a
    /// tool that named none was refused as a wiring defect instead, with
    /// a recovery that sent the model to report the tool.
    #[test]
    fn every_call_names_this_buildings_rules_as_the_scope_it_would_rewrite() {
        let dir = tempfile::tempdir().unwrap();
        let tool = tool(dir.path());
        let lab = GateSubject::Scope("building:lab".to_owned());
        assert_eq!(tool.subject(&call("read", None)).unwrap(), lab);
        assert_eq!(
            tool.subject(&call("propose", Some("confidential = false\n")))
                .unwrap(),
            lab
        );
        assert_eq!(
            tool.subject(&call("delete", None)).unwrap_err(),
            tool.invoke(&call("delete", None)).unwrap_err()
        );
    }

    /// Derived from `Kernel.Gate.GovernReads`: over every operation, the
    /// call's effect is `Read` exactly when the operation leaves the
    /// rules as they stand, so a run reads them and still cannot rewrite
    /// them (`crates/kernel/spec/Gate.lean` D26).
    #[test]
    fn only_the_operation_that_reads_is_a_read_and_every_rewrite_still_governs() {
        let dir = tempfile::tempdir().unwrap();
        let tool = tool(dir.path());
        let effects: Vec<(&str, Effect)> = [
            ("read", None),
            (
                "propose",
                Some(
                    "x = 1
",
                ),
            ),
        ]
        .into_iter()
        .map(|(op, text)| (op, tool.effect_of(&call(op, text)).unwrap()))
        .collect();
        assert_eq!(
            effects,
            vec![("read", Effect::Read), ("propose", Effect::Govern)]
        );
        assert_eq!(
            tool.effect_of(&call("delete", None)).unwrap_err(),
            tool.invoke(&call("delete", None)).unwrap_err()
        );
    }

    #[test]
    fn an_unknown_verb_is_refused_and_the_tool_still_answers() {
        let dir = tempfile::tempdir().unwrap();
        let tool = tool(dir.path());
        assert!(tool.invoke(&call("delete", None)).is_err());
        let mut wrong = call("read", None);
        wrong.name = ToolName::parse("status").unwrap();
        assert!(tool.invoke(&wrong).is_err());
        assert_eq!(tool.meta().name.as_str(), "rules");
    }
}
