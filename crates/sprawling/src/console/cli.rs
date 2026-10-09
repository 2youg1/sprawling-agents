// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! What a submitted line does: the slash verb carried out, plain words
//! dispatched to the chosen room (`crates/sprawling/spec/Console.lean`
//! §8-11, §8-21).
//!
//! Both faces that take lines - the raw CLI and the line console a
//! harness drives - hand each line to [`Session::carry`], so the two
//! answer one line the same way. A command goes onto the `CommandDesk`
//! a browser's frames land on and a question goes into the `Answering`
//! function the socket calls, so this console decides nothing the server
//! does not. What the lifecycle must do - open the page, close the city -
//! comes back as [`Next`] for the face to carry out.

use kernel::{Address, Effort};

use super::language::{Line, Reach, help, parse};
use super::stream::Room;
use super::terminal::{Inside, LineKeys, Terminal, serving};

/// Where a line meant for the person goes: the UI thread's channel.
pub(crate) type Say = std::sync::Arc<dyn Fn(String) + Send + Sync>;

/// What the face does after a line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Next {
    Stay,
    /// `/web`: open the page; the CLI also goes quiet.
    Web,
    /// `/quit` with no run going.
    Quit,
    /// `/quit` with runs going: the face asks how they end.
    AskBeforeQuit(u64),
}

/// What one console remembers between lines.
pub(crate) struct Session {
    pub(crate) room: Address,
    /// The model plain lines ask, when `/model` named one.
    model: Option<String>,
    /// The thinking level plain lines ask for, when `/effort` named one.
    effort: Option<Effort>,
    pub(crate) seen: Room,
    keys: LineKeys,
}

impl Session {
    /// A session in the city's front room.
    ///
    /// # Errors
    /// The entropy source refusing an origin for this console's keys.
    pub(crate) fn begin() -> Result<Session, kernel::AxError> {
        Ok(Session {
            room: Address::parse(kernel::consts_policy::HALL_MAYOR)?,
            model: None,
            effort: None,
            seen: Room::default(),
            keys: LineKeys::drawn()?,
        })
    }

    /// One submitted line, carried out.
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
            Line::Remote(line) => say(crate::outside::console::carry(&inside.remote, line)),
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
                    // The room is already chosen; a line typed after it
                    // continues what is working there.
                    session: None,
                    effort: self.effort,
                    model: self.model.clone(),
                };
                self.post(inside, dispatch, say);
            }
        }
        Next::Stay
    }

    /// `y` or `n` with an empty line: the oldest waiting request.
    pub(crate) fn answer_waiting(
        &mut self,
        inside: &Inside,
        verdict: kernel::Ruling,
        say: &Say,
    ) -> bool {
        if self.seen.waiting.is_empty() {
            return false;
        }
        self.answer(inside, None, verdict, say);
        true
    }

    /// Esc on an empty line: the run working in this room is cancelled,
    /// as Pi's interrupt does.
    pub(crate) fn interrupt(&mut self, inside: &Inside, say: &Say) {
        if let Some(run) = self.seen.run {
            let idem = self.keys.next();
            self.post(inside, wire::WireCommand::Cancel { run, idem }, say);
            say(format!("  stopping the run in {}", self.room.as_str()));
        }
    }

    /// What Tab may put after `/model` or `/effort`, from one read of
    /// the attached endpoints: every model id served, or the thinking
    /// levels the current (Endpoint, model) offers. Any other verb, or a
    /// city that cannot answer, offers nothing.
    pub(crate) fn arguments(&self, inside: &Inside, verb: wire::Slash) -> Vec<String> {
        let book = || match (inside.answering)(wire::Query::EndpointView) {
            (_, Ok(wire::Answer::Endpoints(book))) => Some(book),
            (_, Ok(_) | Err(_)) => None,
        };
        match verb {
            wire::Slash::Model => {
                let Some(book) = book() else {
                    return Vec::new();
                };
                let mut ids: Vec<String> = book
                    .endpoints
                    .iter()
                    .flat_map(|endpoint| endpoint.models.iter().map(|model| model.id.clone()))
                    .collect();
                ids.sort();
                ids.dedup();
                ids
            }
            wire::Slash::Effort => book()
                .as_ref()
                .and_then(|book| self.current_offer(book))
                .map_or_else(Vec::new, |model| {
                    model
                        .thinking
                        .levels
                        .iter()
                        .map(|level| level.as_str().to_owned())
                        .collect()
                }),
            wire::Slash::Help
            | wire::Slash::Room
            | wire::Slash::New
            | wire::Slash::Stop
            | wire::Slash::Halt
            | wire::Slash::Release
            | wire::Slash::Approve
            | wire::Slash::Deny
            | wire::Slash::Web
            | wire::Slash::Quit
            | wire::Slash::Serving
            | wire::Slash::Remote
            | wire::Slash::Acp
            | wire::Slash::Wire => Vec::new(),
        }
    }

    /// The model plain lines ask, on the endpoint that serves it: the
    /// one `/model` chose, looked for first where `main` points, else the
    /// one `main` names.
    fn current_offer<'a>(
        &self,
        book: &'a wire::EndpointsAnswer,
    ) -> Option<&'a wire::ModelFactsSummary> {
        let main = book
            .chosen
            .iter()
            .find(|chosen| chosen.tag == kernel::ModelTag::Main);
        let wanted = self
            .model
            .as_deref()
            .or(main.map(|chosen| chosen.model.as_str()))?;
        let served = |endpoint: &'a wire::EndpointSummary| {
            endpoint.models.iter().find(|model| model.id == wanted)
        };
        book.endpoints
            .iter()
            .filter(|endpoint| main.is_some_and(|chosen| chosen.endpoint == endpoint.name))
            .find_map(served)
            .or_else(|| book.endpoints.iter().find_map(served))
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

    /// Answers one waiting request, or the oldest, under a key of its
    /// own: `y` with an empty line minted none.
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
            None => self.seen.waiting.first().cloned(),
        };
        let Some(item) = item else {
            say(format!("  nothing is waiting in {}", self.room.as_str()));
            return;
        };
        self.seen.waiting.retain(|waiting| *waiting != item);
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

    /// One command, onto the same desk a browser's frames land on; a
    /// refusal comes back as a line under the one that caused it.
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

/// The city's counts, or nothing when it is too busy to answer.
fn metrics(inside: &Inside) -> Option<wire::MetricsAnswer> {
    match (inside.answering)(wire::Query::Metrics) {
        (_, Ok(wire::Answer::Metrics(vitals))) => Some(*vitals),
        (_, Ok(_) | Err(_)) => None,
    }
}

/// One question, answered where it was asked, as one JSON line - the
/// shape `sprawling call` prints.
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
