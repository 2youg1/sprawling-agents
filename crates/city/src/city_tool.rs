// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The face the city itself shows a model (`crates/city/spec/CityTool.lean` §8-23).
//!
//! Three actions on one catalogue line: `list` says what is already
//! here, `raise` lays out a building that is not, and `adopt` draws the
//! city's layout over a directory somebody already has work in. `list`
//! is not a second tool because it is the precondition of the other
//! two — a name that is taken, a directory that is already there — and
//! a model reads every catalogue line every turn.
//!
//! **`Effect::Govern`, not `Effect::Write`.** A building is a top-level
//! directory of the city, and no write domain reaches there. That is
//! not an obstacle to route around: the shape of the city is the
//! person's decision, and `Effect::Govern` is refused at the effect
//! layer (`crates/city/spec/RulesTool.lean` §8-2b) exactly as `crate::rules_tool` is —
//! a run raises nothing; a person does, outside a run.
//!
//! Only City Hall's residents are given this tool
//! ([`crate::vocation`]); whatever the address asking, a building is
//! raised through the `CreateBuilding` command and adopted through the
//! `sprawling adopt` CLI (`crates/city/spec/Building.lean` §8-3).

use std::path::{Path, PathBuf};

use kernel::event::Scope;
use kernel::{
    Address, AxCode, AxError, CostTier, Effect, GateSubject, Payload, RenderIntent, Temporal, Tool,
    ToolCall, ToolMeta, ToolName, ToolOutcome,
};
use serde_json::{Map, Value};

use crate::building::{BuildingTemplate, adopt, all, create};
use crate::policy::{RULES_FILE, rules_path};

/// The tool. It holds where the city is, so a model naming a path
/// cannot move the city it is raising a building in.
pub struct CityTool {
    city_root: PathBuf,
    meta: ToolMeta,
}

/// The one place the three actions are spelled for a caller that got it
/// wrong. A second list would drift from the schema.
const ACTIONS: &str =
    "use `list`, `raise` (name, template) or `adopt` (name, for a directory already here)";

impl CityTool {
    /// # Errors
    /// Propagates a malformed tool name or parameter schema, neither of
    /// which can happen with the literals below.
    pub fn new(city_root: &Path) -> Result<CityTool, AxError> {
        let mut properties = Map::new();
        for (field, description) in [
            (
                "action",
                "`list` | `raise` (name, template) | `adopt` (name)",
            ),
            (
                "name",
                "the building's name: one address segment, not a room inside one",
            ),
            (
                "template",
                "for raise: `minimal` or `confidential`. Left out, it is `minimal`",
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
            Value::Array(vec![Value::String("action".to_owned())]),
        );
        Ok(CityTool {
            city_root: city_root.to_path_buf(),
            meta: ToolMeta {
                name: ToolName::parse("city")?,
                disclosure: "This city's buildings: list them, raise a new one, or adopt a \
                             directory that is already here. Every call here is refused — \
                             the shape of the city is the person's decision, taken outside \
                             a run."
                    .to_owned(),
                params: Payload::new(params)?,
                effect: Effect::Govern,
                cost_tier: CostTier::Light,
                timeout: None,
                render: RenderIntent::Generic,
                temporal: Temporal::Timeless,
            },
        })
    }

    /// The buildings this city has, each with whether anybody has
    /// written rules for it yet. A directory with no `RULES.toml` is
    /// what `adopt` is for, and hiding that from the caller would hide
    /// the one thing it needs to choose a verb.
    fn list(&self) -> Result<Payload, AxError> {
        let mut rows = Vec::new();
        for addr in all(&self.city_root)? {
            let mut row = Map::new();
            row.insert("addr".to_owned(), Value::String(addr.as_str().to_owned()));
            row.insert(
                "has_rules".to_owned(),
                Value::Bool(rules_path(&self.city_root, &addr).is_file()),
            );
            rows.push(Value::Object(row));
        }
        let mut out = Map::new();
        out.insert("buildings".to_owned(), Value::Array(rows));
        out.insert(
            "rules_file".to_owned(),
            Value::String(RULES_FILE.to_owned()),
        );
        Payload::new(out)
    }
}

/// The three things this tool does, each with what it was told.
/// Exhaustive: reading `adopt` as `raise` would lay a template down
/// beside somebody's year of work, and reading `raise` as `adopt` would
/// refuse a building nobody has.
enum Request {
    List,
    Raise {
        addr: Address,
        template: BuildingTemplate,
    },
    Adopt {
        addr: Address,
    },
}

impl Request {
    /// The one reading of a call's arguments, shared by `subject` and
    /// `invoke` so both refuse an unreadable call in the same words.
    fn read(args: &Map<String, Value>) -> Result<Request, AxError> {
        let action = args.get("action").and_then(Value::as_str).ok_or_else(|| {
            AxError::failure(
                AxCode::InvalidArgs,
                "read a city action",
                "missing string argument `action`",
            )
            .with_recovery(ACTIONS)
        })?;
        match action {
            "list" => Ok(Request::List),
            "raise" => {
                let addr = named(args, "raise a building")?;
                let template = match args.get("template").and_then(Value::as_str) {
                    // The ordinary building is what a caller that said
                    // nothing meant; the confidential one is never a
                    // default, because it is a promise about data.
                    None => BuildingTemplate::Minimal,
                    Some(raw) => BuildingTemplate::parse(raw)?,
                };
                Ok(Request::Raise { addr, template })
            }
            "adopt" => Ok(Request::Adopt {
                addr: named(args, "adopt a building")?,
            }),
            other => {
                Err(
                    AxError::failure(AxCode::InvalidArgs, "read a city action", other.to_owned())
                        .with_recovery(ACTIONS),
                )
            }
        }
    }
}

fn named(args: &Map<String, Value>, action: &'static str) -> Result<Address, AxError> {
    let raw = args
        .get("name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .ok_or_else(|| {
            AxError::failure(
                AxCode::InvalidArgs,
                action,
                "missing string argument `name`",
            )
            .with_recovery("pass `name` as the building's one address segment")
        })?;
    Address::parse(raw)
}

impl Tool for CityTool {
    fn meta(&self) -> &ToolMeta {
        &self.meta
    }

    /// Every action reads or changes the shape of the city, so every
    /// call is about the city; a building being raised does not exist
    /// yet to be the scope (`crates/city/spec/RulesTool.lean` §8-36).
    fn subject(&self, call: &ToolCall) -> Result<GateSubject, AxError> {
        Request::read(call.args.as_map())?;
        Ok(GateSubject::Scope(Scope::City.to_string()))
    }

    fn invoke(&self, call: &ToolCall) -> Result<ToolOutcome, AxError> {
        if call.name != self.meta.name {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "read or change the shape of the city",
                format!("call routed to the wrong tool: {}", call.name.as_str()),
            )
            .with_recovery(format!(
                "call `{}`, the name this tool answers to",
                self.meta.name.as_str()
            )));
        }
        let result = match Request::read(call.args.as_map())? {
            Request::List => self.list()?,
            Request::Raise { addr, template } => {
                let building = create(&self.city_root, &addr, template)?;
                let mut out = Map::new();
                out.insert(
                    "addr".to_owned(),
                    Value::String(building.addr().as_str().to_owned()),
                );
                out.insert(
                    "template".to_owned(),
                    Value::String(template.name().to_owned()),
                );
                out.insert("adopted".to_owned(), Value::Bool(false));
                Payload::new(out)?
            }
            Request::Adopt { addr } => {
                let building = adopt(&self.city_root, &addr)?;
                let mut out = Map::new();
                out.insert(
                    "addr".to_owned(),
                    Value::String(building.addr().as_str().to_owned()),
                );
                out.insert(
                    "template".to_owned(),
                    Value::String(BuildingTemplate::Minimal.name().to_owned()),
                );
                out.insert("adopted".to_owned(), Value::Bool(true));
                Payload::new(out)?
            }
        };
        Ok(ToolOutcome {
            result,
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
    use kernel::{AxCode, GateSubject, Payload, Tool, ToolCall, ToolName};
    use serde_json::{Map, Value};

    fn call(fields: &[(&str, &str)]) -> ToolCall {
        let mut args = Map::new();
        for (key, value) in fields {
            args.insert((*key).to_owned(), Value::String((*value).to_owned()));
        }
        ToolCall {
            id: "c1".to_owned(),
            name: ToolName::parse("city").unwrap(),
            args: Payload::new(args).unwrap(),
        }
    }

    #[test]
    fn the_mayor_raises_a_building_and_then_sees_it_on_the_list() {
        let dir = tempfile::tempdir().unwrap();
        let tool = CityTool::new(dir.path()).unwrap();
        let raised = tool
            .invoke(&call(&[("action", "raise"), ("name", "lab")]))
            .unwrap();
        assert_eq!(raised.result.as_map()["addr"], "lab");
        assert!(
            crate::policy::rules_path(dir.path(), &kernel::Address::parse("lab").unwrap())
                .is_file()
        );

        let listed = tool.invoke(&call(&[("action", "list")])).unwrap();
        let buildings = listed.result.as_map()["buildings"].as_array().unwrap();
        assert_eq!(buildings.len(), 1);
        assert_eq!(buildings[0].as_object().unwrap()["addr"], "lab");
    }

    /// Adoption and raising are two verbs over one fact, and reading one
    /// as the other leaves a template beside somebody's work.
    #[test]
    fn adoption_takes_a_directory_that_is_already_there_and_raising_refuses_it() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("imported")).unwrap();
        let tool = CityTool::new(dir.path()).unwrap();
        let adopted = tool
            .invoke(&call(&[("action", "adopt"), ("name", "imported")]))
            .unwrap();
        assert_eq!(adopted.result.as_map()["adopted"], true);

        let again = tool
            .invoke(&call(&[("action", "raise"), ("name", "imported")]))
            .unwrap_err();
        assert_eq!(again.code(), &AxCode::InvalidArgs);
    }

    /// Every action changes or reads the shape of the city, so every
    /// call names the city; a call the grammar cannot read is refused
    /// here exactly as `invoke` would refuse it.
    #[test]
    fn every_call_names_the_city_as_the_scope_it_would_rewrite() {
        let dir = tempfile::tempdir().unwrap();
        let tool = CityTool::new(dir.path()).unwrap();
        let city = GateSubject::Scope("city".to_owned());
        for fields in [
            &[("action", "list")][..],
            &[("action", "raise"), ("name", "kiln")],
            &[("action", "adopt"), ("name", "kiln")],
        ] {
            assert_eq!(tool.subject(&call(fields)).unwrap(), city, "{fields:?}");
        }
        for unread in [
            &[("action", "demolish")][..],
            &[("action", "raise")],
            &[
                ("action", "raise"),
                ("name", "kiln"),
                ("template", "palace"),
            ],
        ] {
            assert_eq!(
                tool.subject(&call(unread)).unwrap_err(),
                tool.invoke(&call(unread)).unwrap_err(),
                "{unread:?}"
            );
        }
        assert!(!dir.path().join("kiln").exists());
    }

    #[test]
    fn an_unknown_action_is_refused_and_says_which_three_exist() {
        let dir = tempfile::tempdir().unwrap();
        let tool = CityTool::new(dir.path()).unwrap();
        let err = tool.invoke(&call(&[("action", "demolish")])).unwrap_err();
        assert!(err.recovery().contains("raise"), "{err}");
        assert_eq!(tool.meta().name.as_str(), "city");
        assert_eq!(tool.meta().effect, kernel::Effect::Govern);
    }
}
