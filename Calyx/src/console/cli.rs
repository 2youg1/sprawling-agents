// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::{Address, Effort};

use super::language::{Line, Reach, help, parse};
use super::stream::Room;
use super::terminal::{Inside, LineKeys, Terminal, serving};

mod offers;

pub(crate) type Say = std::sync::Arc<dyn Fn(String) + Send + Sync>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Next {
    Stay,
    Web,
    Quit,
    AskBeforeQuit(u64),
}

pub(crate) struct Session {
    pub(crate) room: Address,
    model: Option<String>,
    effort: Option<Effort>,
    pub(crate) seen: Room,
    keys: LineKeys,
}

impl Session {
    pub(crate) fn begin() -> Result<Session, kernel::AxError> {
        Ok(Session {
            room: Address::parse(kernel::consts_policy::HALL_MAYOR)?,
            model: None,
            effort: None,
            seen: Room::default(),
            keys: LineKeys::drawn()?,
        })
    }

    pub(crate) fn carry(
        &mut self,
        terminal: &Terminal,
        inside: &Inside,
        typed: &str,
        say: &Say,
    ) -> Next {
        let idem = self.keys.next();
        match parse(typed, idem) {
            Line::Nothing => {}
            Line::Help => say(help(&self.room)),
            Line::OpenWeb => return Next::Web,
            Line::Serving => {
                say(serving(
                    terminal,
                    metrics(inside).as_ref(),
                    std::process::id(),
                ));
            }
            Line::Quit => {
                return match metrics(inside).map_or(0, |vitals| vitals.runs_active) {
                    0 => Next::Quit,
                    going => Next::AskBeforeQuit(going),
                };
            }
            Line::Select(addr) => {
                say(format!("  plain lines go to {}", addr.as_str()));
                self.room = addr;
                self.seen.clear();
            }
            Line::New => self.post(
                inside,
                wire::WireCommand::OpenSession {
                    addr: self.room.clone(),
                    carry: wire::Carry::default(),
                    from: None,
                    idem,
                },
                say,
            ),
            Line::Stop => match self.seen.run {
                Some(run) => self.post(inside, wire::WireCommand::Cancel { run, idem }, say),
                None => say(format!("  no run is working in {}", self.room.as_str())),
            },
            Line::Halt(reach) => {
                let scope = self.scope(reach);
                self.post(inside, wire::WireCommand::Halt { scope, idem }, say);
            }
            Line::Release(reach) => {
                let scope = self.scope(reach);
                self.post(inside, wire::WireCommand::Release { scope, idem }, say);
            }
            Line::Model(None) => say(format!(
                "  plain lines ask {}",
                self.model
                    .as_deref()
                    .unwrap_or("the model the city's `main` names")
            )),
            Line::Model(Some(id)) => {
                say(format!("  plain lines now ask {id}"));
                self.model = Some(id);
            }
            Line::Effort(asked) => self.effort(asked.as_deref(), say),
            Line::Approve(item) => self.answer(inside, item, kernel::Ruling::Allow, say),
            Line::Deny(item) => self.answer(inside, item, kernel::Ruling::Deny, say),
            Line::Acp(None) => answer(inside, wire::Query::AgentCatalog, say),
            Line::Acp(Some(text)) => answer(inside, wire::Query::ParseAgentSpec { text }, say),
            Line::Remote(tail) => say((inside.carry_remote)(&tail)),
            Line::Unknown { verb, nearest } => {
                say(format!("  no verb `/{verb}`"));
                if !nearest.is_empty() {
                    say(format!("  did you mean: {}", nearest.join(", ")));
                }
            }
            Line::Malformed { verb, reason } => {
                say(format!("  `/wire {verb}` cannot be read: {reason}"));
                say("  the body is the wire's own JSON; `idem` may be left out".to_owned());
            }
            Line::Frame(frame) => match *frame {
                wire::ClientFrame::Command(command) => self.post(inside, *command, say),
                wire::ClientFrame::Ask(ask) => answer(inside, ask.query, say),
                wire::ClientFrame::Hello(_) | wire::ClientFrame::Monitor(_) => {
                    say("  this console is already inside the city".to_owned());
                }
            },
            Line::Work(task) => {
                let dispatch = wire::WireCommand::Dispatch {
                    addr: self.room.clone(),
                    task,
                    goal: String::new(),
                    policy: wire::RunPolicy::of(wire::Mode::Work),
                    idem,
                    session: None,
                    resume: None,
                    images: Vec::new(),
                    effort: self.effort,
                    model: self.model.clone(),
                };
                self.post(inside, dispatch, say);
            }
        }
        Next::Stay
    }

    fn scope(&self, reach: Reach) -> wire::HaltScope {
        let building = self
            .room
            .as_str()
            .split_once('/')
            .and_then(|(building, _)| Address::parse(building).ok());
        match (reach, building) {
            (Reach::Building, Some(building)) => wire::HaltScope::Building(building),
            (Reach::Building, None) => wire::HaltScope::Building(self.room.clone()),
            (Reach::City, _) => wire::HaltScope::City,
        }
    }

    fn effort(&mut self, asked: Option<&str>, say: &Say) {
        let offered = || {
            Effort::ALL
                .into_iter()
                .filter(|level| *level != Effort::None)
                .map(Effort::as_str)
                .collect::<Vec<_>>()
                .join(", ")
        };
        match asked {
            None => say(format!(
                "  plain lines think at {}",
                self.effort
                    .map_or("the level the room and the model settle on", Effort::as_str)
            )),
            Some(word) => match Effort::ALL
                .into_iter()
                .find(|level| *level != Effort::None && level.as_str() == word)
            {
                Some(level) => {
                    say(format!("  plain lines now think at {}", level.as_str()));
                    self.effort = Some(level);
                }
                None => say(format!(
                    "  no thinking level `{word}`; one of {}",
                    offered()
                )),
            },
        }
    }

    fn answer(
        &mut self,
        inside: &Inside,
        item: Option<String>,
        verdict: kernel::Ruling,
        say: &Say,
    ) {
        let idem = self.keys.next();
        let item = match item {
            Some(id) => kernel::ApprovalId::new(id),
            None => self.seen.waiting.first().map(|waiting| waiting.id.clone()),
        };
        let Some(item) = item else {
            say(format!("  nothing is waiting in {}", self.room.as_str()));
            return;
        };
        self.seen.waiting.retain(|waiting| waiting.id != item);
        self.post(
            inside,
            wire::WireCommand::Approve {
                item,
                verdict,
                idem,
            },
            say,
        );
    }

    fn post(&self, inside: &Inside, command: wire::WireCommand, say: &Say) {
        let refused = std::sync::Arc::clone(say);
        inside.desk.post(
            command.into(),
            wire::Reply::to(move |error: kernel::AxError| {
                refused(format!("  {error}"));
                refused(format!("  {}", error.recovery()));
                wire::Delivered::ToThePeer
            }),
        );
    }
}

pub(super) fn metrics(inside: &Inside) -> Option<wire::MetricsAnswer> {
    match (inside.answering)(wire::Query::Metrics) {
        (_, Ok(wire::Answer::Metrics(vitals))) => Some(*vitals),
        (_, Ok(_) | Err(_)) => None,
    }
}

fn answer(inside: &Inside, query: wire::Query, say: &Say) {
    let (_as_of, answered) = (inside.answering)(query);
    match answered {
        Ok(answer) => match serde_json::to_string(&answer) {
            Ok(text) => say(text),
            Err(err) => say(format!("  the answer could not be rendered: {err}")),
        },
        Err(error) => {
            say(format!("  {error}"));
            say(format!("  {}", error.recovery()));
        }
    }
}
