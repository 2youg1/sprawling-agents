// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { afterEach, describe, expect, test } from "bun:test";
import { get } from "svelte/store";

import { doctorRefresh, steer } from "./commands";
import { plainDial } from "./line";
import { openConnection } from "./socket";
import { RunId, WIRE_HASH, WIRE_V } from "../wire";

// One fake socket per `new WebSocket(...)`, kept so a test can deliver
// a frame and read whether the browser half closed it.
class FakeSocket {
  static readonly opened: FakeSocket[] = [];
  static readonly OPEN = 1;
  readonly readyState = 1;
  closed = false;
  sent: string[] = [];
  onopen: (() => void) | null = null;
  onmessage: ((message: { data: string }) => void) | null = null;
  onclose: (() => void) | null = null;
  onerror: (() => void) | null = null;

  constructor() {
    FakeSocket.opened.push(this);
  }

  send(text: string): void {
    this.sent.push(text);
  }

  close(): void {
    this.closed = true;
    this.onclose?.();
  }
}

interface Booked {
  readonly run: () => void;
  cancelled: boolean;
}

const booked: Booked[] = [];

function install(): void {
  FakeSocket.opened.length = 0;
  booked.length = 0;
  Object.assign(globalThis, {
    WebSocket: FakeSocket,
    requestAnimationFrame: (run: () => void): number => {
      run();
      return 0;
    },
    setTimeout: (run: () => void): number => {
      booked.push({ run, cancelled: false });
      return booked.length - 1;
    },
    clearTimeout: (id: number): void => {
      const entry = booked[id];
      if (entry !== undefined) {
        entry.cancelled = true;
      }
    },
  });
}

const realTimers = {
  setTimeout: globalThis.setTimeout,
  clearTimeout: globalThis.clearTimeout,
  requestAnimationFrame: globalThis.requestAnimationFrame,
  WebSocket: globalThis.WebSocket,
  document: globalThis.document,
};

// A page the browser has hidden: it never calls an animation frame, and
// it tells the page when it is shown again through the one listener kept
// here.
function hide(): { show: () => void } {
  const heard: (() => void)[] = [];
  const page = {
    visibilityState: "hidden",
    documentElement: { lang: "en" },
    addEventListener: (_type: string, listener: () => void): void => {
      heard.push(listener);
    },
  };
  Object.assign(globalThis, {
    document: page,
    requestAnimationFrame: (): number => 0,
  });
  return {
    show: () => {
      page.visibilityState = "visible";
      for (const listener of heard) listener();
    },
  };
}

function runBooked(): void {
  for (const entry of booked.splice(0)) {
    if (!entry.cancelled) entry.run();
  }
}

afterEach(() => {
  Object.assign(globalThis, realTimers);
});

describe("the browser half", () => {
  // The defect, end to end: a frame the client cannot read used to
  // close the socket, which the machine read as an outage, so a fresh
  // socket opened 250 ms later and met the same frame. Here the ladder
  // books nothing, and whatever it had booked is cancelled.
  test("cancels the reconnect when a frame cannot be read", () => {
    install();
    const conn = openConnection(plainDial("ws://city.invalid/ws"), "en");
    const first = FakeSocket.opened[0];
    expect(first).toBeDefined();
    first?.onopen?.();
    first?.onmessage?.({ data: "{\"welcome\":" });

    expect(get(conn.state).kind).toBe("refused");
    expect(first?.closed).toBe(true);
    expect(booked.filter((entry) => !entry.cancelled)).toHaveLength(0);
    expect(FakeSocket.opened).toHaveLength(1);
  });

  // A socket that drops is the outage the ladder is for, so one attempt
  // is booked and it opens a second socket when it fires.
  test("books a reconnect when the socket drops", () => {
    install();
    openConnection(plainDial("ws://city.invalid/ws"), "en");
    const first = FakeSocket.opened[0];
    first?.onopen?.();
    first?.onclose?.();

    expect(booked).toHaveLength(1);
    booked[0]?.run();
    expect(FakeSocket.opened).toHaveLength(2);
  });

  // The header's promise: what arrived between two paints is one belief
  // update, so a burst of tokens repaints the page once and not once per
  // token.
  test("folds a frame's burst into one belief update", () => {
    install();
    const painting: (() => void)[] = [];
    Object.assign(globalThis, {
      requestAnimationFrame: (run: () => void): number => painting.push(run),
    });
    const conn = openConnection(plainDial("ws://city.invalid/ws"), "en");
    const first = FakeSocket.opened[0];
    first?.onopen?.();
    const welcome = { wire_v: WIRE_V, schema: WIRE_HASH, resume_from: null, city: null };
    first?.onmessage?.({ data: JSON.stringify({ welcome }) });
    let updates = 0;
    const stop = conn.belief.subscribe(() => {
      updates += 1;
    });
    const run = "00000000-0000-4000-8000-000000000001";
    for (const said of ["a", "b", "c"]) {
      first?.onmessage?.({ data: JSON.stringify({ delta: { run, increment: { said } } }) });
    }
    for (const paint of painting.splice(0)) paint();
    stop();

    expect({ updates, saying: get(conn.belief).runs[run]?.saying }).toEqual({
      updates: 2,
      saying: "abc",
    });
  });
  // A browser stops calling animation frames in a hidden tab, so a page
  // that drained only on a paint held every frame - an approval request
  // included - until the person came back to the tab.
  test("drains on a timer while the page is hidden", () => {
    install();
    hide();
    const conn = openConnection(plainDial("ws://city.invalid/ws"), "en");
    const first = FakeSocket.opened[0];
    first?.onopen?.();
    const welcome = { wire_v: WIRE_V, schema: WIRE_HASH, resume_from: null, city: null };
    first?.onmessage?.({ data: JSON.stringify({ welcome }) });
    const run = "00000000-0000-4000-8000-000000000001";
    first?.onmessage?.({ data: JSON.stringify({ delta: { run, increment: { said: "a" } } }) });
    runBooked();

    expect(get(conn.belief).runs[run]?.saying).toBe("a");
  });

  // A tab shown again while the ladder waits tries at once: the person is
  // looking now, and the rung's remaining seconds are seconds of a blank
  // page.
  test("retries at once when a hidden page in backoff is shown", () => {
    install();
    const page = hide();
    openConnection(plainDial("ws://city.invalid/ws"), "en");
    const first = FakeSocket.opened[0];
    first?.onopen?.();
    first?.onclose?.();
    page.show();

    expect({ opened: FakeSocket.opened.length, waiting: booked.filter((entry) => !entry.cancelled).length })
      .toEqual({ opened: 2, waiting: 0 });
  });
  // Words said while the link is down are the person's, not the socket's:
  // they wait for the city and go out, in order, once it greets again,
  // and the banner counts them meanwhile.
  test("holds words said off the link and sends them on the welcome", () => {
    install();
    const conn = openConnection(plainDial("ws://city.invalid/ws"), "en");
    const welcome = JSON.stringify({ welcome: { wire_v: WIRE_V, schema: WIRE_HASH, resume_from: null, city: null } });
    FakeSocket.opened[0]?.onopen?.();
    FakeSocket.opened[0]?.onclose?.();
    const words = steer(RunId.make("00000000-0000-4000-8000-000000000001"), "and the tests");
    const accepted = conn.command(words);
    const held = get(conn.unsent);
    runBooked();
    const second = FakeSocket.opened[1];
    second?.onopen?.();
    second?.onmessage?.({ data: welcome });

    expect({ accepted, held, after: get(conn.unsent), sent: second?.sent.slice(1) }).toEqual({
      accepted: true,
      held: 1,
      after: 0,
      sent: [JSON.stringify({ command: words })],
    });
  });
  // A refused link reaches no welcome until the person acts, and the
  // only lever some refusals offer is a reload, which drops the queue:
  // so the words are not taken, and the composer keeps the draft.
  test("refuses to hold words while the link is refused", () => {
    install();
    const conn = openConnection(plainDial("ws://city.invalid/ws"), "en");
    FakeSocket.opened[0]?.onopen?.();
    FakeSocket.opened[0]?.onmessage?.({ data: "{\"welcome\":" });
    const accepted = conn.command(steer(RunId.make("00000000-0000-4000-8000-000000000001"), "and the tests"));

    expect({ state: get(conn.state).kind, accepted, held: get(conn.unsent) })
      .toEqual({ state: "refused", accepted: false, held: 0 });
  });

  // A press made while the link is down sends nothing, and the page says
  // so where every refusal lands, instead of a button that does nothing.
  test("says in the corner that a command made off the link was not sent", () => {
    install();
    const conn = openConnection(plainDial("ws://city.invalid/ws"), "en");
    FakeSocket.opened[0]?.onopen?.();
    FakeSocket.opened[0]?.onclose?.();
    const sent = conn.command(doctorRefresh());

    expect({ sent, refusal: get(conn.belief).refusal }).toEqual({
      sent: false,
      refusal: {
        code: "E_TIMEOUT",
        action: "send a command to the city",
        subject: "doctor_refresh",
        recovery: "wait until the page says it is connected, then do it again",
        nearby: [],
        retry: "yes",
      },
    });
  });

  // Fifty records written while the page was away are fifty records to
  // fetch, not every answer on the page asked again: the welcome names
  // the ledger head, and the page asks for the range between its own
  // high-water mark and that head.
  test("fetches the records written while it was away rather than a snapshot", () => {
    install();
    const conn = openConnection(plainDial("ws://city.invalid/ws"), "en");
    const first = FakeSocket.opened[0];
    first?.onopen?.();
    const welcome = { wire_v: WIRE_V, schema: WIRE_HASH, resume_from: 10, city: null };
    first?.onmessage?.({ data: JSON.stringify({ welcome }) });
    const metrics = {
      approvals_waiting: 0,
      buildings: 0,
      discards_outstanding: 0,
      events: 10,
      runs_active: 0,
      runs_frozen: 0,
      signals_waiting: 0,
    };
    const record = {
      seq: 10,
      prev: "0".repeat(64),
      t: 1,
      v: 1,
      who: "city",
      run: "00000000-0000-4000-8000-000000000001",
      kind: "log_truncated",
      data: {},
    };
    first?.onmessage?.({ data: JSON.stringify({ event: record }) });
    const stop = conn.asking.ask("metrics").subscribe(() => undefined);
    first?.onmessage?.({ data: JSON.stringify({ answered: { ask_id: 1, as_of: 11, outcome: { answer: { metrics } } } }) });
    const before = booked.length;
    first?.onclose?.();
    booked[before]?.run();
    const second = FakeSocket.opened[1];
    second?.onopen?.();
    second?.onmessage?.({ data: JSON.stringify({ welcome: { ...welcome, resume_from: 60 } }) });

    const asked: unknown[] | undefined = second?.sent.slice(1).map((text): unknown => JSON.parse(text));
    stop();

    expect(asked).toEqual([{ ask: { ask_id: 2, query: { history_range: { from: 11, to: 60, limit: 200 } } } }]);
  });
  // A welcome from another ledger - the city was made again - names a
  // head the old mark means nothing against: the page forgets what it
  // folded and asks everything again rather than fetching a range.
  test("rebuilds from a snapshot when the ledger's epoch changed", () => {
    install();
    const conn = openConnection(plainDial("ws://city.invalid/ws"), "en");
    const first = FakeSocket.opened[0];
    first?.onopen?.();
    const welcome = { wire_v: WIRE_V, schema: WIRE_HASH, resume_from: 10, city: null, epoch: "a".repeat(64) };
    first?.onmessage?.({ data: JSON.stringify({ welcome }) });
    const record = {
      seq: 10,
      prev: "0".repeat(64),
      t: 1,
      v: 1,
      who: "city",
      run: "00000000-0000-4000-8000-000000000001",
      kind: "log_truncated",
      data: {},
    };
    first?.onmessage?.({ data: JSON.stringify({ event: record }) });
    const stop = conn.asking.ask("metrics").subscribe(() => undefined);
    const before = booked.length;
    first?.onclose?.();
    booked[before]?.run();
    const second = FakeSocket.opened[1];
    second?.onopen?.();
    const renewed = { ...welcome, resume_from: 60, epoch: "b".repeat(64) };
    second?.onmessage?.({ data: JSON.stringify({ welcome: renewed }) });

    const asked: unknown[] | undefined = second?.sent.slice(1).map((text): unknown => JSON.parse(text));
    stop();

    expect(asked).toEqual([{ ask: { ask_id: 2, query: "metrics" } }]);
  });
});
