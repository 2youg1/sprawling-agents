// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The rows the thread's notes hand their look: each ending of a wait
// and of a handback in its own ink, and a name that leads only where
// there is somewhere to lead (client D86).

import { describe, expect, test } from "bun:test";

import { handbackLook, named, replyWaitLook } from "./note";
import { TimeMs } from "../../wire";

const ROOM = named("tests", "#/talk/shop/tests");
const AT = TimeMs.make(1);

describe("a note's row", () => {
  test("a name without a place to go stands in the quieter ink", () => {
    expect([named("tests", null), ROOM]).toEqual([
      { kind: "words", text: "tests", ink: "quiet" },
      { kind: "link", text: "tests", href: "#/talk/shop/tests", face: "text" },
    ]);
  });

  test("an open wait says the time left, and once it is due says so in the alert ink", () => {
    expect(replyWaitLook("en", ROOM, 90_000, null)).toEqual({
      role: undefined,
      pieces: [{ kind: "words", text: "waiting for a reply from", ink: "faint" }, ROOM, { kind: "words", text: "· 1m 30s left", ink: "quiet" }],
    });
    expect(replyWaitLook("en", ROOM, 0, undefined).pieces[2]).toEqual({ kind: "words", text: "· past the deadline", ink: "alert" });
  });

  test("each of the three endings of a wait is drawn in its own ink", () => {
    expect(
      (["reply", "timeout", "left"] as const).map((by) => replyWaitLook("en", ROOM, -1, { by, t: AT }).pieces[2]),
    ).toEqual([
      { kind: "words", text: "· replied", ink: "accent" },
      { kind: "words", text: "· no reply before the deadline", ink: "alert" },
      { kind: "words", text: "· left without replying", ink: "faint" },
    ]);
  });

  test("a handback says how it ended: finished in the accent ink, stopped in the alert ink", () => {
    expect(handbackLook("en", ROOM, { finished: { verified_by: "cargo nextest" } }, "12:00")).toEqual({
      role: "note",
      pieces: [
        ROOM,
        { kind: "words", text: "handed back:", ink: "faint" },
        { kind: "words", text: "finished (verified by cargo nextest)", ink: "accent" },
        { kind: "words", text: "· 12:00", ink: "faint" },
      ],
    });
    expect(handbackLook("en", ROOM, { stopped: { because: "refused" } }, "12:00").pieces[2]).toEqual({
      kind: "words",
      text: "stopped, because refused",
      ink: "alert",
    });
  });
});
