// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { pressCoin, type Face } from "./coin_face";

// Every face the key can show, so the property below is checked over
// the whole type, as the Lean theorem quantifies over it.
const FACES: readonly Face[] = ["send", "idle", "stop"];

describe("the coin key", () => {
  // `only_the_lit_send_face_sends_words` (client/spec/Views/Workspace.lean):
  // a press sends the words exactly when the lit send face is up.
  test("only the lit send face sends words", () => {
    expect(FACES.filter((face) => pressCoin(face) === "words")).toEqual(["send"]);
  });
});
