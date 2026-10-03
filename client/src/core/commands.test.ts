// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { Address, Seq } from "../wire";
import { FIRST_POLICY, changeRunPolicy, nameSession, selectModel } from "./commands";

// What a person states about a model reaches the frame, and what they
// did not state is absent rather than guessed (client/Spec.lean §7).
describe("choosing a model", () => {
  test("carries the input kinds the person stated", () => {
    const frame = selectModel("zenmux", "vision-1", "main", { contextTokens: null, maxOutputTokens: null, input: "text_image" });
    expect("select_model" in frame ? frame.select_model.input : undefined).toBe("text_image");
  });

  test("leaves the input kinds to the city when nobody stated them", () => {
    const frame = selectModel("zenmux", "plain-1", "main");
    expect("select_model" in frame ? frame.select_model.input : undefined).toBeNull();
  });
});

// The two session verbs carry what the menu chose and nothing it did not
// (crates/wire/spec/Answer/Sessions.lean D27).
describe("the session menu's two verbs", () => {
  const room = Address.make("lab/bench");

  test("a name is sent trimmed, and an empty one takes the name back", () => {
    const frame = nameSession(room, Seq.make(41), "  wire audit ");
    expect("name_session" in frame ? { ...frame.name_session, idem: null } : null).toEqual({ room, began: Seq.make(41), name: "wire audit", idem: null });
    const cleared = nameSession(room, Seq.make(41), "   ");
    expect("name_session" in cleared ? cleared.name_session.name : null).toBe("");
  });

  test("a run policy change carries the whole policy for the room", () => {
    const policy = { ...FIRST_POLICY, mode: "work" as const, write: "create" as const };
    const frame = changeRunPolicy(room, policy);
    expect("change_run_policy" in frame ? { ...frame.change_run_policy, idem: null } : null).toEqual({ room, policy, idem: null });
  });
});
