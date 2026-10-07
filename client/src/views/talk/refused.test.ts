// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The three readings of the thread's refusal card: where the city's
// own sentence stands - as a line, one fold away, or not at all because
// the page's words already say it - and which ways out the card offers.

import { describe, expect, test } from "bun:test";

import { toFragment } from "../../core/route";
import { failedLook, refusedLook, unreadableLook } from "./refused";
import type { Way } from "./refused";
import type { AxError } from "../../wire";

const OVERFLOW: AxError = {
  action: "call the model",
  code: "E_PROVIDER",
  nearby: [],
  provider: { kind: "overflow", status: 400 },
  recovery: "start a new session with /new",
  retry: "no",
  subject: "gallery/model",
};

const BUSY: AxError = {
  action: "write src/lib.rs",
  code: "E_WORKTREE_BUSY",
  nearby: [],
  recovery: "another run holds this worktree",
  retry: "yes",
  subject: "shop/main",
};

const SETTINGS: Way = { kind: "link", text: "open model settings", href: toFragment({ kind: "setup" }) };

describe("a refusal a turn came to", () => {
  test("a failed model call says its kind and folds the city's sentence, with the settings as the way out", () => {
    expect(refusedLook("en", OVERFLOW)).toEqual({
      role: undefined,
      head: "E_PROVIDER",
      after: "call the model · gallery/model",
      lines: [
        {
          text: "the conversation no longer fits this model's window (the provider answered 400); start a new session that keeps the summary, or choose a model with a larger window",
          ink: "faint",
        },
      ],
      fold: { summary: "the city's own words", text: "start a new session with /new" },
      ways: [SETTINGS],
    });
  });

  test("any other refusal shows the city's sentence as it is and offers no settings", () => {
    expect(refusedLook("en", BUSY)).toEqual({
      role: undefined,
      head: "E_WORKTREE_BUSY",
      after: "write src/lib.rs · shop/main",
      lines: [{ text: "another run holds this worktree", ink: "faint" }],
      fold: undefined,
      ways: [],
    });
  });
});

describe("a message that got no reply", () => {
  const retry = (): void => undefined;

  test("is an alert that says the next step in the page's words and folds the city's different sentence", () => {
    expect(failedLook("en", "no reply", BUSY, { settings: "decided", onRetry: retry })).toEqual({
      role: "alert",
      head: "no reply",
      after: undefined,
      lines: [
        { text: "E_WORKTREE_BUSY · shop/main", ink: "quiet" },
        { text: "Wait for the other run working in this tree to finish, then try again.", ink: "faint" },
      ],
      fold: { summary: "the city's own words", text: "another run holds this worktree" },
      ways: [{ kind: "press", label: "send again", onPress: retry }],
    });
  });

  test("folds nothing when the city's sentence is the next step, and a caller's settings answer wins", () => {
    const said: AxError = { ...BUSY, recovery: "Wait for the other run working in this tree to finish, then try again." };
    const look = failedLook("en", "no reply", said, { settings: "offered", onRetry: undefined });
    expect([look.fold, look.ways]).toEqual([undefined, [SETTINGS]]);
    expect(failedLook("en", "no reply", OVERFLOW, { settings: "absent", onRetry: undefined }).ways).toEqual([]);
  });

  test("with no refusal says only what happened", () => {
    expect(failedLook("en", "no reply", undefined, { settings: "decided", onRetry: undefined })).toEqual({
      role: "alert",
      head: "no reply",
      after: undefined,
      lines: [],
      fold: undefined,
      ways: [],
    });
  });
});

test("a record that did not read back leads to the Ledger", () => {
  expect(unreadableLook("en", 7, "an unknown frame")).toEqual({
    role: undefined,
    head: "record 7 did not read back",
    after: "an unknown frame",
    lines: [],
    fold: undefined,
    ways: [{ kind: "link", text: "read it in the Ledger", href: toFragment({ kind: "record", lens: "ledger" }) }],
  });
});
