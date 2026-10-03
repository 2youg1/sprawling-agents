// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { afterEach, describe, expect, test } from "bun:test";

import { forget, markEnd, markEndAtFrame, markStart, measured, timeAtRoot } from "./timing";

describe("timing", () => {
  afterEach(() => {
    forget("session_switch");
    forget("layer_drawn");
    forget("keystroke_echo");
  });

  test("a start and an end make one measure; an end with no start makes none", () => {
    markEnd("session_switch");
    markStart("session_switch");
    markEnd("session_switch");
    markEnd("session_switch");
    const durations = measured("session_switch");
    expect(durations.length).toBe(1);
    expect(durations.every((duration) => duration >= 0)).toBe(true);
    expect(measured("layer_drawn")).toEqual([]);
  });

  test("an end at the frame records the measure once the frame comes", async () => {
    markStart("layer_drawn");
    markEndAtFrame("layer_drawn");
    await new Promise((settled) => setTimeout(settled, 50));
    expect(measured("layer_drawn").length).toBe(1);
  });

  test("forgetting drops what was recorded", () => {
    markStart("session_switch");
    markEnd("session_switch");
    forget("session_switch");
    expect(measured("session_switch")).toEqual([]);
  });

  test("a keystroke heard at the root is measured from the event's own time stamp", async () => {
    const root = new EventTarget();
    const stop = timeAtRoot(root);
    root.dispatchEvent(new Event("input"));
    stop();
    root.dispatchEvent(new Event("input"));
    await new Promise((settled) => setTimeout(settled, 50));
    expect(measured("keystroke_echo").length).toBe(1);
  });
});
