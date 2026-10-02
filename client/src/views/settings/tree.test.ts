// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { SETUP_GROUPS } from "../../core/route";
import { Address } from "../../wire";
import { TREE, buildingPages, standingOf } from "./tree";

// The address bar may name any group (`core/route.ts`), and the tree is
// the only way to one by hand, so a group the tree leaves out is a
// group nobody reaches, and one it offers twice is two places that
// claim to be it.
describe("the settings tree", () => {
  test("offers every group the address bar reads, each once, folded or not", () => {
    const offered = TREE.flatMap((branch) =>
      [...branch.entries, ...branch.more].flatMap((entry) => (entry.kind === "group" ? [entry.group] : [])),
    );
    expect([...offered].sort()).toEqual([...SETUP_GROUPS].sort());
  });

  test("opens on the branch of the group drawn, with its more fold open when the group is inside it", () => {
    const talk = { kind: "talk", address: Address.make("hall/mayor") } as const;
    TREE.forEach((branch, at) => {
      branch.entries.forEach((entry) => {
        if (entry.kind === "group") expect(standingOf(entry.group, talk)).toEqual({ branch: at, more: false });
      });
      branch.more.forEach((entry) => {
        if (entry.kind === "group") expect(standingOf(entry.group, talk)).toEqual({ branch: at, more: true });
      });
    });
  });

  test("opens on the branch of the page beneath when the tree offers that page", () => {
    const at = TREE.findIndex((branch) => branch.entries.some((entry) => entry.kind === "nest" && entry.nest === "buildings"));
    expect(standingOf("you", { kind: "building", address: Address.make("shop") })).toEqual({ branch: at, more: false });
    const record = TREE.findIndex((branch) => branch.entries.some((entry) => entry.kind === "nest" && entry.nest === "record"));
    expect(standingOf("you", { kind: "record", lens: "archive" })).toEqual({ branch: record, more: false });
    const cost = TREE.findIndex((branch) => branch.more.some((entry) => entry.kind === "page" && entry.view.kind === "cost"));
    expect(standingOf("you", { kind: "cost" })).toEqual({ branch: cost, more: true });
  });

  test("lists a city of one building, the hall", () => {
    expect(buildingPages([Address.make("hall")]).map((page) => page.view)).toEqual([
      { kind: "building", address: Address.make("hall") },
    ]);
  });
});
