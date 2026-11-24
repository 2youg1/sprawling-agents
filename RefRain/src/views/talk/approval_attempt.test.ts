// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { expect, test } from "bun:test";
import { approve } from "../../core/commands";
import { get, writable } from "../../core/store";
import { ApprovalId, Locator, TimeMs, type ApprovalItem } from "../../wire";
import { keepAttempt, seenApprovals, stillOpen, type Attempt } from "./approval_attempt";

function question(id: string): ApprovalItem {
  return {
    id: ApprovalId.make(id),
    actor: "lab/review",
    action_desc: "review a local document",
    artifact: Locator.make(`cas:b3-${"ab".repeat(32)}`),
    cluster_key: { class: "question", detail: "document" },
    created: TimeMs.make(1),
    tainted: false,
  };
}

test("an uncertain grouped approval retries the unchanged request for the item still waiting", () => {
  const first = question("first");
  const second = question("second");
  const commands = [approve(first.id, "allow"), approve(second.id, "allow")].flatMap((command) =>
    "approve" in command ? [command] : [],
  );
  const attempt: Attempt = { kind: "unknown", commands, at: 0 };
  const state = writable<ReadonlyMap<string, Attempt>>(new Map());
  keepAttempt(state, "question:document", attempt);
  seenApprovals(state, [second]);

  const held = get(state).get("question:document");
  expect(held).toBe(attempt);
  expect(stillOpen(attempt, [second])).toEqual(commands.slice(1));

  seenApprovals(state, []);
  expect(get(state).size).toBe(0);
});
