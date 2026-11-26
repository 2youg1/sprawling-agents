// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import type { Line } from "../line";
import type { Door, Refusal } from "./connect";
import type { Session } from "./handshake";
import { reach } from "./transport";
import { opener, payloadBytes, payloadOf, sealer } from "./seal";
import { talk } from "./session";

const TO_CITY = new Uint8Array(32).fill(1);
const TO_DEVICE = new Uint8Array(32).fill(2);
const FRAMES = 24;

interface Wire {
  readonly door: Door;
  readonly sent: Uint8Array<ArrayBuffer>[];
  readonly arrive: (bytes: Uint8Array<ArrayBuffer> | null) => void;
  readonly shut: () => boolean;
}

function wire(): Wire {
  const sent: Uint8Array<ArrayBuffer>[] = [];
  const queued: (Uint8Array<ArrayBuffer> | null)[] = [];
  const waiting: ((bytes: Uint8Array<ArrayBuffer> | null) => void)[] = [];
  let closed = false;
  const nobody: Refusal = { kind: "closed", code: null };
  const door: Door = {
    send: (bytes) => {
      sent.push(bytes);
    },
    next: () => {
      if (queued.length > 0) return Promise.resolve(queued.shift() ?? null);
      return new Promise((wake) => {
        waiting.push(wake);
      });
    },
    closed: Promise.resolve(null),
    ended: () => Promise.resolve(nobody),
    refuse: () => nobody,
    close: () => {
      closed = true;
    },
  };
  return {
    door,
    sent,
    arrive: (bytes) => {
      const wake = waiting.shift();
      if (wake === undefined) queued.push(bytes);
      else wake(bytes);
    },
    shut: () => closed,
  };
}

function device(): Session {
  return { sealer: sealer(TO_CITY, "device_to_city"), opener: opener(TO_DEVICE, "city_to_device") };
}

interface Heard {
  readonly hearing: { opened: () => void; heard: (text: string) => void; closed: () => void };
  readonly said: string[];
  readonly events: string[];
}

function listener(): Heard {
  const said: string[] = [];
  const events: string[] = [];
  return {
    said,
    events,
    hearing: {
      opened: () => events.push("opened"),
      heard: (text) => said.push(text),
      closed: () => events.push("closed"),
    },
  };
}

async function settled(until: () => boolean): Promise<void> {
  for (let turn = 0; turn < 500 && !until(); turn += 1) {
    await new Promise((wake) => setTimeout(wake, 1));
  }
}

const texts = Array.from({ length: FRAMES }, (_, at) => `{"frame":${String(at)}}`);

describe("a line over a remote session", () => {
  test("preserves ordered frames across fragmented duplex stream chunks and backpressure", async () => {
    const location = Object.getOwnPropertyDescriptor(globalThis, "location");
    const transport = Object.getOwnPropertyDescriptor(globalThis, "WebTransport");
    const origin = "https://city.example";
    const frames = Array.from({ length: 12 }, (_, value) => value);
    const sent: number[][] = [];
    const bytes = new Uint8Array([
      ...frames.flatMap((value) => [0, 0, 0, 0, 1, value]),
      1, 0, 0, 0, 0,
    ]);
    class Loop {
      readonly ready = Promise.resolve();
      readonly closed = Promise.resolve();
      close() {}
      createBidirectionalStream() {
        return Promise.resolve({
          readable: new ReadableStream({
            start(controller) {
              controller.enqueue(bytes.slice(0, 3));
              controller.enqueue(bytes.slice(3));
              controller.close();
            },
          }),
          writable: new WritableStream<Uint8Array>({
            write(chunk) {
              sent.push(Array.from(chunk));
            },
          }),
        });
      }
    }
    Object.defineProperty(globalThis, "location", { configurable: true, value: new URL(origin) });
    Object.defineProperty(globalThis, "WebTransport", { configurable: true, value: Loop });
    await (async () => {
      const door = await reach(origin + "/remote/session", new Uint8Array([55]));
      door?.send(new Uint8Array([56, 57]));
      const received = [];
      for (const _frame of frames) received.push((await door?.next())?.[0]);
      expect(received).toEqual(frames);
      expect(await door?.closed).toBeNull();
      expect(sent).toEqual([[0, 0, 0, 0, 1, 55], [0, 0, 0, 0, 2, 56, 57]]);
    })().finally(() => {
      if (location === undefined) Reflect.deleteProperty(globalThis, "location");
      else Object.defineProperty(globalThis, "location", location);
      if (transport === undefined) Reflect.deleteProperty(globalThis, "WebTransport");
      else Object.defineProperty(globalThis, "WebTransport", transport);
    });
  });

  test("lets the link send its greeting from the opened callback", async () => {
    const { door, sent } = wire();
    let line: Line | undefined;
    let accepted = false;
    const hearing = listener().hearing;
    line = talk(door, device(), {
      ...hearing,
      opened: () => {
        accepted = line?.send("greeting") === true;
      },
    });
    await Promise.resolve();
    expect(accepted).toBe(true);
    await settled(() => sent.length === 1);
    expect(sent).toHaveLength(1);
    line.close();
  });

  test("sends every frame the link sends, sealed in the order it sent them", async () => {
    const { door, sent } = wire();
    const line = talk(door, device(), listener().hearing);
    const went = texts.map((text) => line.send(text));
    await settled(() => sent.length === FRAMES);
    const city = opener(TO_CITY, "device_to_city");
    const opened: (string | null)[] = [];
    for (const bytes of sent) {
      const plain = await city.open(bytes);
      const payload = plain === null ? null : payloadOf(plain);
      opened.push(payload?.kind === "frame" ? payload.text : null);
    }
    expect({ went, opened }).toEqual({ went: texts.map(() => true), opened: texts });
  });

  test("hands the link every frame the city sends, in the order it sent them", async () => {
    const { door, arrive } = wire();
    const heard = listener();
    talk(door, device(), heard.hearing);
    const city = sealer(TO_DEVICE, "city_to_device");
    const sealed = await Promise.all(
      texts.map((text) => city.seal(payloadBytes({ kind: "frame", text }))),
    );
    for (const bytes of sealed) arrive(bytes);
    await settled(() => heard.said.length === FRAMES);
    expect({ said: heard.said, events: heard.events }).toEqual({ said: texts, events: ["opened"] });
  });

  test("closes once on a frame that does not open, and sends nothing after", async () => {
    const { door, sent, arrive, shut } = wire();
    const heard = listener();
    const line = talk(door, device(), heard.hearing);
    arrive(new Uint8Array([1, 2, 3]));
    await settled(() => heard.events.includes("closed"));
    const after = line.send(texts[0] ?? "");
    line.close();
    expect({ events: heard.events, shut: shut(), after, sent: sent.length }).toEqual({
      events: ["opened", "closed"],
      shut: true,
      after: false,
      sent: 0,
    });
  });

  test("closes once when the city ends the connection", async () => {
    const { door, arrive } = wire();
    const heard = listener();
    talk(door, device(), heard.hearing);
    arrive(null);
    await settled(() => heard.events.includes("closed"));
    expect(heard.events).toEqual(["opened", "closed"]);
  });
});
