// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use crate::scene::Scene;
use console_ffi::{CityConfig, CityHost, run_city_ui};

use super::cli::{Next, Say, Session};
use super::lifecycle::{Asker, Event, Face, Surface};
use super::local_time;
use super::terminal::{Inside, Started, Terminal};
use wire::CloseMode;

const UI_DEPTH: usize = 256;

#[repr(u32)]
enum HostEventKind {
    Empty = 0,
    Face = 1,
    Notice = 2,
    Pairing = 3,
    Entries = 4,
    LiveMeta = 5,
    #[expect(
        dead_code,
        reason = "ABI parity with Zig HostEventKind.lost; host pushes Asking/Notice instead"
    )]
    Lost = 6,
    Asking = 7,
}

impl HostEventKind {
    const fn code(self) -> u32 {
        match self {
            Self::Empty => 0,
            Self::Face => 1,
            Self::Notice => 2,
            Self::Pairing => 3,
            Self::Entries => 4,
            Self::LiveMeta => 5,
            Self::Lost => 6,
            Self::Asking => 7,
        }
    }
}

#[repr(u32)]
enum FaceTag {
    Opening = 0,
    Cli = 1,
    QuietHost = 2,
    Headless = 3,
    Stopping = 4,
    Gone = 5,
}

impl FaceTag {
    const fn code(self) -> u32 {
        match self {
            Self::Opening => 0,
            Self::Cli => 1,
            Self::QuietHost => 2,
            Self::Headless => 3,
            Self::Stopping => 4,
            Self::Gone => 5,
        }
    }
}

#[repr(C)]
struct FaceBytes {
    tag: u32,
    mode: u32,
    sinks: u32,
    handoff: u32,
}

struct Bridge {
    terminal: Terminal,
    inside: Inside,
    session: Mutex<Option<Session>>,
    queue: Mutex<VecDeque<Queued>>,
    watching: Mutex<tokio::sync::broadcast::Receiver<wire::Committed>>,
    face: Mutex<tokio::sync::watch::Receiver<Face>>,
    shown: Mutex<Face>,
    notice_tx: std::sync::mpsc::SyncSender<String>,
}

struct Queued {
    kind: HostEventKind,
    bytes: Vec<u8>,
}

pub(super) fn start(
    terminal: Terminal,
    inside: Inside,
    watching: tokio::sync::broadcast::Receiver<wire::Committed>,
    face: tokio::sync::watch::Receiver<Face>,
) -> Started {
    let (to, from) = std::sync::mpsc::sync_channel::<String>(UI_DEPTH);
    let notice_tx = to.clone();
    let notice: Say = Arc::new(move |line: String| match to.try_send(line) {
        Ok(()) => {}
        Err(std::sync::mpsc::TrySendError::Full(_)) => {}
        Err(std::sync::mpsc::TrySendError::Disconnected(_)) => {}
    });
    let session = match Session::begin() {
        Ok(session) => Some(session),
        Err(err) => {
            notice(format!("  {err}"));
            notice(format!("  {}", err.recovery()));
            None
        }
    };
    let bridge = Arc::new(Bridge {
        terminal,
        inside,
        session: Mutex::new(session),
        queue: Mutex::new(VecDeque::new()),
        watching: Mutex::new(watching),
        face: Mutex::new(face),
        shown: Mutex::new(Face::Opening),
        notice_tx,
    });
    let notices = Arc::clone(&bridge);
    std::thread::spawn(move || {
        for line in from {
            notices.push(HostEventKind::Notice, line.into_bytes());
        }
    });
    let writer_bridge = Arc::clone(&bridge);
    let writer = std::thread::spawn(move || writer_bridge.run_zig());
    Started {
        writer: Some(writer),
        notice,
    }
}

impl Bridge {
    fn push(&self, kind: HostEventKind, bytes: Vec<u8>) {
        if let Ok(mut q) = self.queue.lock() {
            if q.len() >= UI_DEPTH {
                q.pop_front();
            }
            q.push_back(Queued { kind, bytes });
        }
    }

    fn run_zig(self: &Arc<Self>) {
        let url = self.terminal.url.clone();
        let city = self.terminal.city.clone();
        let token = self.terminal.token.clone().unwrap_or_default();
        let surface = match self.terminal.surface {
            Surface::QuietHost => 1u32,
            Surface::Cli | Surface::Headless => 0u32,
        };
        let config = CityConfig {
            url_ptr: url.as_ptr(),
            url_len: url.len(),
            city_ptr: city.as_ptr(),
            city_len: city.len(),
            token_ptr: if token.is_empty() {
                std::ptr::null()
            } else {
                token.as_ptr()
            },
            token_len: token.len(),
            surface,
        };
        let code: i32 = run_city_ui(&config, self.as_ref());
        if code != 0 {}
    }

    fn pump(&self) {
        if let Ok(mut face) = self.face.lock() {
            let now = *face.borrow_and_update();
            let mut shown = self
                .shown
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if now != *shown {
                *shown = now;
                self.push(HostEventKind::Face, face_bytes(now));
            }
        }
        loop {
            let committed = {
                let Ok(mut watching) = self.watching.lock() else {
                    break;
                };
                match watching.try_recv() {
                    Ok(c) => c,
                    Err(tokio::sync::broadcast::error::TryRecvError::Lagged(n)) => {
                        drop(watching);
                        self.push(
                            HostEventKind::Notice,
                            format!("  missed {n} ledger commit(s)").into_bytes(),
                        );
                        continue;
                    }
                    Err(_) => break,
                }
            };
            let entries = {
                let Ok(mut session) = self.session.lock() else {
                    break;
                };
                match session.as_mut() {
                    Some(session) => {
                        let room = session.room.clone();
                        session
                            .seen
                            .read(committed.record(), &room, local_time::local)
                    }
                    None => Vec::new(),
                }
            };
            if !entries.is_empty() {
                self.push(
                    HostEventKind::Entries,
                    Scene::entry_list(&entries).bytes().to_vec(),
                );
            }
            self.push_live_meta();
        }
    }

    fn push_live_meta(&self) {
        let Ok(session) = self.session.lock() else {
            return;
        };
        let Some(session) = session.as_ref() else {
            return;
        };
        let room = session.room.as_str();
        let offer = session.offer(&self.inside);
        let waiting = session
            .seen
            .waiting
            .first()
            .map(|w| w.what.as_str())
            .unwrap_or("");
        let waiting_more = session.seen.waiting.len().saturating_sub(1);
        let working_since = session
            .seen
            .since()
            .and_then(local_time::local)
            .map(|t| u64::from(t.seconds()))
            .unwrap_or(u64::MAX);
        let (call_name, call_subject) = session
            .seen
            .calls
            .last()
            .map(|c| (c.name.as_str(), c.subject.as_str()))
            .unwrap_or(("", ""));
        let calling_count: u32 = u32::from(!call_name.is_empty());

        let mut bytes = Vec::new();
        bytes.extend_from_slice(
            &u32::try_from(waiting_more)
                .unwrap_or(u32::MAX)
                .to_le_bytes(),
        );
        bytes.extend_from_slice(&working_since.to_le_bytes());
        bytes.extend_from_slice(&calling_count.to_le_bytes());
        bytes.extend_from_slice(&u32::try_from(room.len()).unwrap_or(u32::MAX).to_le_bytes());
        bytes.extend_from_slice(&u32::try_from(offer.len()).unwrap_or(u32::MAX).to_le_bytes());
        bytes.extend_from_slice(
            &u32::try_from(waiting.len())
                .unwrap_or(u32::MAX)
                .to_le_bytes(),
        );
        bytes.extend_from_slice(room.as_bytes());
        bytes.extend_from_slice(offer.as_bytes());
        bytes.extend_from_slice(waiting.as_bytes());
        if calling_count > 0 {
            bytes.extend_from_slice(
                &u32::try_from(call_name.len())
                    .unwrap_or(u32::MAX)
                    .to_le_bytes(),
            );
            bytes.extend_from_slice(call_name.as_bytes());
            bytes.extend_from_slice(
                &u32::try_from(call_subject.len())
                    .unwrap_or(u32::MAX)
                    .to_le_bytes(),
            );
            bytes.extend_from_slice(call_subject.as_bytes());
        }
        self.push(HostEventKind::LiveMeta, bytes);
    }
}

impl CityHost for Bridge {
    fn poll(&self, into: &mut [u8]) -> (u32, usize) {
        self.pump();
        let Some(Queued { kind, bytes }) = self.queue.lock().ok().and_then(|mut q| q.pop_front())
        else {
            return (HostEventKind::Empty.code(), 0);
        };
        let Some(slot) = into.get_mut(..bytes.len()) else {
            return (HostEventKind::Empty.code(), 0);
        };
        slot.copy_from_slice(&bytes);
        (kind.code(), bytes.len())
    }

    fn tell(&self, event: u32, arg: u32) {
        let mapped = match event {
            0 => Event::Web,
            1 => Event::Back,
            2 => Event::Quit(Asker::Console, CloseMode::Drain),
            3 => Event::Quit(Asker::Console, CloseMode::Interrupt),
            4 => Event::TerminalLost,
            5 => {
                let mut reply = arg.to_le_bytes().to_vec();
                match self.inside.door.issue_display_code() {
                    Ok(code) => {
                        reply.push(0);
                        reply.extend_from_slice(code.as_bytes());
                    }
                    Err(err) => {
                        reply.push(1);
                        reply.extend_from_slice(
                            format!(
                                "Could not issue a pairing code\n```toml\ncode = {:?}\naction = {:?}\nsubject = {:?}\nrecovery = {:?}\n```",
                                err.code().as_str(),
                                err.action(),
                                err.subject(),
                                err.recovery(),
                            )
                            .as_bytes(),
                        );
                    }
                }
                self.push(HostEventKind::Pairing, reply);
                return;
            }
            _ => return,
        };
        self.inside.tell(mapped);
    }

    fn carry_line(&self, line: &str) {
        let tx = self.notice_tx.clone();
        let say: Say = Arc::new(move |text: String| match tx.try_send(text) {
            Ok(()) => {}
            Err(std::sync::mpsc::TrySendError::Full(_)) => {}
            Err(std::sync::mpsc::TrySendError::Disconnected(_)) => {}
        });
        let next = {
            let Ok(mut session) = self.session.lock() else {
                return;
            };
            let Some(session) = session.as_mut() else {
                return;
            };
            session.carry(&self.terminal, &self.inside, line, &say)
        };
        match next {
            Next::Stay => self.push_live_meta(),
            Next::Web => self.inside.tell(Event::Web),
            Next::Quit => self
                .inside
                .tell(Event::Quit(Asker::Console, CloseMode::Drain)),
            Next::AskBeforeQuit(going) => {
                say(format!(
                    "  {going} run(s) still working; Enter drains, n interrupts, Esc keeps serving"
                ));
                self.push(HostEventKind::Asking, going.to_le_bytes().to_vec());
            }
        }
    }

    fn open_paired(&self, url: &str) -> i32 {
        match (self.inside.open_paired)(url, &self.inside.door) {
            Ok(()) => 0,
            Err(err) => {
                self.push(
                    HostEventKind::Notice,
                    format!("  {}: {}", err.action(), err.recovery()).into_bytes(),
                );
                1
            }
        }
    }

    fn carry_remote(&self, tail: &str, into: &mut [u8]) -> Result<usize, i32> {
        let text = (self.inside.carry_remote)(tail);
        let Some(slot) = into.get_mut(..text.len()) else {
            return Err(2);
        };
        slot.copy_from_slice(text.as_bytes());
        Ok(text.len())
    }

    fn clock_ms(&self) -> Result<u64, i32> {
        match (self.inside.clock)() {
            Ok(t) => Ok(t.value()),
            Err(_) => Err(1),
        }
    }
}

fn face_bytes(face: Face) -> Vec<u8> {
    let packed = match face {
        Face::Opening => FaceBytes {
            tag: FaceTag::Opening.code(),
            mode: 0,
            sinks: 0,
            handoff: 0,
        },
        Face::Cli => FaceBytes {
            tag: FaceTag::Cli.code(),
            mode: 0,
            sinks: 0,
            handoff: 0,
        },
        Face::QuietHost => FaceBytes {
            tag: FaceTag::QuietHost.code(),
            mode: 0,
            sinks: 0,
            handoff: 0,
        },
        Face::Headless => FaceBytes {
            tag: FaceTag::Headless.code(),
            mode: 0,
            sinks: 0,
            handoff: 0,
        },
        Face::Stopping { mode, sinks } => FaceBytes {
            tag: FaceTag::Stopping.code(),
            mode: match mode {
                CloseMode::Drain => 0,
                CloseMode::Interrupt => 1,
            },
            sinks: match sinks {
                super::lifecycle::Sinks::Held => 0,
                super::lifecycle::Sinks::Cut => 1,
            },
            handoff: 0,
        },
        Face::Gone(handoff) => FaceBytes {
            tag: FaceTag::Gone.code(),
            mode: 0,
            sinks: 0,
            handoff: match handoff {
                super::lifecycle::Handoff::Written => 0,
                super::lifecycle::Handoff::Skipped => 1,
            },
        },
    };
    let mut bytes = Vec::with_capacity(16);
    bytes.extend_from_slice(&packed.tag.to_le_bytes());
    bytes.extend_from_slice(&packed.mode.to_le_bytes());
    bytes.extend_from_slice(&packed.sinks.to_le_bytes());
    bytes.extend_from_slice(&packed.handoff.to_le_bytes());
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::Entry;

    #[test]
    fn entry_list_encodes_a_note() {
        let scene = Scene::entry_list(&[Entry::Note {
            said: "host note".to_owned(),
        }]);
        let bytes = scene.bytes();
        assert!(bytes.len() > 4);
        let count = bytes
            .get(..4)
            .and_then(|prefix| <[u8; 4]>::try_from(prefix).ok())
            .map(u32::from_le_bytes);
        assert_eq!(count, Some(1));
    }

    #[test]
    fn face_bytes_match_c_layout_size() {
        assert_eq!(std::mem::size_of::<FaceBytes>(), 16);
        let bytes = face_bytes(Face::QuietHost);
        assert_eq!(bytes.len(), 16);
        assert_eq!(bytes.first().copied(), Some(2));
    }
}
