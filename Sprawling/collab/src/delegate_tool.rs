// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{
    Address, AxCode, AxError, CostTier, DelegateKind, Depth, Difficulty, Effect, GateOutcome,
    Payload, RenderIntent, Temporal, Tool, ToolCall, ToolDef, ToolMeta, ToolName, ToolOutcome,
};
use serde_json::{Map, Value};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Delegated {
    pub room: Address,
    pub task: String,
    pub goal: String,
    pub kind: DelegateKind,

    pub difficulty: Option<Difficulty>,
}

#[derive(Clone)]
pub struct StartCheck(Arc<Judge>);

type Judge = dyn Fn(&Address) -> Result<(), AxError> + Send + Sync;

impl StartCheck {
    #[must_use]
    pub fn new(
        judge: impl Fn(&Address) -> Result<(), AxError> + Send + Sync + 'static,
    ) -> StartCheck {
        StartCheck(Arc::new(judge))
    }
}

impl std::fmt::Debug for StartCheck {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("StartCheck")
    }
}

#[derive(Debug)]
pub struct DelegateDesk {
    depth: Depth,

    building: Address,

    check: StartCheck,
    asked: Vec<Delegated>,

    handed_over: usize,
}

impl DelegateDesk {
    #[must_use]
    pub fn new(depth: Depth, building: Address, check: StartCheck) -> DelegateDesk {
        DelegateDesk {
            depth,
            building,
            check,
            asked: Vec::new(),
            handed_over: 0,
        }
    }

    #[must_use]
    pub fn beside(&self) -> DelegateDesk {
        DelegateDesk::new(self.depth, self.building.clone(), self.check.clone())
    }

    pub fn ask(&mut self, work: Delegated) -> Result<&Delegated, AxError> {
        match kernel::gate::spawn(self.depth, &work.kind) {
            GateOutcome::Allow => {}
            GateOutcome::Deny { refusal } => return Err(*refusal),

            GateOutcome::Ask { question } => return Err(*question),
        }
        if !work.room.is_within(&self.building) {
            return Err(AxError::failure(
                AxCode::CrossBuildingDenied,
                "delegate work",
                work.room.as_str().to_owned(),
            )
            .with_recovery(format!(
                "put the delegate in a room inside {}; work in another building is asked for \
                 with `signal`",
                self.building.as_str()
            )));
        }
        (self.check.0)(&work.room)?;
        self.asked.push(work);
        self.asked.last().ok_or_else(|| {
            AxError::failure(AxCode::InvalidArgs, "delegate work", "the request vanished")
                .with_recovery(
                    "report this against collab::delegate_tool: the request was pushed \
                     onto the queue one line above and the queue reads back empty",
                )
        })
    }

    #[must_use]
    pub fn asked(&self) -> &[Delegated] {
        &self.asked
    }

    pub fn hand_over(&mut self) -> Vec<Delegated> {
        let fresh: Vec<Delegated> = self.asked.iter().skip(self.handed_over).cloned().collect();
        self.handed_over = self.asked.len();
        fresh
    }

    pub fn take(&mut self) -> Vec<Delegated> {
        self.handed_over = 0;
        std::mem::take(&mut self.asked)
    }
}

pub struct DelegateTool {
    desk: std::sync::Arc<std::sync::Mutex<DelegateDesk>>,
    meta: ToolMeta,
}

impl DelegateTool {
    pub fn definition() -> Result<ToolDef, AxError> {
        let mut properties = Map::new();
        for (field, description) in [
            ("room", "where the delegate works, as building/room"),
            ("task", "one line: what the delegate is to produce"),
            (
                "goal",
                "what counts as done and when to stop; a delegate without one does not stop",
            ),
            (
                "kind",
                "`ephemeral` for one piece of work, `resident` for somebody who stays. \
                 Defaults to ephemeral",
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
        properties.insert(
            "difficulty".to_owned(),
            Value::Object(Map::from_iter([
                ("type".to_owned(), Value::String("string".to_owned())),
                (
                    "enum".to_owned(),
                    Value::Array(
                        ["easy", "medium", "hard"]
                            .into_iter()
                            .map(|word| Value::String(word.to_owned()))
                            .collect(),
                    ),
                ),
                (
                    "description".to_owned(),
                    Value::String(
                        "how difficult this subtask is: easy, medium or hard; the city chooses \
                         the model and effort from the workspace policy"
                            .to_owned(),
                    ),
                ),
            ])),
        );
        let mut params = Map::new();
        params.insert("type".to_owned(), Value::String("object".to_owned()));
        params.insert("properties".to_owned(), Value::Object(properties));
        params.insert(
            "required".to_owned(),
            Value::Array(
                ["room", "task", "goal"]
                    .into_iter()
                    .map(|field| Value::String(field.to_owned()))
                    .collect(),
            ),
        );
        Ok(ToolDef {
            name: ToolName::parse("delegate")?,
            description: "Hand one piece of work to another agent in this building, one \
                          level down. Nobody is asked: one level is the whole rule, and \
                          the types hold it, so the call either names the room the work \
                          will happen in or is refused."
                .to_owned(),
            input_schema: Payload::new(params)?,
        })
    }

    pub fn new(
        desk: std::sync::Arc<std::sync::Mutex<DelegateDesk>>,
    ) -> Result<DelegateTool, AxError> {
        let defined = Self::definition()?;
        Ok(DelegateTool {
            desk,
            meta: ToolMeta {
                name: defined.name,
                disclosure: defined.description,
                params: defined.input_schema,
                effect: Effect::Spawn,
                cost_tier: CostTier::Heavy,
                timeout: None,
                render: RenderIntent::Delegate,
                temporal: Temporal::Timeless,
            },
        })
    }
}

fn arg<'a>(args: &'a Map<String, Value>, key: &str) -> Result<&'a str, AxError> {
    args.get(key).and_then(Value::as_str).ok_or_else(|| {
        AxError::failure(
            AxCode::InvalidArgs,
            "delegate work",
            format!("missing string argument `{key}`"),
        )
        .with_recovery("pass room, task and goal - all strings")
    })
}

impl Tool for DelegateTool {
    fn meta(&self) -> &ToolMeta {
        &self.meta
    }

    fn invoke(&self, call: &ToolCall) -> Result<ToolOutcome, AxError> {
        if call.name != self.meta.name {
            return Err(AxError::failure(
                AxCode::InvalidArgs,
                "delegate work",
                format!("call routed to the wrong tool: {}", call.name.as_str()),
            )
            .with_recovery(format!(
                "call `{}`, the name this tool answers to",
                self.meta.name.as_str()
            )));
        }
        let args = call.args.as_map();
        let room = Address::parse(arg(args, "room")?)?;

        let kind = match args.get("kind").and_then(Value::as_str) {
            None | Some("ephemeral") => DelegateKind::Ephemeral,
            Some("resident") => DelegateKind::Resident,
            Some(other) => {
                return Err(AxError::failure(
                    AxCode::InvalidArgs,
                    "delegate work",
                    format!("no such delegate kind: {other}"),
                )
                .with_recovery("use `ephemeral` for one piece of work, or `resident`"));
            }
        };
        let difficulty = match args.get("difficulty") {
            None => None,
            Some(Value::String(word)) if word == "easy" => Some(Difficulty::Easy),
            Some(Value::String(word)) if word == "medium" => Some(Difficulty::Medium),
            Some(Value::String(word)) if word == "hard" => Some(Difficulty::Hard),
            Some(other) => {
                return Err(AxError::failure(
                    AxCode::InvalidArgs,
                    "delegate work",
                    format!("no such task difficulty: {other}"),
                )
                .with_recovery("pass difficulty as `easy`, `medium` or `hard`, or omit it"));
            }
        };
        let work = Delegated {
            room,
            task: arg(args, "task")?.to_owned(),
            goal: arg(args, "goal")?.to_owned(),
            kind,
            difficulty,
        };
        let mut desk = self.desk.lock().map_err(|_| {
            AxError::failure(
                AxCode::StorageFatal,
                "delegate work",
                "the desk was left locked by a thread that died",
            )
            .with_recovery(
                "end this run and resume it: the delegation desk cannot be reached again \
                 inside a process where a thread died holding it",
            )
        })?;
        let accepted = desk.ask(work)?;
        let mut out = Map::new();
        out.insert(
            "room".to_owned(),
            Value::String(accepted.room.as_str().to_owned()),
        );
        out.insert(
            "starts".to_owned(),
            Value::String("now: the city starts it while this run goes on".to_owned()),
        );
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

    fn desk(depth: Depth) -> std::sync::Arc<std::sync::Mutex<DelegateDesk>> {
        std::sync::Arc::new(std::sync::Mutex::new(DelegateDesk::new(
            depth,
            Address::parse("lab").unwrap(),
            StartCheck::new(|_room: &Address| Ok(())),
        )))
    }

    fn call(room: &str, kind: Option<&str>) -> ToolCall {
        let mut args = Map::new();
        args.insert("room".to_owned(), Value::String(room.to_owned()));
        args.insert("task".to_owned(), Value::String("measure it".to_owned()));
        args.insert(
            "goal".to_owned(),
            Value::String("a number, then stop".to_owned()),
        );
        if let Some(kind) = kind {
            args.insert("kind".to_owned(), Value::String(kind.to_owned()));
        }
        ToolCall {
            id: "call-1".to_owned(),
            name: ToolName::parse("delegate").unwrap(),
            args: Payload::new(args).unwrap(),
        }
    }

    #[test]
    fn the_parent_supplies_only_a_closed_task_difficulty() {
        let desk = desk(Depth::Root);
        let tool = DelegateTool::new(std::sync::Arc::clone(&desk)).unwrap();
        let mut request = call("lab/helper", None);
        let mut args = request.args.as_map().clone();
        args.insert("difficulty".to_owned(), Value::String("easy".to_owned()));
        request.args = Payload::new(args).unwrap();
        tool.invoke(&request).unwrap();
        assert_eq!(
            desk.lock().unwrap().asked()[0].difficulty,
            Some(Difficulty::Easy)
        );

        for invalid in [
            Value::from(3),
            Value::Null,
            Value::Bool(true),
            Value::String("tiny".to_owned()),
        ] {
            args = request.args.as_map().clone();
            args.insert("difficulty".to_owned(), invalid);
            request.args = Payload::new(args).unwrap();
            assert_eq!(
                tool.invoke(&request).unwrap_err().code(),
                &AxCode::InvalidArgs
            );
        }
        assert_eq!(desk.lock().unwrap().asked().len(), 1);
    }

    #[test]
    fn the_delegate_tool_declares_the_delegate_render_intent() {
        let tool = DelegateTool::new(desk(Depth::Root)).unwrap();
        assert_eq!(tool.meta().render, RenderIntent::Delegate);
    }

    #[test]
    fn a_root_run_may_hand_work_down_and_is_told_where_it_will_happen() {
        let desk = desk(Depth::Root);
        let tool = DelegateTool::new(std::sync::Arc::clone(&desk)).unwrap();
        let outcome = tool.invoke(&call("lab/helper", None)).unwrap();
        assert_eq!(outcome.result.as_map()["room"], "lab/helper");
        assert_eq!(
            desk.lock().unwrap().asked().len(),
            1,
            "the desk answers what has been asked for before it is taken"
        );

        let taken = desk.lock().unwrap().take();
        assert_eq!(taken.len(), 1);
        assert_eq!(taken[0].kind, DelegateKind::Ephemeral);
        assert!(desk.lock().unwrap().take().is_empty(), "taken once");
    }

    #[test]
    fn the_answer_says_the_delegate_starts_while_this_run_goes_on() {
        let tool = DelegateTool::new(desk(Depth::Root)).unwrap();
        let outcome = tool.invoke(&call("lab/helper", None)).unwrap();
        let starts = outcome.result.as_map()["starts"].as_str().unwrap();
        assert!(!starts.contains("settles"), "{starts}");
    }

    #[test]
    fn a_desk_a_dead_thread_held_is_refused_in_one_plain_line() {
        let desk = desk(Depth::Root);
        let held = std::sync::Arc::clone(&desk);
        std::thread::spawn(move || {
            let _guard = held.lock().unwrap();
            panic!("die holding the desk");
        })
        .join()
        .unwrap_err();
        let err = DelegateTool::new(desk)
            .unwrap()
            .invoke(&call("lab/helper", None))
            .unwrap_err();
        assert_eq!(err.code(), &AxCode::StorageFatal);
        assert!(
            !err.recovery().contains('\n') && !err.recovery().contains("  "),
            "{:?}",
            err.recovery()
        );
    }

    #[test]
    fn a_delegate_cannot_delegate_and_the_refusal_says_what_to_do_instead() {
        let desk = desk(Depth::Delegated);
        let tool = DelegateTool::new(std::sync::Arc::clone(&desk)).unwrap();
        for kind in [None, Some("ephemeral"), Some("resident")] {
            let err = tool.invoke(&call("lab/helper", kind)).unwrap_err();
            assert_eq!(err.code(), &AxCode::DelegationDepth);
            let refusal = err.gate().expect("a gate refusal has three parts");
            assert!(refusal.alternative().contains("return this subtask"));
        }
        assert!(desk.lock().unwrap().take().is_empty());
    }

    #[test]
    fn a_delegate_stays_inside_the_building_that_asked_for_it() {
        let desk = desk(Depth::Root);
        let tool = DelegateTool::new(std::sync::Arc::clone(&desk)).unwrap();
        let err = tool.invoke(&call("shop/helper", None)).unwrap_err();
        assert_eq!(err.code(), &AxCode::CrossBuildingDenied);
        assert!(err.recovery().contains("signal"));
    }

    #[test]
    fn a_room_the_city_could_not_start_is_refused_at_the_call() {
        let refusal =
            AxError::failure(AxCode::ConfigInvalid, "dispatch work", "lab/helper: claude")
                .with_recovery("name an agent the city knows");
        let answer = refusal.clone();
        let desk = std::sync::Arc::new(std::sync::Mutex::new(DelegateDesk::new(
            Depth::Root,
            Address::parse("lab").unwrap(),
            StartCheck::new(move |_room: &Address| Err(answer.clone())),
        )));
        let tool = DelegateTool::new(std::sync::Arc::clone(&desk)).unwrap();
        assert_eq!(tool.invoke(&call("lab/helper", None)).err(), Some(refusal));
        assert!(desk.lock().unwrap().hand_over().is_empty());
    }

    #[test]
    fn an_unknown_kind_is_refused_rather_than_rounded_down() {
        let desk = desk(Depth::Root);
        let tool = DelegateTool::new(std::sync::Arc::clone(&desk)).unwrap();
        let err = tool
            .invoke(&call("lab/helper", Some("apprentice")))
            .unwrap_err();
        assert_eq!(err.code(), &AxCode::InvalidArgs);
    }

    #[test]
    fn the_tool_refuses_another_tools_call_and_still_answers() {
        let desk = desk(Depth::Root);
        let tool = DelegateTool::new(desk).unwrap();
        let mut wrong = call("lab/helper", None);
        wrong.name = ToolName::parse("signal").unwrap();
        let err = tool.invoke(&wrong).unwrap_err();
        assert_eq!(err.code(), &AxCode::InvalidArgs);
        assert_eq!(tool.meta().name.as_str(), "delegate");
    }
}
