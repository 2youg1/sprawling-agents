// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::path::{Path, PathBuf};

use kernel::event::Scope;
use kernel::{
    Address, AxCode, AxError, CostTier, Effect, GateSubject, Payload, RenderIntent, Temporal, Tool,
    ToolCall, ToolDef, ToolMeta, ToolName, ToolOutcome,
};
use serde_json::{Map, Value};

use crate::policy::{RULES_FILE, rules_path};

pub struct RulesTool {
    city_root: PathBuf,
    building: Address,
    meta: ToolMeta,
}

impl RulesTool {
    pub fn definition() -> Result<ToolDef, AxError> {
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
        Ok(ToolDef {
            name: ToolName::parse("rules")?,
            description: format!(
                "What this building's runs are judged by. `read` shows them. No run may \
                 change them: the User edits the {RULES_FILE}."
            ),
            input_schema: Payload::new(params)?,
        })
    }

    pub fn new(city_root: &Path, building: Address) -> Result<RulesTool, AxError> {
        let defined = Self::definition()?;
        Ok(RulesTool {
            city_root: city_root.to_path_buf(),
            building,
            meta: ToolMeta {
                name: defined.name,
                disclosure: defined.description,
                params: defined.input_schema,
                effect: Effect::Govern,
                cost_tier: CostTier::Light,
                timeout: None,
                render: RenderIntent::Generic,
                temporal: Temporal::Timeless,
            },
        })
    }

    fn standing(&self) -> Result<String, AxError> {
        let path = rules_path(&self.city_root, &self.building);
        match std::fs::read_to_string(&path) {
            Ok(text) => Ok(text),

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

enum Op {
    Read,
    Rewrite,
}

impl Op {
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

    fn subject(&self, call: &ToolCall) -> Result<GateSubject, AxError> {
        Op::read(call.args.as_map())?;
        Ok(GateSubject::Scope(
            Scope::Building(self.building.clone()).to_string(),
        ))
    }

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
