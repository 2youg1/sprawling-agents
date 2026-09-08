// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The one seam between the effectful core and the views. Effect owns
// what can fail, retry or be interrupted; Solid owns what is drawn. A
// value crosses here only as data: a stream has no error channel, and an
// effect's outcome arrives as an `Exit` the view matches on.

import { Effect, Fiber, Stream } from "effect";
import type { Exit } from "effect";
import { createResource, createSignal, onCleanup } from "solid-js";
import type { Accessor, Resource } from "solid-js";

// Each element of the stream becomes the signal's value, in order. The
// stream runs on a fiber owned by the current Solid owner: when that
// owner is disposed the fiber is interrupted, so a component that unmounts
// stops listening rather than writing into a signal nobody reads.
export function signalFromStream<A>(
  stream: Stream.Stream<A>,
  initial: A,
): Accessor<A> {
  const [value, setValue] = createSignal<A>(initial);
  const fiber = Effect.runFork(
    Stream.runForEach(stream, (element) =>
      Effect.sync(() => {
        setValue(() => element);
      }),
    ),
  );
  onCleanup(() => {
    Effect.runFork(Fiber.interrupt(fiber));
  });
  return value;
}

// Runs an effect once and hands the view its `Exit`. Failure is a value
// in the `Exit`, so the resource's own `error` field never fires and no
// view has to catch anything.
export function resourceFromEffect<A, E>(
  effect: Effect.Effect<A, E>,
): Resource<Exit.Exit<A, E>> {
  const [resource] = createResource(() => Effect.runPromiseExit(effect));
  return resource;
}
