// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import type { RunBelief } from "../../core/belief";
import { Address, RunId, Seq, TimeMs } from "../../wire";
import { boxKey, menuColumns, modelMove, picksFor, pills, sessionModel } from "./composer";
import type { Around, BoxKey, Picks } from "./composer";

const ignore = (): void => undefined;
const picks: Picks = { model: ignore, workspace: ignore, effort: ignore };

const around: Around = {
  served: [
    { endpoint: "local", model: "fake-small", label: "Local" },
    { endpoint: "local", model: "fake-chat", label: "Local" },
  ],
  chosen: { endpoint: "local", model: "fake-chat" },
  session: "fake-small",
  rooms: [],
  here: null,
  effort: null,
};

describe("the `/` menu", () => {
  test("a line no verb matches opens no menu, so Enter reaches the box", () => {
    expect(menuColumns("en", "/etc/hosts is broken")).toEqual([]);
    expect(menuColumns("en", "/st").flatMap((column) => column.rows.map((row) => row.id))).toEqual([
      "/steer",
      "/stop",
    ]);
  });
});

describe("the model pill", () => {
  test("a session's model is what the pill shows, not the city's next pick", () => {
    const [model] = pills("en", around, picks);
    expect(model.value).toBe("local\u0000fake-small");
  });

  test("with no session in the room the pill shows the chosen main model", () => {
    const [model] = pills("en", { ...around, session: null }, picks);
    expect(model.value).toBe("local\u0000fake-chat");
  });
});

const room = Address.make("mayor");

function run(id: string, lastSeq: number, model: string | null): RunBelief {
  return {
    run: RunId.make(id),
    addr: room,
    started: TimeMs.make(lastSeq),
    task: null,
    goal: null,
    lastSeq: Seq.make(lastSeq),
    doing: { kind: "frozen", completion: null },
    model,
    pr: null,
    ask: null,
    local: false,
    saying: "",
    thinking: "",
  };
}

describe("the session's model", () => {
  test("is the newest run's since the session began, and none once a new session opens", () => {
    const runs = [run("00000000-0000-4000-8000-000000000001", 3, "fake-small"), run("00000000-0000-4000-8000-000000000002", 7, "fake-chat")];
    expect(sessionModel(runs, Seq.make(2))).toBe("fake-chat");
    expect(sessionModel(runs, Seq.make(9))).toBeNull();
  });

  test("is the newest known model when the newest run has not called one yet", () => {
    const runs = [run("00000000-0000-4000-8000-000000000001", 3, "fake-small"), run("00000000-0000-4000-8000-000000000002", 7, null)];
    expect(sessionModel(runs, Seq.make(2))).toBe("fake-small");
  });
});

describe("picking a model", () => {
  test("never changes the model of an existing session", () => {
    const names = { endpoint: "local", model: "fake-chat" };
    expect(modelMove("local\u0000fake-chat", null)).toEqual({ kind: "select", names });
    expect(modelMove("local\u0000fake-chat", "fake-small")).toEqual({ kind: "stay" });
    expect(modelMove("local\u0000fake-chat", "fake-chat")).toEqual({ kind: "stay" });
  });
});

test("a model pick sends selection alone, never a session command", () => {
  const sent: unknown[] = [];
  const hands = { send: (command: unknown) => { sent.push(command); return true; }, go: ignore, chooseEffort: ignore };
  picksFor(hands, null).model("local\u0000fake-chat");
  expect(sent).toHaveLength(1);
  expect(sent).toMatchObject([{ select_model: { endpoint: "local", model: "fake-chat", tag: "main" } }]);
  picksFor(hands, "fake-small").model("local\u0000fake-chat");
  expect(sent).toHaveLength(1);
});

describe("a key in the box", () => {
  const pressed = (key: string, shiftKey = false, isComposing = false) => ({ key, shiftKey, isComposing });
  const keys = ["Enter", "Tab", "ArrowUp", "a"] as const;
  const boxes = [
    { menu: false, empty: false },
    { menu: false, empty: true },
    { menu: true, empty: false },
  ] as const;

  // Every key, with and without Shift, in every box: a key that confirms
  // an input method's composition is the input method's, so an Enter that
  // picks a candidate never sends half a sentence.
  test("a composing key is always the box's", () => {
    const decided: BoxKey[] = keys.flatMap((key) =>
      boxes.flatMap((box) => [false, true].map((shift) => boxKey(pressed(key, shift, true), box))),
    );
    expect(new Set(decided)).toEqual(new Set(["type"]));
  });

  test("Enter sends and Shift+Enter starts a line", () => {
    expect([boxKey(pressed("Enter"), boxes[0]), boxKey(pressed("Enter", true), boxes[0])]).toEqual(["send", "type"]);
  });

  test("ArrowUp recalls only in an empty box, where it cannot move the caret", () => {
    expect([boxKey(pressed("ArrowUp"), boxes[1]), boxKey(pressed("ArrowUp"), boxes[0])]).toEqual(["recall", "type"]);
  });

  // Under an open menu Tab completes and every other key is offered to
  // the menu first; one the menu does not take is the box's own again.
  test("an open menu takes Tab and is offered the rest", () => {
    expect(keys.map((key) => boxKey(pressed(key), boxes[2]))).toEqual(["menu", "complete", "menu", "menu"]);
    expect(boxKey(pressed("Enter"), { ...boxes[2], menu: false })).toBe("send");
  });
});
