// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { ApprovalId, type ApprovalItem, Locator, TimeMs } from "../wire";
import { notices, UNHEARD, WARMUP_MS, type Heard, type Scene } from "./notify";

function item(id: string, actor: string): ApprovalItem {
  return {
    action_desc: `run ${id}`,
    actor,
    artifact: Locator.make("cas:b3-00"),
    cluster_key: { class: "question", detail: "" },
    created: TimeMs.make(0),
    id: ApprovalId.make(id),
    tainted: false,
  };
}

const AWAY: Scene = { notifying: "on", focus: "blurred", elapsed: WARMUP_MS, watching: null };
const OLD = item("a", "lab/room1");
const NEW = item("b", "lab/room2");

// The account after the first answer, which held only `OLD`.
function snapshot(): Heard {
  return notices(UNHEARD, [OLD], AWAY)[0];
}

describe("notices", () => {
  test("raises only what arrived after the snapshot", () => {
    expect(notices(UNHEARD, [OLD], AWAY)[1]).toEqual([]);
    expect(notices(snapshot(), [OLD, NEW], AWAY)[1]).toEqual([NEW]);
  });

  test("each gate holds a new item back, and a held item is never raised later", () => {
    const gates: readonly Scene[] = [
      { ...AWAY, notifying: "off" },
      { ...AWAY, focus: "focused" },
      { ...AWAY, elapsed: WARMUP_MS - 1 },
      { ...AWAY, watching: "lab/room2" },
    ];
    for (const scene of gates) {
      const [heard, raised] = notices(snapshot(), [OLD, NEW], scene);
      expect(raised).toEqual([]);
      expect(notices(heard, [OLD, NEW], AWAY)[1]).toEqual([]);
    }
  });
});
