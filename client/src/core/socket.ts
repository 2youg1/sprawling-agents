// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The browser half of the link: one socket, three listeners, and no
// judgements about the wire. Every judgement is `link.ts`'s; this turns
// browser callbacks into link events, carries out the actions the machine
// answers with, and hands what arrives to the belief and the asking.
//
// The one conversation it holds of its own is the walk over a gap: a
// `lagged` frame names a range of ledger records this page never received,
// and the range is asked for a page at a time and folded like any other
// record. That is a sequence of questions and answers rather than one
// frame in and one action out, which is why it lives here and not in the
// tree of judgements `link.ts` is.
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
import type { Belief } from "./belief";
import { decodeFrame, encodeFrame } from "./frames";
import { langOf, say } from "./lang";
import { advance, connect as start, isLive, isRefused, newLink, unreadableRecord } from "./link";
import type { Link, LinkAction, LinkEvent, LinkState } from "./link";
import { Seq } from "../wire";
import type { Command, EventRecord, HistoryRangeAnswer, Query, ServerFrame } from "../wire";

export interface Connection {
  readonly state: Readable<LinkState>;
  readonly belief: Readable<Belief>;
  readonly asking: Asking;
  // Sends one command; false when the link is not live, in which case
  // nothing was sent and the person should be told rather than left to
  // wonder.
  readonly command: (command: Command) => boolean;
  readonly retry: () => void;
  readonly dismissRefusal: () => void;
  // Everything in the bell has now been looked at.
  readonly markNoticesSeen: () => void;
}

// The pairing code the host put on the URL that opened this page. An
// empty value is not a value.
export function tokenIn(search: string): string | null {
  const value = new URLSearchParams(search).get("token");
  return value === null || value === "" ? null : value;
}

// A range of ledger records this page was told it never received.
interface Gap {
  // The first sequence still to fetch.
  readonly at: Seq;
  // The last sequence the range reaches.
  readonly to: Seq;
  // What the records of the range do to the answers on the page once
  // folded: see `filled` and `resume`.
  readonly records: GapRecords;
}

// A range `lagged` named holds records the live stream skipped, which
// invalidate nothing; a range written while the socket was down holds
// the news the answers on the page missed, and each record invalidates
// what it touches.
type GapRecords = "folded" | "invalidating";

// One page of a gap. The server caps an answer at its own limit whatever
// this asks for, and a page small enough to fold in one frame keeps a
// long gap from holding up the records that are still arriving.
const GAP_PAGE = 200;

// How many gap pages a reconnect fetches before a snapshot is the
// cheaper way to the present: past this, asking every watched question
// again moves fewer bytes than walking the range (client-SPEC 4-39).
const RESUME_PAGES = 2;

// How a POST offers this city's pairing token.
//
// The socket offers it inside the hello frame, where the frame type
// names the field; a POST has no frame, so it carries the standard
// bearer header, which `channels::reception::offered_pairing` reads.
// A page opened without a code sends no header at all: a city with no
// token configured is a city on loopback, and it admits every door.
export function bearing(token: string | null): Readonly<Record<string, string>> {
  return token === null ? {} : { authorization: `Bearer ${token}` };
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

  const queue: ServerFrame[] = [];
  let scheduled = false;
  // The ranges this page has been told it never received, oldest first,
  // and the near end of the page in flight. Ranges queue rather than
  // replace: two losses are two ranges, and the answer's own cursor is
  // what advances one - so a range that arrives while another is being
  // filled waits its turn and nothing is dropped between them.
  const gaps: Gap[] = [];
  let fetching: Seq | null = null;
  // The highest ledger sequence this page has folded, from the live
  // stream or a gap: the point a reconnect resumes after. Null until
  // the first record, and a page with no mark has nothing to resume.
  let mark: Seq | null = null;
  // The attempt the ladder scheduled, held so a link that turns out to
  // be refused can cancel it. A refused link that left this running
  // would reopen the socket behind a message telling the person the
  // opposite.
  let reconnect: ReturnType<typeof setTimeout> | null = null;

  function sendText(text: string): boolean {
    if (socket?.readyState !== WebSocket.OPEN) {
      return false;
    }
    socket.send(text);
    return true;
  }

  // Asks for one page of the oldest range still owed, and only when none
  // is in flight: the wire carries no request id, so a second question
  // for the same range would be an answer this page cannot tell apart
  // from the first.
  function askGap(): void {
    const front = gaps[0];
    if (front === undefined || fetching !== null) {
      return;
    }
    fetching = front.at;
    sendText(
      encodeFrame({
        query: { history_range: { from: front.at, to: front.to, limit: GAP_PAGE } },
      }),
    );
  }

  // The records of one page of a gap, folded like any other record: what
  // the page lost is what happened, and the fold is what draws it. A
  // `folded` gap (the stream lagged while the socket stayed open)
  // invalidates nothing: its answers were already asked after those
  // records. An `invalidating` gap (written while the socket was down)
  // marks stale each answer a record touches, since no answer since
  // the disconnect has seen it.
  function filled(range: HistoryRangeAnswer): void {
    // One report per page, naming the first field this build could not
    // read: a page of two hundred records is one question's answer, and
    // a notice per record would bury the question in its answer.
    let bad: string | null = null;
    const records = gaps[0]?.records ?? "folded";
    for (const record of range.records) {
      const unreadable = folded(record);
      if (records === "invalidating") asking.invalidate(record);
      bad ??= unreadable;
    }
    if (bad !== null) store.refused(unreadableRecord(lang, bad));
    if (fetching === null || range.from !== fetching) {
      // An answer to a page this page no longer waits for. Its records
      // are folded above; the range it belonged to has moved on.
      return;
    }
    fetching = null;
    const front = gaps.shift();
    // `??` rather than a null test: an absent cursor and an explicit one
    // are the same fact on this wire, and the server leaves the field out
    // when the range is answered.
    const cursor = range.next ?? null;
    if (front !== undefined && cursor !== null) {
      // The server's cursor, not arithmetic here: it is the one place
      // that knows where the Ledger holds the next record of the range.
      gaps.unshift({ ...front, at: cursor });
    }
    askGap();
  }

  // Folds one record and moves the mark past it.
  function folded(record: EventRecord): string | null {
    if (mark === null || record.seq > mark) mark = record.seq;
    return store.apply(record);
  }

  // A welcome names the ledger head. A page that knows where it stopped
  // and is at most `RESUME_PAGES` pages behind fetches the records in
  // between and asks again only what the dead socket took with it;
  // any other page asks every watched question again.
  function resume(head: Seq | null): void {
    const owed = gaps.reduce((far, gap) => (gap.to > far ? gap.to : far), mark ?? head ?? Seq.make(0));
    if (mark === null || head === null || head < mark || head - owed > RESUME_PAGES * GAP_PAGE) {
      asking.reconnected();
      return;
    }
    // A lagged gap still pending was folded-only because the answers of
    // the open socket had seen it; those answers died with the socket,
    // so its records now mark stale what they touch like the new gap's.
    gaps.splice(0, gaps.length, ...gaps.map((gap): Gap => ({ ...gap, records: "invalidating" })));
    if (head > owed) gaps.push({ at: Seq.make(owed + 1), to: head, records: "invalidating" });
    asking.resumed();
  }

  const asking = createAsking(
    (query: Query) => (isLive(link) ? sendText(encodeFrame({ query })) : false),
    now,
    // A question that never came back, and an answer that settles no
    // question, both land where every other refusal lands: the corner
    // once, and the bell until the person has read it. The recovery
    // is the one part written for a person, so it is said here, in
    // the language `<html lang>` states - which `app.tsx` keeps equal
    // to the person's choice, and which a screen reader reads the
    // page by.
    (phrase, error) => {
      const lang = langOf(document.documentElement.lang);
      store.refused({ ...error, recovery: say(lang, phrase) });
    },
  );

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
        // A new connection cannot be holding an answer the old one was
        // asked for, so the walk starts its front range again rather than
        // waiting for a page that will never arrive.
        fetching = null;
        resume(action.welcome.resume_from ?? null);
        askGap();
        return;
      case "deliver": {
        // The field this build could not read, if any, is reported here
        // rather than swallowed: the frame decoded and the socket is
        // still speaking this wire, so it goes where every other refusal
        // goes, and the rest of the record has already been folded.
        const bad = folded(action.event);
        asking.invalidate(action.event);
        if (bad !== null) store.refused(unreadableRecord(lang, bad));
        return;
      }
      case "answered":
        if ("history_range" in action.answer) {
          // The one answer that is this page's own question rather than a
          // view's: it settles no held question, so it is folded here and
          // never reaches the asking.
          filled(action.answer.history_range);
          return;
        }
        if ("city" in action.answer) {
          store.adoptCity(action.answer.city);
        }
        asking.answered(action.answer);
        return;
      case "saying":
        store.say(action.delta);
        return;
      case "logged":
        store.logged(action.line);
        return;
      case "lagged":
        gaps.push({ at: action.from, to: action.to, records: "folded" });
        askGap();
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
        requestAnimationFrame(drain);
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

  const [begun, first] = start(link);
  link = begun;
  state.set(begun.state);
  perform(first);

  return {
    state,
    belief: store.belief,
    asking,
    command(command) {
      return isLive(link) && sendText(encodeFrame({ command }));
    },
    retry() {
      step({ kind: "retry" });
    },
    dismissRefusal() {
      store.refused(null);
    },
    markNoticesSeen() {
      store.noticesSeen();
    },
  };
}
