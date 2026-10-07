// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// One answer covers one question: identical questions share a card, a
// tainted one never does, and the cards stand in the order their first
// question arrived.

import { expect, test } from "bun:test";

import { grouped } from "./waiting";
import { ApprovalId, Locator, TimeMs } from "../../wire";
import type { ApprovalItem } from "../../wire";

function item(id: number, created: number, detail: string, tainted: boolean): ApprovalItem {
  return {
    action_desc: `question ${String(id)}`,
    actor: "shop/main",
    artifact: Locator.make("shop/src/lib.rs"),
    cluster_key: { class: "question", detail },
    created: TimeMs.make(created),
    id: ApprovalId.make(String(id)),
    tainted,
  };
}

test("identical questions share a card, a tainted one stands alone, in the order they arrived", () => {
  const items = [item(1, 30, "lib.rs", false), item(2, 10, "lib.rs", false), item(3, 20, "lib.rs", true), item(4, 40, "main.rs", false)];
  expect(grouped(items).map((group) => [group.key, group.items.map((each) => each.action_desc)])).toEqual([
    ["question:lib.rs", ["question 2", "question 1"]],
    ["item:3", ["question 3"]],
    ["question:main.rs", ["question 4"]],
  ]);
});
