// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";
import { Effect, Exit, Queue, Stream } from "effect";
import { createRoot } from "solid-js";

import { resourceFromEffect, signalFromStream } from "./bridge";

// Lets every fiber and every resource promise that is ready run to its
// next quiet point, so an assertion reads what the bridge wrote rather
// than what it was about to write.
async function settle(): Promise<void> {
  for (let i = 0; i < 4; i += 1) {
    await new Promise<void>((resolve) => {
      setTimeout(resolve, 0);
    });
  }
}

describe("bridge", () => {
  test("a stream element becomes the signal's value, in order", async () => {
    const queue = await Effect.runPromise(Queue.unbounded<number>());
    const seen: number[] = [];
    const dispose = createRoot((dispose) => {
      const value = signalFromStream(Stream.fromQueue(queue), 0);
      expect(value()).toBe(0);
      seen.push(value());
      return () => {
        seen.push(value());
        dispose();
      };
    });
    await Effect.runPromise(Queue.offer(queue, 1));
    await Effect.runPromise(Queue.offer(queue, 2));
    await settle();
    dispose();
    expect(seen).toEqual([0, 2]);
  });

  test("disposing the owner stops the stream reaching the signal", async () => {
    const queue = await Effect.runPromise(Queue.unbounded<number>());
    let read: () => number = () => -1;
    const dispose = createRoot((dispose) => {
      read = signalFromStream(Stream.fromQueue(queue), 0);
      return dispose;
    });
    await Effect.runPromise(Queue.offer(queue, 1));
    await settle();
    expect(read()).toBe(1);
    dispose();
    await Effect.runPromise(Queue.offer(queue, 2));
    await settle();
    expect(read()).toBe(1);
  });

  test("an effect's outcome reaches the view as an Exit, never as a throw", async () => {
    const { won, lost, dispose } = createRoot((dispose) => ({
      won: resourceFromEffect(Effect.succeed(7)),
      lost: resourceFromEffect(Effect.fail("no")),
      dispose,
    }));
    await settle();
    expect(won()).toEqual(Exit.succeed(7));
    expect(lost()).toEqual(Exit.fail("no"));
    expect(lost.error).toBeUndefined();
    dispose();
  });
});
