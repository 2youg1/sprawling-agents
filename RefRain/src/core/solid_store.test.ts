// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { expect, test } from "bun:test";
import { createEffect, createRoot, createSignal, flush, type Setter } from "solid-js";

import { storeValue } from "./solid_store";
import { readable, type Readable } from "./store";

test("file and session readers share one source subscription and stop on owner disposal", () => {
  interface Work {
    subject: "files" | "session";
    draft: string;
  }
  let starts = 0;
  let stops = 0;
  let publish: (work: Work) => void = () => undefined;
  const source = readable<Work>({ subject: "files", draft: "selected material" }, (set) => {
    starts += 1;
    publish = set;
    return () => {
      stops += 1;
    };
  });
  const subjects: Work["subject"][] = [];
  const drafts: string[] = [];
  const stop = createRoot((dispose) => {
    const current = storeValue(source);
    createEffect(
      () => current().subject,
      (subject) => {
        subjects.push(subject);
      },
    );
    createEffect(
      () => current().draft,
      (draft) => {
        drafts.push(draft);
      },
    );
    return dispose;
  });
  flush();
  publish({ subject: "session", draft: "selected material with a question" });
  flush();
  publish({ subject: "files", draft: "selected material with a question" });
  flush();
  stop();
  publish({ subject: "session", draft: "after disposal" });
  flush();

  expect(starts).toBe(1);
  expect(stops).toBe(1);
  expect(subjects).toEqual(["files", "session", "files"]);
  expect(drafts).toEqual([
    "selected material",
    "selected material with a question",
    "selected material with a question",
  ]);
});

test("a changing Readable starts and releases only the current source", async () => {
  const started: string[] = [];
  const released: string[] = [];
  const seen: (string | undefined)[] = [];
  let emitA: (value: string) => void = () => undefined;
  const a = readable("A", (emit) => {
    started.push("A");
    emitA = emit;
    return () => {
      released.push("A");
    };
  });
  const b = readable("B", () => {
    started.push("B");
    return () => {
      released.push("B");
    };
  });
  const skipped = readable("skipped", () => {
    started.push("skipped");
    return undefined;
  });
  let switchTo: Setter<Readable<string> | undefined> = () => undefined;
  const stop = createRoot((dispose) => {
    const [source, change] = createSignal<Readable<string> | undefined>(a);
    switchTo = change;
    const value = storeValue(source);
    createEffect(value, (next) => {
      seen.push(next);
    });
    return dispose;
  });

  flush();
  emitA("A updated");
  await Promise.resolve();
  flush();
  switchTo(skipped);
  switchTo(b);
  flush();
  emitA("A after switch");
  await Promise.resolve();
  flush();
  switchTo(undefined);
  flush();
  switchTo(b);
  flush();
  stop();

  expect(started).toEqual(["A", "B", "B"]);
  expect(released).toEqual(["A", "B", "B"]);
  expect(seen).toEqual(["A", "A updated", "B", undefined, "B"]);
});
