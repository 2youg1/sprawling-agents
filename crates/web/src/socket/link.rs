// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

//! The link: state, events, actions, and the backoff ladder.

#[cfg(target_arch = "wasm32")]
use super::frames::read_frame;

use channels::{
    Address, AxCode, AxError, EventRecord, Hello, Seq, ServerFrame, Welcome, schema_hash,
};

/// The backoff ladder in milliseconds. Ends flat rather than growing without
/// bound: a person who left the laptop closed should find the interface live
/// within a minute of opening it, not on the far side of an hour-long wait.
const BACKOFF_LADDER_MS: [u64; 6] = [250, 500, 1_000, 2_000, 5_000, 10_000];

/// Where the link is.
#[derive(Debug, Clone, PartialEq)]
pub enum LinkState {
    /// Nothing attempted yet.
    Idle,
    /// A socket is opening.
    Opening,
    /// Open, `Hello` sent, waiting for the server's answer.
    Handshaking,
    /// Serving frames.
    Live { resume_from: Option<Seq> },
    /// Waiting out a failed attempt. The attempt *number* is not here: it
    /// survives the trip through `Opening` and so belongs to the link, not
    /// to a momentary phase (see [`Link::consecutive_failures`]).
    Backoff,
    /// Stopped on purpose. Not a failure to retry - a fact to report.
    Refused(Box<AxError>),
    /// Nobody is looking at this tab. Not a failure and not a refusal:
    /// the link is closed because there is no reader, and it comes back
    /// when there is one. Carrying `resume_from` across is what makes
    /// coming back cheap - the server replays a tail rather than a
    /// history.
    Suspended { resume_from: Option<Seq> },
}

/// What happened to the link.
#[derive(Debug, Clone, PartialEq)]
pub enum LinkEvent {
    Opened,
    Received(Box<ServerFrame>),
    Closed,
    /// The transport failed to open or died mid-flight.
    TransportFailed,
    /// The backoff wait elapsed.
    WaitElapsed,
    /// The tab went out of view.
    Backgrounded,
    /// The tab came back.
    Foregrounded,
}

/// What the shell should do next. Exhaustive, so the shell cannot invent an
/// action the machine never authorised.
#[derive(Debug, Clone, PartialEq)]
pub enum LinkAction {
    Nothing,
    OpenSocket,
    Send(Box<Hello>),
    /// Hand an event to `app::Snapshot::apply`.
    Deliver(Box<EventRecord>),
    /// Hand a query answer to whichever view asked for it.
    Answered(Box<channels::Answer>),
    /// Hand text a model is still saying to the page showing that run.
    ///
    /// Never `Deliver`: an increment has no sequence number and is never
    /// written down, so folding it into the snapshot would put something
    /// on screen that no replay could produce.
    Saying(channels::Delta),
    /// Sleep this long, then feed back `WaitElapsed`.
    WaitMs(u64),
    /// Show this and stop. The interface renders the three-part refusal.
    Report(Box<AxError>),
    /// Close the socket and do nothing further until asked. Distinct
    /// from `WaitMs`: a slowed tab still holds a connection, still wakes
    /// the machine, and still spends something nobody agreed to spend.
    CloseSocket,
}

/// One connection, as a value.
#[derive(Debug, Clone, PartialEq)]
pub struct Link {
    state: LinkState,
    token: Option<String>,
    /// Failures since the last time frames flowed. Held on the link rather
    /// than inside `LinkState::Backoff`, because every retry passes through
    /// `Opening` on its way back to `Backoff` - a counter living in the
    /// phase is reset by its own retry loop and the ladder never climbs.
    consecutive_failures: u32,
    /// The city named in the welcome, once one has arrived.
    city: Option<Address>,
}

impl Link {
    /// A link that has not tried yet. `token` is present only when the
    /// operator paired this browser with a non-loopback host.
    #[must_use]
    pub fn new(token: Option<String>) -> Self {
        Self {
            state: LinkState::Idle,
            token,
            consecutive_failures: 0,
            city: None,
        }
    }

    /// Failures since frames last flowed. Zero whenever the link is live.
    #[must_use]
    pub fn consecutive_failures(&self) -> u32 {
        self.consecutive_failures
    }

    #[must_use]
    pub fn state(&self) -> &LinkState {
        &self.state
    }

    /// The city this link reached, once it has been welcomed.
    #[must_use]
    pub fn city(&self) -> Option<&Address> {
        self.city.as_ref()
    }

    /// Whether frames are flowing. The interface dims itself when not.
    #[must_use]
    pub fn is_live(&self) -> bool {
        matches!(self.state, LinkState::Live { .. })
    }

    /// Starts, or restarts after a `Refused` was cleared by the operator.
    pub fn connect(&mut self) -> LinkAction {
        self.state = LinkState::Opening;
        LinkAction::OpenSocket
    }

    /// Advances the machine.
    ///
    /// `resume_from` is carried across a reconnect so the server can replay
    /// the tail rather than the whole history; `app::Snapshot` drops any
    /// overlap for free, so the cut does not need to be exact.
    pub fn advance(&mut self, event: LinkEvent) -> LinkAction {
        match (&self.state, event) {
            (LinkState::Refused(_), _) => LinkAction::Nothing,

            (LinkState::Opening, LinkEvent::Opened) => {
                self.state = LinkState::Handshaking;
                LinkAction::Send(Box::new(Hello {
                    wire_v: channels::WIRE_V,
                    schema: schema_hash(),
                    token: self.token.clone(),
                }))
            }

            (LinkState::Handshaking, LinkEvent::Received(frame)) => match *frame {
                ServerFrame::Welcome(welcome) => self.welcomed(&welcome),
                ServerFrame::Refusal(err) => self.refuse(*err),
                // A server that streams or answers before welcoming is not
                // speaking this protocol; treat it as the mismatch it is.
                ServerFrame::Event(_) | ServerFrame::Answer(_) | ServerFrame::Delta(_) => {
                    self.refuse(out_of_order())
                }
            },

            (LinkState::Live { resume_from }, LinkEvent::Received(frame)) => {
                let resume_from = *resume_from;
                match *frame {
                    ServerFrame::Event(event) => {
                        self.state = LinkState::Live {
                            resume_from: Some(event.seq()),
                        };
                        LinkAction::Deliver(event)
                    }
                    ServerFrame::Answer(answer) => {
                        self.state = LinkState::Live { resume_from };
                        LinkAction::Answered(answer)
                    }
                    // Text arriving mid-call. It does not advance
                    // `resume_from`: nothing here is recoverable from the
                    // ledger, because nothing here is in it.
                    ServerFrame::Delta(delta) => LinkAction::Saying(delta),
                    ServerFrame::Refusal(err) => LinkAction::Report(err),
                    ServerFrame::Welcome(_) => {
                        self.state = LinkState::Live { resume_from };
                        LinkAction::Nothing
                    }
                }
            }

            (LinkState::Backoff, LinkEvent::WaitElapsed) => {
                self.state = LinkState::Opening;
                LinkAction::OpenSocket
            }

            // Going out of view stops the link from wherever it was. A
            // refusal is already caught by the first arm above: it is a
            // fact to report, and it does not become less true because
            // somebody switched tabs.
            (state, LinkEvent::Backgrounded) => {
                let resume_from = match state {
                    LinkState::Live { resume_from } => *resume_from,
                    _ => None,
                };
                self.state = LinkState::Suspended { resume_from };
                LinkAction::CloseSocket
            }
            (LinkState::Suspended { .. }, LinkEvent::Foregrounded) => {
                // Back from the bottom of the ladder: time away is not
                // evidence that the server is unwell.
                self.consecutive_failures = 0;
                self.state = LinkState::Opening;
                LinkAction::OpenSocket
            }
            // Nothing reaches a suspended link. A socket that was closing
            // while the tab went away will report it, and that report
            // must not restart anything.
            (LinkState::Suspended { .. }, _) => LinkAction::Nothing,

            (_, LinkEvent::Closed | LinkEvent::TransportFailed) => self.retreat(),

            // Anything else is a frame arriving in a state that did not ask
            // for one: ignore rather than crash, because the link's job is
            // to keep the interface honest, not to police the server.
            _ => LinkAction::Nothing,
        }
    }

    fn welcomed(&mut self, welcome: &Welcome) -> LinkAction {
        if welcome.wire_v != channels::WIRE_V || welcome.schema != schema_hash() {
            return self.refuse(mismatch(welcome));
        }
        // Which city answered. Kept here because the handshake is where it
        // was said, and the interface reads it from the link rather than
        // waiting for an event that will never come again.
        self.city.clone_from(&welcome.city);
        self.state = LinkState::Live {
            resume_from: welcome.resume_from,
        };
        // Frames flow again: the next outage starts at the bottom of the
        // ladder rather than inheriting an old grudge.
        self.consecutive_failures = 0;
        LinkAction::Nothing
    }

    fn refuse(&mut self, err: AxError) -> LinkAction {
        let boxed = Box::new(err);
        self.state = LinkState::Refused(boxed.clone());
        LinkAction::Report(boxed)
    }

    fn retreat(&mut self) -> LinkAction {
        let attempt = self.consecutive_failures;
        self.consecutive_failures = attempt.saturating_add(1);
        self.state = LinkState::Backoff;
        LinkAction::WaitMs(backoff_ms(attempt))
    }
}

/// The wait for one attempt number, clamped to the end of the ladder.
#[must_use]
pub fn backoff_ms(attempt: u32) -> u64 {
    let last = BACKOFF_LADDER_MS.len().saturating_sub(1);
    let index = usize::try_from(attempt).unwrap_or(last).min(last);
    BACKOFF_LADDER_MS.get(index).copied().unwrap_or(10_000)
}

fn mismatch(welcome: &Welcome) -> AxError {
    AxError::failure(
        AxCode::WireMismatch,
        "join this city's control surface",
        format!(
            "this page speaks wire v{} and the server speaks v{}",
            channels::WIRE_V,
            welcome.wire_v
        ),
    )
    .with_recovery("reload the page to fetch the client this server was built with")
}

fn out_of_order() -> AxError {
    AxError::failure(
        AxCode::WireMismatch,
        "join this city's control surface",
        "the server streamed events before completing the handshake",
    )
    .with_recovery("reload the page; if it repeats, the address is not a sprawling server")
}

/// The browser half: one socket, three listeners, and no decisions.
///
/// Every judgement belongs to [`Link`]. This opens the connection the
/// machine asked for and turns browser callbacks into [`LinkEvent`]s.
/// Reconnect timing stays with the caller, because a timer belongs to
/// whoever owns the frame loop rather than to a transport.
///
/// # Errors
/// Refuses an address the browser will not open, naming it.
#[cfg(target_arch = "wasm32")]
pub fn open(
    url: &str,
    on_event: impl FnMut(LinkEvent) + 'static,
) -> Result<web_sys::WebSocket, AxError> {
    use std::cell::RefCell;
    use std::rc::Rc;
    use wasm_bindgen::JsCast;
    use wasm_bindgen::closure::Closure;

    let socket = web_sys::WebSocket::new(url).map_err(|_| {
        AxError::failure(
            AxCode::WireMismatch,
            "open the control surface socket",
            url.to_owned(),
        )
        .with_recovery("reload the page; this client is served by the city it talks to")
    })?;

    type Sink = Rc<RefCell<Box<dyn FnMut(LinkEvent)>>>;
    let sink: Sink = Rc::new(RefCell::new(Box::new(on_event)));
    let deliver = move |sink: &Sink, event: LinkEvent| {
        // A listener that fires while the machine is mid-step drops its
        // event rather than reentering it: the socket will report the
        // same condition again, and a half-applied transition would not.
        if let Ok(mut hold) = sink.try_borrow_mut() {
            hold(event);
        }
    };

    let opened = Closure::<dyn FnMut()>::new({
        let sink = Rc::clone(&sink);
        move || deliver(&sink, LinkEvent::Opened)
    });
    socket.set_onopen(Some(opened.as_ref().unchecked_ref()));
    opened.forget();

    let message = Closure::<dyn FnMut(web_sys::MessageEvent)>::new({
        let sink = Rc::clone(&sink);
        move |event: web_sys::MessageEvent| {
            if let Some(text) = event.data().as_string() {
                deliver(&sink, read_frame(&text));
            }
        }
    });
    socket.set_onmessage(Some(message.as_ref().unchecked_ref()));
    message.forget();

    let closed = Closure::<dyn FnMut(web_sys::CloseEvent)>::new({
        let sink = Rc::clone(&sink);
        move |_event: web_sys::CloseEvent| deliver(&sink, LinkEvent::Closed)
    });
    socket.set_onclose(Some(closed.as_ref().unchecked_ref()));
    closed.forget();

    let errored = Closure::<dyn FnMut(web_sys::Event)>::new({
        let sink = Rc::clone(&sink);
        move |_event: web_sys::Event| deliver(&sink, LinkEvent::Closed)
    });
    socket.set_onerror(Some(errored.as_ref().unchecked_ref()));
    errored.forget();

    Ok(socket)
}

/// Sends one client frame. Serialization failure is impossible for the
/// frames this crate builds, so it reports the send failure only.
///
/// # Errors
/// Names the frame the socket refused to carry.
#[cfg(target_arch = "wasm32")]
pub fn send(socket: &web_sys::WebSocket, frame: &channels::ClientFrame) -> Result<(), AxError> {
    let text = serde_json::to_string(frame).map_err(|err| {
        AxError::failure(
            AxCode::WireMismatch,
            "encode a client frame",
            err.to_string(),
        )
    })?;
    socket.send_with_str(&text).map_err(|_| {
        AxError::failure(
            AxCode::WireMismatch,
            "send a client frame",
            "the socket is not open",
        )
        .with_recovery("wait for the link to come back; it retries on a fixed ladder")
    })
}
