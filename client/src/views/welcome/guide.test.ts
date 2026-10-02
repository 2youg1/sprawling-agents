// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import type { GuideProgress } from "../../wire";
import type { Configured } from "./guide";
import { advanced, currentOf, opened, opensGuide, putOff, putOffTheRest, skipAll, standingOf } from "./guide";

const NOTHING: Configured = { provider: false, dependencies: false, texts: false, skills: null, mcp: null };

describe("the guide's standing", () => {
  test("a step put off and then done elsewhere is done, not skipped", () => {
    const progress: GuideProgress = { dependencies: "skipped" };
    expect(standingOf("dependencies", progress, { ...NOTHING, dependencies: true })).toBe("configured");
  });

  test("opening a step is not doing it", () => {
    const progress = opened({}, "texts");
    expect(progress).toEqual({ at: "texts", texts: "seen" });
    expect(standingOf("texts", progress, NOTHING)).toBe("seen");
  });

  test("looking again at a step put off leaves it put off", () => {
    expect(opened({ skills: "skipped" }, "skills")).toEqual({ at: "skills", skills: "skipped" });
  });

  test("the first step has no mark, only the city's answer", () => {
    expect(opened({}, "provider")).toEqual({ at: "provider" });
    expect(standingOf("provider", {}, NOTHING)).toBe("required");
  });

  test("putting a step off moves the guide to the next one", () => {
    expect(putOff({ at: "mcp" }, "mcp")).toEqual({ at: "mcp", mcp: "skipped" });
    expect(putOff({}, "dependencies")).toEqual({ at: "texts", dependencies: "skipped" });
  });

  test("putting off the rest marks only the steps nobody looked at", () => {
    expect(putOffTheRest({ texts: "seen" })).toEqual({
      texts: "seen",
      dependencies: "skipped",
      skills: "skipped",
      mcp: "skipped",
    });
  });

  test("a fresh guide opens on the first step the city has not done", () => {
    expect(currentOf({}, { ...NOTHING, provider: true })).toBe("dependencies");
    expect(currentOf({ dependencies: "skipped" }, { ...NOTHING, provider: true })).toBe("texts");
    expect(currentOf({ at: "mcp" }, NOTHING)).toBe("mcp");
  });

  test("skipping every optional step leaves the guide", () => {
    expect(skipAll({ texts: "seen" })).toEqual({
      texts: "seen",
      dependencies: "skipped",
      skills: "skipped",
      mcp: "skipped",
      state: "left",
    });
  });

  test("a step done while open opens the next step nobody did or put off", () => {
    const before = { ...NOTHING, dependencies: true };
    const after = { ...before, provider: true };
    expect(advanced({ at: "provider", texts: "skipped" }, "provider", before, after)).toEqual({
      at: "skills",
      texts: "skipped",
      skills: "seen",
    });
  });

  test("an answer that only now arrived does not move the guide", () => {
    const unknown = { ...NOTHING, provider: null };
    expect(advanced({ at: "provider" }, "provider", unknown, { ...NOTHING, provider: true })).toBeNull();
    expect(advanced({ at: "provider" }, "provider", NOTHING, NOTHING)).toBeNull();
  });
});

describe("the launch", () => {
  test("opens the guide whenever the city has no provider endpoint", () => {
    for (const main of [false, true])
      for (const welcomed of [false, true]) expect(opensGuide({ endpoints: 0, main, welcomed })).toBe(true);
  });

  test("with an endpoint, opens it only where nobody walked it and no main model is chosen", () => {
    expect(opensGuide({ endpoints: 1, main: false, welcomed: false })).toBe(true);
    expect(opensGuide({ endpoints: 1, main: false, welcomed: true })).toBe(false);
    expect(opensGuide({ endpoints: 2, main: true, welcomed: false })).toBe(false);
  });
});
