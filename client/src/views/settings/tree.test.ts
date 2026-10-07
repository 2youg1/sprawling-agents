// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import { SETUP_GROUPS, type SetupGroup, type View } from "../../core/route";
import { Address } from "../../wire";
import { foldsOf, toggled, type Fold, type Folds } from "./folds";
import { TREE, buildingPages, byInitial, lookOf, standingOf, stepped, type LeafLook, type TreeLook } from "./tree";

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

// The look is handed every role, state and handler of client/Spec.lean
// §7-11 in its wire bags, so these hold for whatever look draws the tree.
describe("the settings tree's wiring", () => {
  const talk = { kind: "talk", address: Address.make("hall/mayor") } as const;
  const leaves = { buildings: buildingPages([Address.make("hall")]), record: [] };

  function drawn(group: SetupGroup, beneath: View, folds: Folds = foldsOf(group, beneath)) {
    const picked: SetupGroup[] = [];
    const pressed: Fold[] = [];
    const look = lookOf({ group, beneath, folds, leaves, reading: "3% · 120 MB" }, "t", {
      words: (key) => key,
      pick: (named) => picked.push(named),
      toggle: (fold) => pressed.push(fold),
      walk: () => undefined,
      hold: () => undefined,
    });
    return { look, picked, pressed };
  }

  const leavesOf = (look: TreeLook): LeafLook[] =>
    look.branches.flatMap((branch) => [
      ...branch.entries.flatMap((entry): readonly LeafLook[] => (entry.kind === "nest" ? entry.pages : [entry])),
      ...(branch.more?.entries ?? []),
    ]);

  test("names the group drawn, and only it, as the current one", () => {
    const { look } = drawn("rules", talk);
    const current = leavesOf(look).filter((leaf) => leaf.wire["aria-current"] !== undefined);
    expect(current.map((leaf) => [leaf.key, leaf.wire["aria-current"]])).toEqual([["rules", "true"]]);
  });

  test("names the page beneath as the current page", () => {
    const { look } = drawn("you", { kind: "monitor" });
    const current = leavesOf(look).filter((leaf) => leaf.kind === "page" && leaf.wire["aria-current"] === "page");
    expect(current.map((leaf) => leaf.wire)).toEqual([{ "data-entry": "", href: "#/monitor", "aria-current": "page" }]);
  });

  test("opens one branch, whose button says so and names the list it opens", () => {
    const { look } = drawn("automation", talk);
    const open = look.branches.filter((branch) => branch.fold.wire["aria-expanded"]);
    expect(open).toHaveLength(1);
    const [branch] = open;
    expect(branch?.fold.wire["aria-controls"]).toBe(branch?.fold.list);
    expect(branch?.more?.fold.wire["aria-expanded"]).toBe(true);
  });

  test("hands each press to the seat: a group to pick, a fold to toggle", () => {
    const { look, picked, pressed } = drawn("you", talk);
    const group = leavesOf(look).find((leaf) => leaf.kind === "group" && leaf.key === "performance");
    if (group?.kind === "group") group.wire.onclick();
    look.branches[2]?.fold.wire.onclick();
    expect(picked).toEqual(["performance"]);
    expect(pressed).toEqual([{ kind: "branch", at: 2 }]);
  });

  test("marks every entry the keys walk, and gives a group the letter it is reached by", () => {
    const { look } = drawn("you", talk);
    for (const leaf of leavesOf(look)) {
      expect(leaf.wire["data-entry"]).toBe("");
      if (leaf.kind === "group") expect(leaf.wire["data-initial"]).toBe(leaf.initial);
    }
    expect(look.branches.map((branch) => branch.fold.wire["data-entry"])).toEqual(look.branches.map(() => ""));
  });

  test("puts the process reading on the performance page's entry alone", () => {
    const { look } = drawn("you", talk);
    const read = leavesOf(look).flatMap((leaf) => (leaf.kind === "page" && leaf.reading !== null ? [leaf.key] : []));
    expect(read).toEqual(["#/monitor"]);
  });
});

describe("the settings tree's folds", () => {
  const folds: Folds = { branch: 1, more: true, nests: { buildings: false, record: true } };

  test("opening another branch folds the open one and its more", () => {
    expect(toggled(folds, { kind: "branch", at: 3 })).toEqual({ ...folds, branch: 3, more: false });
  });

  test("pressing the open branch folds it", () => {
    expect(toggled(folds, { kind: "branch", at: 1 })).toEqual({ ...folds, branch: null, more: false });
  });

  test("a nest and the more fold open and close on their own", () => {
    expect(toggled(folds, { kind: "nest", nest: "buildings" })).toEqual({ ...folds, nests: { buildings: true, record: true } });
    expect(toggled(folds, { kind: "more" })).toEqual({ ...folds, more: false });
  });
});

describe("the settings tree's keys", () => {
  const entries = ["a", "b", "c", "d"];

  test("step to the next, the previous and the ends, from outside the entries too", () => {
    expect([stepped(entries, -1, "line.next"), stepped(entries, 3, "line.next"), stepped(entries, 0, "line.previous")]).toEqual([
      "a",
      "d",
      "a",
    ]);
    expect([stepped(entries, 1, "line.first"), stepped(entries, 1, "line.last"), stepped(entries, 1, null)]).toEqual([
      "a",
      "d",
      undefined,
    ]);
  });

  test("a letter moves to the next entry it starts, wrapping past the last", () => {
    const initial = (entry: string): string => (entry === "b" || entry === "d" ? "x" : "y");
    expect([byInitial(entries, 1, "x", initial), byInitial(entries, 3, "x", initial), byInitial(entries, 0, "z", initial)]).toEqual([
      "d",
      "b",
      undefined,
    ]);
  });
});
