// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The browser half of the link: one socket, three listeners, and no
// judgements about the wire. Every judgement is `link.ts`'s; this turns
// browser callbacks into link events, carries out the actions the machine
// answers with, and hands what arrives to the belief and the asking.
//
// The one conversation the link holds of its own, the walk over a gap,
// is `gap_walk.ts`'s; this hands it the frames and answers it names.
//
// The browser's own facilities are reached here and nowhere below:
// the socket, the frame callback, the timers, and the clock the
// asking measures its patience by. A module that read the clock
// itself would be a module a test cannot put in a hurry.
//
// Frames are folded once per animation frame rather than as they land:
// a burst of records becomes one store update and one paint, which is
// what keeps a busy city under the 16 ms the person asked for.

import { writable } from "svelte/store";
import type { Readable } from "svelte/store";

import type { Lang } from "./lang";
import { createAsking } from "./asking";
import type { Asking } from "./asking";
import { createBelief } from "./belief";
import { createWatching } from "./watching";
import type { Watching } from "./watching";
import type { Belief } from "./belief";
import { NO_TAIL, appended } from "./live_output";
import type { Tail } from "./live_output";
import { decodeFrame, encodeFrame } from "./frames";
import { createUnsent, isSpeech } from "./unsent";
import { langOf, say } from "./lang";
import { createGapWalk } from "./gap_walk";
import { advance, connect as start, isLive, isRefused, newLink, unreadableRecord, unsentCommand } from "./link";
import type { Link, LinkAction, LinkEvent, LinkState } from "./link";
import { AskId } from "../wire";
import type { Command, Query, RunId, ServerFrame } from "../wire";

export interface Connection {
  readonly state: Readable<LinkState>;
  readonly belief: Readable<Belief>;
  readonly asking: Asking;
  // Each run's running command, as far as it has written, until the
  // call's result lands (core/live_output.ts).
  readonly live: Readable<Readonly<Partial<Record<RunId, Tail>>>>;
  // Words said while the link is down, waiting for the next welcome.
  readonly unsent: Readable<number>;
  // Sends one command, or holds it in `unsent` when it is words and the
  // link is down; false when neither happened, and the person should be
  // told rather than left to wonder.
  readonly command: (command: Command) => boolean;
  readonly retry: () => void;
  readonly dismissRefusal: () => void;
  // Everything in the bell has now been looked at.
  readonly markNoticesSeen: () => void;
  readonly monitor: Pick<Watching, "samples" | "watch" | "watchSummary">;
}

// The pairing code the host put on the URL that opened this page. An
// empty value is not a value.
export function tokenIn(search: string): string | null {
  const value = new URLSearchParams(search).get("token");
  return value === null || value === "" ? null : value;
}

// How a POST offers this city's pairing token.
//
// The socket offers it inside the hello frame, where the frame type
// names the field; a POST has no frame, so it carries the standard
// bearer header, which `wire::reception::offered_pairing` reads.
// A page opened without a code sends no header at all: a city with no
// token configured is a city on loopback, and it admits every door.
export function bearing(token: string | null): Readonly<Record<string, string>> {
  return token === null ? {} : { authorization: `Bearer ${token}` };
}

function hidden(): boolean {
  return typeof document !== "undefined" && document.visibilityState === "hidden";
}

// The address of this city's socket, derived from the page's own origin:
// a client served by the city it talks to needs no configured endpoint.
export function socketUrl(location: Location): string {
  const scheme = location.protocol === "https:" ? "wss" : "ws";
  return `${scheme}://${location.host}/ws`;
}

export function openConnection(
  url: string,
  token: string | null,
  lang: Lang,
): Connection {
  let link: Link = newLink(token, lang);
  let socket: WebSocket | null = null;
  // One clock for this connection: the asking measures its patience by
  // it and the belief stamps each refusal with it, so a question and its
  // refusal can never disagree about when they happened.
  const now = () => Date.now();
  const state = writable<LinkState>(link.state);
  const store = createBelief(now);
  const unsent = createUnsent();
  const live = writable<Readonly<Partial<Record<RunId, Tail>>>>({});

  const queue: ServerFrame[] = [];
  let scheduled = false;
  // The last id this connection's page minted. One counter for every
  // question, so the gap walk and the views never share an id.
  let lastAsk = 0;
  // The attempt the ladder scheduled, held so a link that turns out to
  // be refused can cancel it. A refused link that left this running
  // would reopen the socket behind a message telling the person the
  // opposite.
  let reconnect: ReturnType<typeof setTimeout> | null = null;

  // Sends one question under a fresh id; null when the socket is not open.
  function sendAsk(query: Query): AskId | null {
    lastAsk = lastAsk >= 0xffff_ffff ? 1 : lastAsk + 1;
    const askId = AskId.make(lastAsk);
    return sendText(encodeFrame({ ask: { ask_id: askId, query } })) ? askId : null;
  }

  function sendText(text: string): boolean {
    if (socket?.readyState !== WebSocket.OPEN) {
      return false;
    }
    socket.send(text);
    return true;
  }
  const watching = createWatching(sendText);

  const asking = createAsking(
    (query: Query) => (isLive(link) ? sendAsk(query) : null),
    now,
    // A question that never came back lands where every other refusal
    // lands: the corner once, and the bell until the person has read
    // it. The recovery is the one part written for a person, so it is
    // said here, in the language `<html lang>` states - which `app.tsx`
    // keeps equal to the person's choice, and which a screen reader
    // reads the page by.
    (phrase, error) => {
      const lang = langOf(document.documentElement.lang);
      store.refused({ ...error, recovery: say(lang, phrase) });
    },
  );
  const walk = createGapWalk(sendAsk, store, asking, lang);

  function perform(action: LinkAction): void {
    switch (action.kind) {
      case "nothing":
        return;
      case "open":
        open();
        return;
      case "send":
        sendText(encodeFrame(action.frame));
        return;
      case "welcomed":
        store.refused(null);
        store.named(action.welcome.city ?? null);
        walk.welcomed(action.welcome.epoch ?? null, action.welcome.resume_from ?? null);
        unsent.release((command) => sendText(encodeFrame({ command })));
        watching.reconnected();
        return;
      case "deliver": {
        // The field this build could not read, if any, is reported here
        // rather than swallowed: the frame decoded and the socket is
        // still speaking this wire, so it goes where every other refusal
        // goes, and the rest of the record has already been folded.
        const bad = walk.fold(action.event);
        if (action.event.kind === "tool_result") {
          live.update(({ [action.event.run]: _settled, ...rest }) => rest);
        }
        asking.invalidate(action.event);
        if (bad !== null) store.refused(unreadableRecord(lang, bad));
        return;
      }
      case "answered": {
        const { ask_id, as_of, outcome } = action.answered;
        if ("refusal" in outcome) store.refused(outcome.refusal);
        // The walk's own question settles no held question, so its
        // answer never reaches the asking.
        if (walk.answered(ask_id, outcome)) return;
        if ("answer" in outcome && "city" in outcome.answer) {
          store.adoptCity(outcome.answer.city);
        }
        asking.answered(ask_id, as_of, outcome);
        return;
      }
      case "saying":
        store.say(action.delta);
        return;
      case "logged":
        store.logged(action.line);
        return;
      case "writing": {
        const { piece } = action;
        live.update((tails) => ({ ...tails, [piece.run]: appended(tails[piece.run] ?? NO_TAIL, piece) }));
        return;
      }
      case "lagged":
        walk.lagged(action.from, action.to);
        return;
      case "sampled":
        watching.sampled(action.sample);
        return;
      case "wait":
        reconnect = setTimeout(() => {
          reconnect = null;
          step({ kind: "wait_elapsed" });
        }, action.ms);
        return;
      case "report":
        store.refused(action.error);
        return;
      case "close":
        socket?.close();
        return;
    }
  }

  function step(event: LinkEvent): void {
    const was = link;
    const [next, action] = advance(link, event);
    link = next;
    if (next.state !== was.state) {
      state.set(next.state);
    }
    perform(action);
    if (isRefused(link)) {
      // The machine has stopped this link. Nothing here decides that:
      // the socket half only carries it out, by cancelling the attempt
      // the ladder booked and dropping the socket the refusal came in
      // on. The person's retry is what starts it again.
      if (reconnect !== null) {
        clearTimeout(reconnect);
        reconnect = null;
      }
      const open = socket;
      socket = null;
      open?.close();
    }
  }

  // Drains what arrived since the last paint, in order, as one update.
  function drain(): void {
    scheduled = false;
    store.batch(() => {
      for (const frame of queue.splice(0, queue.length)) {
        step({ kind: "received", frame });
      }
    });
  }

  function open(): void {
    const opened = new WebSocket(url);
    socket = opened;
    opened.onopen = () => {
      if (socket === opened) step({ kind: "opened" });
    };
    opened.onmessage = (message: MessageEvent) => {
      if (socket !== opened || typeof message.data !== "string") {
        return;
      }
      const frame = decodeFrame(message.data);
      if (frame === null) {
        // The two ends disagree about the wire. Reported as what it is,
        // never as a close: a page told the socket dropped reconnects,
        // and it would meet this same frame every time.
        step({ kind: "undecodable" });
        return;
      }
      // A welcome is folded at once, so the questions the page has been
      // holding go out on the same tick the link comes up.
      if ("welcome" in frame || "refusal" in frame) {
        store.batch(() => {
          drain();
          step({ kind: "received", frame });
        });
        return;
      }
      queue.push(frame);
      if (!scheduled) {
        scheduled = true;
        // A hidden tab is never painted, so it drains on a timer instead;
        // otherwise an approval request waits for the person to come back.
        if (hidden()) setTimeout(drain, 0);
        else requestAnimationFrame(drain);
      }
    };
    const closed = () => {
      if (socket !== opened) return;
      socket = null;
      step({ kind: "closed" });
    };
    opened.onclose = closed;
    opened.onerror = closed;
  }

  // Either way the queue drains now: going hidden, the paint it waits
  // for will not come, and shown again, the person is looking. Shown, the
  // ladder's wait is spent too - `wait_elapsed` is the machine's own door
  // back to `opening`, and it opens only from backoff.
  function visibilityChanged(): void {
    if (queue.length > 0) drain();
    if (hidden() || reconnect === null) return;
    step(unbook({ kind: "wait_elapsed" }));
  }

  // Cancels the attempt the ladder booked, for an event that opens the
  // link now instead.
  function unbook(event: LinkEvent): LinkEvent {
    if (reconnect !== null) clearTimeout(reconnect);
    reconnect = null;
    return event;
  }
  if (typeof document !== "undefined") document.addEventListener("visibilitychange", visibilityChanged);

  const [begun, first] = start(link);
  link = begun;
  state.set(begun.state);
  perform(first);

  return {
    state,
    belief: store.belief,
    asking,
    unsent: unsent.count,
    live,
    command(command) {
      switch (link.state.kind) {
        case "live":
          return sendText(encodeFrame({ command }));
        // Words are held only while the link is on its way back by
        // itself (client/Spec.lean, socket.ts); a refused link waits for the
        // person, and an idle one has not been started.
        case "idle":
        case "refused":
          return false;
        case "opening":
        case "handshaking":
        case "backoff":
          if (!isSpeech(command)) {
            store.refused(unsentCommand(lang, Object.keys(command).join()));
            return false;
          }
          unsent.hold(command);
          return true;
      }
    },
    retry() {
      step(unbook({ kind: "retry" }));
    },
    dismissRefusal() {
      store.refused(null);
    },
    markNoticesSeen() {
      store.noticesSeen();
    },
    monitor: watching,
  };
}
