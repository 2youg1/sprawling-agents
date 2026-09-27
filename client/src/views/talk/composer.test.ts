// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import type { RunBelief } from "../../core/belief";
import { Address, RunId, Seq, TimeMs } from "../../wire";
import { modelMove, pills, sessionModel } from "./composer";
import type { Around, Picks } from "./composer";

const ignore = (): void => undefined;
const picks: Picks = { model: ignore, workspace: ignore, effort: ignore, mode: ignore };

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
  mode: "plan_goal",
};

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
  test("opens a new session only when a session already answers with another model", () => {
    const names = { endpoint: "local", model: "fake-chat" };
    expect(modelMove("local\u0000fake-chat", null)).toEqual({ kind: "select", names });
    expect(modelMove("local\u0000fake-chat", "fake-small")).toEqual({ kind: "reopen", names });
    expect(modelMove("local\u0000fake-chat", "fake-chat")).toEqual({ kind: "stay" });
  });
});
