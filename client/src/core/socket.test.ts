// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { afterEach, describe, expect, test } from "bun:test";

import { openConnection } from "./socket";

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
};

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
    const conn = openConnection("ws://city.invalid/ws", null);
    const first = FakeSocket.opened[0];
    expect(first).toBeDefined();
    first?.onopen?.();
    first?.onmessage?.({ data: "{\"welcome\":" });

    expect(conn.state().kind).toBe("refused");
    expect(first?.closed).toBe(true);
    expect(booked.filter((entry) => !entry.cancelled)).toHaveLength(0);
    expect(FakeSocket.opened).toHaveLength(1);
  });

  // A socket that drops is the outage the ladder is for, so one attempt
  // is booked and it opens a second socket when it fires.
  test("books a reconnect when the socket drops", () => {
    install();
    openConnection("ws://city.invalid/ws", null);
    const first = FakeSocket.opened[0];
    first?.onopen?.();
    first?.onclose?.();

    expect(booked).toHaveLength(1);
    booked[0]?.run();
    expect(FakeSocket.opened).toHaveLength(2);
  });
});
