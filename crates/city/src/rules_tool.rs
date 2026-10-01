// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The face a building's rules show a model: read them, and propose the
//! whole of them again.
//!
//! **Why this is not `edit`.** `RULES.toml` lives in the building's
//! reserved subtree, and no write domain reaches there — which is not an
//! oversight to work around but the rule itself: a run may not quietly
//! widen what it is allowed to do. The declaration is `Effect::Govern`,
//! and that effect is refused at the effect layer (city-SPEC section
//! 8-2b): a run may not change what governs it, so every call comes
//! back as a refusal and a person edits the file.
//!
//! **Whole document, not a patch.** These rules are evaluated as one
//! text — a confidential building may list no egress domains, so two
//! lines can be legal apart and illegal together. A proposal is
//! therefore the complete file, evaluated before a byte is written, and
//! a file that does not evaluate is refused rather than half-applied.

use std::path::{Path, PathBuf};

use kernel::event::Scope;
use kernel::{
    Address, AxCode, AxError, CostTier, Effect, GateSubject, Payload, RenderIntent, Temporal, Tool,
    ToolCall, ToolMeta, ToolName, ToolOutcome,
};
use serde_json::{Map, Value};

use crate::policy::{RULES_FILE, rules_path, write_rules};

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
        let mut properties = Map::new();
        for (field, description) in [
            (
                "op",
                "`read` for the rules as they stand, `propose` to replace them",
            ),
            (
                "text",
                "for propose: the whole of the new RULES.toml. It is evaluated before anything \
                 is written, and a document that does not evaluate is refused",
            ),
        ] {
            let mut spec = Map::new();
            spec.insert("type".to_owned(), Value::String("string".to_owned()));
            spec.insert(
                "description".to_owned(),
                Value::String(description.to_owned()),
            );
            properties.insert(field.to_owned(), Value::Object(spec));
        }
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
                    "What this building's runs are judged by. No run may change it: every \
                     call here is refused, and a person edits the {RULES_FILE}."
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

/// The two things this tool does. Exhaustive: an unknown verb is refused
/// rather than read as the harmless one, because guessing wrong here
/// means either a silent read where a rewrite was meant or the reverse.
enum Op<'a> {
    Read,
    /// The whole new document.
    Propose(&'a str),
}

impl Op<'_> {
    /// The one reading of a call's arguments, shared by `subject` and
    /// `invoke` so both refuse an unreadable call in the same words.
    fn read(args: &Map<String, Value>) -> Result<Op<'_>, AxError> {
        match arg(args, "op")? {
            "read" => Ok(Op::Read),
            "propose" => Ok(Op::Propose(arg(args, "text")?)),
            other => Err(AxError::failure(
                AxCode::InvalidArgs,
                "read or change a building's rules",
                format!("no such operation: {other}"),
            )
            .with_recovery("`read`, or `propose` with the whole new document in `text`")),
        }
    }
}

fn arg<'a>(args: &'a Map<String, Value>, key: &str) -> Result<&'a str, AxError> {
    args.get(key).and_then(Value::as_str).ok_or_else(|| {
        AxError::failure(
            AxCode::InvalidArgs,
            "read or change a building's rules",
            format!("missing string argument `{key}`"),
        )
        .with_recovery("`read`, or `propose` with the whole new document in `text`")
    })
}

impl Tool for RulesTool {
    fn meta(&self) -> &ToolMeta {
        &self.meta
    }

    /// This building's rules, read or rewritten, are what every call is
    /// about (city-SPEC.md section 8-36).
    fn subject(&self, call: &ToolCall) -> Result<GateSubject, AxError> {
        Op::read(call.args.as_map())?;
        Ok(GateSubject::Scope(
            Scope::Building(self.building.clone()).to_string(),
        ))
    }

    fn invoke(&self, call: &ToolCall) -> Result<ToolOutcome, AxError> {
        if call.name != self.meta.name {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "read or change a building's rules",
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
            Op::Propose(text) => {
                let rules = write_rules(&self.city_root, &self.building, text)?;
                out.insert(
                    "confidential".to_owned(),
                    Value::Bool(rules.policy().confidential),
                );
                out.insert("review".to_owned(), Value::Bool(rules.review()));
                out.insert(
                    "takes_effect".to_owned(),
                    Value::String("on the next run in this building".to_owned()),
                );
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

    #[test]
    fn a_proposal_that_evaluates_becomes_the_rules_the_next_run_is_judged_by() {
        let dir = tempfile::tempdir().unwrap();
        let tool = tool(dir.path());
        let outcome = tool
            .invoke(&call(
                "propose",
                Some(
                    "confidential = false\nwrite = \"everything\"\nreview = true\n\
                     prefixes = [\"lab\"]\n",
                ),
            ))
            .unwrap();
        assert_eq!(outcome.result.as_map()["review"], true);

        let reloaded = crate::policy::load(dir.path(), &Address::parse("lab").unwrap()).unwrap();
        assert!(reloaded.review());
        let read_back = tool.invoke(&call("read", None)).unwrap();
        assert!(
            read_back.result.as_map()["text"]
                .as_str()
                .unwrap()
                .contains("review = true")
        );
    }

    /// The reason the whole document is the unit: two lines legal apart
    /// and illegal together. Nothing is written when they are.
    #[test]
    fn a_proposal_that_does_not_evaluate_leaves_the_old_rules_standing() {
        let dir = tempfile::tempdir().unwrap();
        let tool = tool(dir.path());
        tool.invoke(&call(
            "propose",
            Some("confidential = false\nwrite = \"everything\"\n"),
        ))
        .unwrap();

        let err = tool
            .invoke(&call(
                "propose",
                Some(
                    "confidential = true\nwrite = \"everything\"\n\
                     egress = [\"example.com\"]\n",
                ),
            ))
            .unwrap_err();
        assert_eq!(err.code(), &AxCode::ConfigInvalid);

        let standing = crate::policy::load(dir.path(), &Address::parse("lab").unwrap()).unwrap();
        assert!(
            !standing.policy().confidential,
            "a refused proposal changed the building anyway"
        );
    }

    #[test]
    fn a_document_that_does_not_say_whether_it_is_confidential_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let tool = tool(dir.path());
        let err = tool
            .invoke(&call(
                "propose",
                Some("write = \"everything\"\nreview = false\n"),
            ))
            .unwrap_err();
        assert_eq!(err.code(), &AxCode::ConfigInvalid);
        assert!(!rules_path(dir.path(), &Address::parse("lab").unwrap()).exists());
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
