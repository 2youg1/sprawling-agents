// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

type Stop = () => void;
export type Subscriber<T> = (value: T) => void;
type Update<T> = (change: (previous: T) => T) => void;
type Start<T> = (set: Subscriber<T>, update: Update<T>) => Stop | undefined;

export interface Readable<T> {
  readonly subscribe: (listen: Subscriber<T>, invalidate?: Stop) => Stop;
}

export interface Writable<T> extends Readable<T> {
  set: Subscriber<T>;
  update: Update<T>;
}

const queue: Stop[] = [];
let delivering = false;

function drain(): void {
  if (delivering) return;
  delivering = true;
  using _delivery = {
    [Symbol.dispose]() {
      queue.length = 0;
      delivering = false;
    },
  };
  for (const deliver of queue) deliver();
}

function unchanged<T>(previous: T, next: T): boolean {
  if (next !== null && (typeof next === "object" || typeof next === "function")) return false;
  return previous === next || (Number.isNaN(previous) && Number.isNaN(next));
}

function cell<T>(start: Start<T>, initial?: { value: T }): Writable<T> {
  const listeners = new Set<{ listen: Subscriber<T>; invalidate: Stop | undefined }>();
  let snapshot = initial;
  let active = false;
  let cleanup: Stop | undefined;

  function set(value: T): void {
    if (snapshot && unchanged(snapshot.value, value)) return;
    snapshot = { value };
    if (!active) return;
    for (const observer of listeners) {
      observer.invalidate?.();
      queue.push(() => {
        observer.listen(value);
      });
    }
    drain();
  }

  function update(change: (previous: T) => T): void {
    if (snapshot) set(change(snapshot.value));
  }

  function subscribe(listen: Subscriber<T>, invalidate?: Stop): Stop {
    const observer = { listen, invalidate };
    listeners.add(observer);
    if (listeners.size === 1) {
      cleanup = start(set, update);
      active = true;
    }
    if (snapshot) listen(snapshot.value);
    return () => {
      if (listeners.delete(observer) && listeners.size === 0) {
        active = false;
        const stop = cleanup;
        cleanup = undefined;
        stop?.();
      }
    };
  }

  return { set, update, subscribe };
}

export function writable<T>(value: T): Writable<T> {
  return cell(() => undefined, { value });
}

export function readonly<T>(source: Readable<T>): Readable<T> {
  return { subscribe: (listen, invalidate) => source.subscribe(listen, invalidate) };
}

export function readable<T>(value: T, start: Start<T> = () => undefined): Readable<T> {
  return readonly(cell(start, { value }));
}

export function get<T>(source: Readable<T>): T {
  let current!: T;
  const stop = source.subscribe((value) => {
    current = value;
  });
  stop();
  return current;
}

export function derived<T, R>(source: Readable<T>, project: (value: T) => R): Readable<R> {
  return readonly(
    cell<R>((set) =>
      source.subscribe((value) => {
        set(project(value));
      }),
    ),
  );
}

export function derivedList<T, R>(
  sources: readonly Readable<T>[],
  project: (values: T[]) => R,
): Readable<R> {
  return readonly(
    cell<R>((set) => {
      const pending = new Set(sources.keys());
      const values: T[] = [];
      let listening = true;
      if (sources.length === 0) set(project(values));
      const stops = sources.map((source, index) =>
        source.subscribe(
          (value) => {
            values[index] = value;
            pending.delete(index);
            if (listening && pending.size === 0) set(project(values));
          },
          () => {
            pending.add(index);
          },
        ),
      );
      return () => {
        listening = false;
        for (const stop of stops) stop();
      };
    }),
  );
}
