// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

use kernel::RunId;
use runtime::Interrupt;

use super::relay::Wake;
use crate::worker::Closing;

#[derive(Default)]
pub struct CommandDesk {
    waiting: std::sync::Mutex<Waiting>,

    bell: std::sync::Mutex<Option<std::sync::mpsc::Sender<Wake>>>,

    closing: std::sync::OnceLock<Closing>,

    interrupting: std::sync::atomic::AtomicBool,
}

#[derive(Default)]
struct Waiting {
    queue: std::collections::VecDeque<Posted>,

    keys: std::collections::BTreeSet<kernel::IdemKey>,
    steered: std::collections::BTreeMap<kernel::IdemKey, SteerReceipt>,
    image_receivers: std::collections::BTreeMap<RunId, kernel::Address>,
}

enum SteerReceipt {
    Waiting { run: RunId, reply: wire::Reply },
    Accepted,
    Refused(kernel::AxError),
}

pub struct Underway<'desk> {
    desk: &'desk CommandDesk,
    key: Option<kernel::IdemKey>,
}

impl Drop for Underway<'_> {
    fn drop(&mut self) {
        let Some(key) = self.key else { return };
        if let Ok(mut waiting) = self.desk.waiting.lock() {
            waiting.keys.remove(&key);
        }
    }
}

impl Waiting {
    fn forget(&mut self, at: usize) -> Option<Posted> {
        let posted = self.queue.remove(at)?;
        if let Some(key) = posted.command.idem() {
            self.keys.remove(key);
        }
        Some(posted)
    }
}

pub struct Posted {
    pub command: wire::Command,
    pub reply: wire::Reply,
}

pub enum DeskWait<'desk> {
    Command(Box<Posted>, Underway<'desk>),
    Idle,

    Close(&'desk Closing),
    Gone,
}

pub(super) const SCHEDULE_TICK_MS: u64 = 20_000;

impl CommandDesk {
    pub fn close(&self, why: Closing) {
        if let Closing::Chosen {
            mode: wire::CloseMode::Interrupt,
            ..
        } = &why
        {
            self.interrupt();
        }
        self.closing.get_or_init(|| why);
        self.ring(Wake::Close);
    }

    pub fn interrupt(&self) {
        self.interrupting
            .store(true, std::sync::atomic::Ordering::SeqCst);
        self.ring(Wake::Close);
    }

    pub fn interrupting(&self) -> bool {
        self.interrupting.load(std::sync::atomic::Ordering::SeqCst)
    }

    pub(crate) fn ring_through(&self, bell: std::sync::mpsc::Sender<Wake>) {
        if let Ok(mut ringing) = self.bell.lock() {
            *ringing = Some(bell);
        }
    }

    fn ring(&self, wake: Wake) {
        if let Ok(ringing) = self.bell.lock()
            && let Some(bell) = ringing.as_ref()
        {
            drop(bell.send(wake));
        }
    }

    pub fn post(&self, command: wire::Command, reply: wire::Reply) {
        if let Ok(mut waiting) = self.waiting.lock() {
            if let wire::Command::Steer { idem, .. } = &command {
                match waiting.steered.get_mut(idem) {
                    Some(SteerReceipt::Waiting {
                        reply: previous, ..
                    }) => {
                        *previous = reply;
                        return;
                    }
                    Some(SteerReceipt::Accepted) => {
                        drop(waiting);
                        let _delivered = reply.accept(*idem);
                        return;
                    }
                    Some(SteerReceipt::Refused(error)) => {
                        let error = error.clone();
                        drop(waiting);
                        let _delivered = reply.refuse(error);
                        return;
                    }
                    None => {}
                }
            }
            if let Some(key) = command.idem()
                && kernel::idem::claim(&mut waiting.keys, *key).is_err()
            {
                return;
            }
            waiting.queue.push_back(Posted { command, reply });
        }
        self.ring(Wake::Command);
    }

    pub fn next(&self, driving: impl Fn(RunId) -> bool) -> DeskWait<'_> {
        let Ok(mut waiting) = self.waiting.lock() else {
            return DeskWait::Gone;
        };
        let for_a_lane = |posted: &Posted| match wire::classify(&posted.command) {
            wire::ControlVerdict::Intervene { run: Some(run), .. } => driving(run),
            wire::ControlVerdict::Intervene { run: None, .. }
            | wire::ControlVerdict::NotAnIntervention
            | wire::ControlVerdict::Refuse(_) => false,
        };
        let first = waiting.queue.iter().position(|posted| !for_a_lane(posted));
        match first.and_then(|at| waiting.queue.remove(at)) {
            Some(posted) => self.carrying(posted),
            None => match self.closing.get() {
                Some(why) => DeskWait::Close(why),
                None => DeskWait::Idle,
            },
        }
    }

    fn carrying(&self, posted: Posted) -> DeskWait<'_> {
        let key = posted.command.idem().copied();
        DeskWait::Command(Box::new(posted), Underway { desk: self, key })
    }

    #[cfg(test)]
    pub fn take(&self) -> Option<wire::Command> {
        let mut waiting = self.waiting.lock().ok()?;
        let posted = waiting.queue.pop_front()?;
        if let Some(key) = posted.command.idem() {
            waiting.keys.remove(key);
        }
        Some(posted.command)
    }

    pub fn interrupt_for(
        &self,
        run: RunId,
        images_store: Option<&std::sync::Mutex<storage::Cas>>,
    ) -> Interrupt {
        if self.interrupting() {
            return Interrupt::Cancel;
        }
        let Ok(mut waiting) = self.waiting.lock() else {
            return Interrupt::None;
        };
        let cancel = waiting.queue.iter().position(
            |posted| matches!(&posted.command, wire::Command::Cancel { run: r, .. } if *r == run),
        );
        if let Some(at) = cancel {
            waiting.forget(at);
            return Interrupt::Cancel;
        }
        let steer = waiting.queue.iter().position(
            |posted| matches!(&posted.command, wire::Command::Steer { run: r, .. } if *r == run),
        );
        let Some(at) = steer else {
            return Interrupt::None;
        };
        let Some(Posted {
            command: wire::Command::Steer {
                text, images, idem, ..
            },
            reply,
        }) = waiting.forget(at)
        else {
            return Interrupt::None;
        };

        match collab::Steer::from_person(&text) {
            Ok(steer) => {
                let image_room = waiting.image_receivers.get(&run).cloned();
                waiting
                    .steered
                    .insert(idem, SteerReceipt::Waiting { run, reply });
                drop(waiting);
                let pictures = match place_pictures(&images, run, image_room, images_store) {
                    Ok(pictures) => pictures,
                    Err(error) => {
                        self.reject_steer(idem, error);
                        return Interrupt::None;
                    }
                };
                Interrupt::Steer {
                    speaker: runtime::conversation::Speaker::Person,
                    text: steer.text().to_owned(),
                    request: Some(idem),
                    pictures,
                }
            }
            Err(error) => {
                drop(waiting);
                let _delivered = reply.refuse(error);
                Interrupt::None
            }
        }
    }

    pub fn observe(&self, record: &kernel::EventRecord) {
        if record.kind() == kernel::EventKind::RunStarted {
            if record
                .data()
                .as_map()
                .get("image_input")
                .and_then(serde_json::Value::as_bool)
                == Some(true)
                && let Some(room) = record.addr()
                && let Ok(mut waiting) = self.waiting.lock()
            {
                waiting.image_receivers.insert(record.run(), room.clone());
            }
        } else if record.kind() == kernel::EventKind::SteerReceived {
            let Ok(steer) = record.data().read::<kernel::event::record::SteerReceived>() else {
                return;
            };
            let Some(key) = steer.idem else { return };
            if let Ok(mut waiting) = self.waiting.lock()
                && let Some(SteerReceipt::Waiting { reply, .. }) =
                    waiting.steered.insert(key, SteerReceipt::Accepted)
            {
                drop(waiting);
                let _delivered = reply.accept(key);
            }
        } else if record.kind() == kernel::EventKind::RunFrozen {
            self.end_steers(record.run());
        }
    }

    fn reject_steer(&self, key: kernel::IdemKey, error: kernel::AxError) {
        if let Ok(mut waiting) = self.waiting.lock()
            && let Some(SteerReceipt::Waiting { reply, .. }) = waiting
                .steered
                .insert(key, SteerReceipt::Refused(error.clone()))
        {
            drop(waiting);
            let _delivered = reply.refuse(error);
        }
    }

    fn end_steers(&self, ended: RunId) {
        let Ok(mut waiting) = self.waiting.lock() else {
            return;
        };
        waiting.image_receivers.remove(&ended);
        let error = kernel::AxError::failure(kernel::AxCode::InvalidArgs, "steer a run", ended.to_string())
            .with_recovery("the run ended before it accepted the steer; review the retained draft and send it as a follow-up");
        let mut replies = Vec::new();
        for receipt in waiting.steered.values_mut() {
            if let SteerReceipt::Waiting { run, .. } = receipt
                && *run == ended
                && let SteerReceipt::Waiting { reply, .. } =
                    std::mem::replace(receipt, SteerReceipt::Refused(error.clone()))
            {
                replies.push(reply);
            }
        }
        drop(waiting);
        for reply in replies {
            let _delivered = reply.refuse(error.clone());
        }
    }
}

fn place_pictures(
    encoded: &[String],
    run: RunId,
    room: Option<kernel::Address>,
    store: Option<&std::sync::Mutex<storage::Cas>>,
) -> Result<Vec<kernel::ImageRef>, kernel::AxError> {
    use super::dispatching::images::{no_picture_input, read_images, store_images};
    if encoded.is_empty() {
        return Ok(Vec::new());
    }
    let Some((building, store)) = room.zip(store) else {
        return Err(no_picture_input());
    };
    let mut pictures = read_images(encoded)?;
    let mut held = super::held(store, "store a selected image")?;
    store_images(
        &mut held,
        &storage::BlockOrigin { run, building },
        &mut pictures,
    )?;
    Ok(pictures
        .into_iter()
        .map(|picture| picture.reference)
        .collect())
}
