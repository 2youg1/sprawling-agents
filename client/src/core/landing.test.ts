// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";
import type { RunBelief } from "./belief";
import { landingOf, sentFrom } from "./landing";
import { Address, RunId, Seq, TimeMs } from "../wire";

const SHOP = Address.make("shop");
const EARLIER = RunId.make("00000000-0000-4000-8000-000000000001");
const OURS = RunId.make("00000000-0000-4000-8000-000000000002");
const OTHER = RunId.make("00000000-0000-4000-8000-000000000003");
const TASK = "add a test to price";

function run(id: RunId, addr: string, task: string, at: number): RunBelief {
  return {
    run: id,
    addr: Address.make(addr),
    started: TimeMs.make(at),
    task,
    lastSeq: Seq.make(at),
    doing: { kind: "thinking" },
    model: null,
    local: false,
    saying: "",
    thinking: "",
  };
}

describe("where a dispatch landed", () => {
  const before = { [EARLIER]: run(EARLIER, "shop/older", TASK, 1) };
  const sent = sentFrom(SHOP, TASK, before);

  test("a run already known when the dispatch went out is not the one it opened", () => {
    expect(landingOf(sent, before)).toEqual({ kind: "pending" });
  });

  test.each(["shop/add-a-test-to-price", "shop/pricing/add-a-test-to-price"])(
    "a new run in %s, a room at any depth under the building, is followed there",
    (addr) => {
      const after = { ...before, [OURS]: run(OURS, addr, TASK, 5) };
      expect(landingOf(sent, after)).toEqual({
        kind: "elsewhere",
        run: OURS,
        addr: Address.make(addr),
      });
    },
  );

  test("a new run in the room itself stays here", () => {
    const after = { ...before, [OURS]: run(OURS, "shop", TASK, 5) };
    expect(landingOf(sent, after)).toEqual({ kind: "here" });
  });

  test("a run with other words or in another building is somebody else's", () => {
    const after = {
      ...before,
      [OTHER]: run(OTHER, "shopfront/room", TASK, 4),
      [OURS]: run(OURS, "shop/room", "another task", 5),
    };
    expect(landingOf(sent, after)).toEqual({ kind: "pending" });
  });

  test("of two new matching runs the first to start is the one sent", () => {
    const after = {
      ...before,
      [OTHER]: run(OTHER, "shop/second", TASK, 9),
      [OURS]: run(OURS, "shop/first", TASK, 6),
    };
    expect(landingOf(sent, after)).toEqual({
      kind: "elsewhere",
      run: OURS,
      addr: Address.make("shop/first"),
    });
  });
});
