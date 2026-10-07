// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The count form of a badge (client/Spec.lean §4-32): the number a key
// carries in its corner, which must never grow wider than the key and
// never be read twice by a screen reader that already heard the key's
// name.

import { describe, expect, test } from "bun:test";

import { badgeOf, CAP } from "./badge";

describe("a count badge", () => {
  test("draws nothing at zero, the number up to its cap, and cap+ past it", () => {
    expect(badgeOf({ count: 0 })).toBeUndefined();
    expect(badgeOf({ count: 7 })).toEqual({ form: "count", text: "7", tier: "alert", quiet: { "aria-hidden": "true" } });
    expect(badgeOf({ count: CAP + 1 })).toMatchObject({ text: `${String(CAP)}+` });
    expect(badgeOf({ count: 10, cap: 9, weight: "live" })).toMatchObject({ text: "9+", tier: "live" });
  });

  test("something new without a number is a lone dot, hidden like the number", () => {
    expect(badgeOf({ count: "fresh", weight: "quiet" })).toEqual({
      form: "count",
      text: undefined,
      tier: "quiet",
      quiet: { "aria-hidden": "true" },
    });
  });
});

describe("a word badge", () => {
  test("a status takes its drawing and its tier from the status table, a tier its caller's", () => {
    expect(badgeOf({ text: "refused", status: "refused" })).toEqual({ form: "word", text: "refused", mark: "cross", tier: "alert" });
    expect(badgeOf({ text: "live", weight: "live", dot: true })).toEqual({ form: "word", text: "live", mark: "dot", tier: "live" });
    expect(badgeOf({ text: "local" })).toEqual({ form: "word", text: "local", mark: undefined, tier: "quiet" });
  });
});
