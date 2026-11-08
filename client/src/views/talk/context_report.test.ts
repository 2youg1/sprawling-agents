// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";
import { fill, say } from "../../core/lang";
import type { PrefixAnswer, PromptShapeAnswer } from "../../wire";
import { B3Hash, RunId } from "../../wire";
import { reportOf, splitPercents } from "./context_report";

const RUN = RunId.make("00000000-0000-4000-8000-000000000001");
const HASH = {
  city: B3Hash.make("a".repeat(64)),
  building: B3Hash.make("b".repeat(64)),
  resident: B3Hash.make("c".repeat(64)),
  run: B3Hash.make("d".repeat(64)),
  tools: B3Hash.make("e".repeat(64)),
  conversation: B3Hash.make("f".repeat(64)),
} as const;

describe("splitPercents", () => {
  test("sums to 100 and prefers the largest remainder", () => {
    const shares = splitPercents([
      ["tools", 1],
      ["city", 1],
      ["building", 1],
      ["resident", 0],
      ["run", 0],
      ["conversation", 0],
    ]);
    expect(shares.reduce((sum, row) => sum + row.share, 0)).toBe(100);
    expect(shares.filter((row) => row.bytes > 0).map((row) => row.share).sort()).toEqual([
      33, 33, 34,
    ]);
  });

  test("is all zeros when every weight is zero", () => {
    expect(splitPercents([["tools", 0], ["conversation", 0]]).every((row) => row.share === 0)).toBe(
      true,
    );
  });
});

describe("reportOf", () => {
  const prefix: PrefixAnswer = {
    run: RUN,
    segments: [
      { slot: "city", hash: HASH.city, bytes: 1_000, text: "", stored: true, sources: [] },
      { slot: "building", hash: HASH.building, bytes: 2_000, text: "", stored: true, sources: [] },
      { slot: "resident", hash: HASH.resident, bytes: 500, text: "", stored: true, sources: [] },
      { slot: "run", hash: HASH.run, bytes: 100, text: "", stored: true, sources: [] },
    ],
    tools: {
      hash: HASH.tools,
      index: 0,
      len: 3_000,
      rows: [
        {
          name: "read",
          namespace: "core",
          exposure: "core",
          from: "default",
          hint: "",
          len: 2_000,
        },
        {
          name: "search",
          namespace: "core",
          exposure: "core",
          from: "default",
          hint: "",
          len: 1_000,
        },
        {
          name: "sleeping",
          namespace: "core",
          exposure: "dormant",
          from: "default",
          hint: "",
          len: 400,
        },
        {
          name: "offed",
          namespace: "core",
          exposure: "off",
          from: "default",
          hint: "",
          len: 9_999,
        },
      ],
    },
  };

  const shape: PromptShapeAnswer = {
    run: RUN,
    tools: { bytes: 3_000, hash: HASH.tools },
    conversation: { bytes: 20_000, hash: HASH.conversation },
    upper_bound: 26_600,
  };

  test("ranks conversation first and hides empty noise", () => {
    const look = reportOf("en", RUN, prefix, shape, {
      used: 211_000,
      window: 256_000,
      first: 30,
      second: 65,
    });
    expect(look.buckets[0]?.id).toBe("conversation");
    expect(look.buckets.every((bucket) => bucket.bytes > 0)).toBe(true);
    expect(look.insight).toBe(
      fill(say("en", "context_report_insight"), { bucket: say("en", "context_bucket_conversation"), share: String(look.buckets[0]?.share) }),
    );
    expect(look.buckets.find((bucket) => bucket.id === "tools")?.children.map((c) => c.name)).toEqual([
      "core/read",
      "core/search",
      "core/sleeping",
    ]);
  });

  test("warns past the handoff mark", () => {
    const look = reportOf("en", RUN, prefix, shape, {
      used: 200_000,
      window: 256_000,
      first: 30,
      second: 65,
    });
    expect(look.warning).toContain("65");
  });

  test("warns when used tokens land exactly on the handoff percent", () => {
    const look = reportOf("en", RUN, prefix, shape, {
      used: 166_400,
      window: 256_000,
      first: 30,
      second: 65,
    });
    expect(look.warning).toContain("65");
  });

  test("does not trip the handoff when only the rounded display reaches it", () => {
    // 64.6% rounds to 65 for the header figure, but the raw share is still
    // below the 65% handoff — only the earlier filling line may sound.
    const look = reportOf("en", RUN, prefix, shape, {
      used: 165_376,
      window: 256_000,
      first: 30,
      second: 65,
    });
    expect(look.fullPercent).toBe(65);
    expect(look.warning).toContain("30");
    expect(look.warning).not.toContain("65");
  });

  test("keeps a non-empty bucket that rounded to zero percent", () => {
    const tiny: PrefixAnswer = {
      ...prefix,
      segments: [
        { slot: "city", hash: HASH.city, bytes: 100, text: "", stored: true, sources: [] },
        { slot: "building", hash: HASH.building, bytes: 0, text: "", stored: true, sources: [] },
        { slot: "resident", hash: HASH.resident, bytes: 0, text: "", stored: true, sources: [] },
        { slot: "run", hash: HASH.run, bytes: 0, text: "", stored: true, sources: [] },
      ],
      tools: null,
    };
    const heavy: PromptShapeAnswer = {
      run: RUN,
      tools: { bytes: 0, hash: HASH.tools },
      conversation: { bytes: 100_000, hash: HASH.conversation },
      upper_bound: 100_100,
    };
    const look = reportOf("en", RUN, tiny, heavy, null);
    const city = look.buckets.find((bucket) => bucket.id === "city");
    expect(city?.bytes).toBe(100);
    expect(city?.share).toBe(0);
  });

  test("waits honestly when nothing has assembled", () => {
    const look = reportOf("en", RUN, null, null, null);
    expect(look.buckets).toEqual([]);
    expect(look.insight).toBe(say("en", "context_report_empty"));
  });
});
