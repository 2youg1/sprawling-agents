// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The attach form's fold, driven the way its seat drives it and with no
// look at all: whichever look draws the fold, every press leaves through
// the form's own setter, and a row of a key-value table keeps its boxes
// when another row goes.

import { describe, expect, test } from "bun:test";

import { lookOf } from "./advanced";
import type { Hands } from "./advanced";
import type { Draft, Line } from "./draft";

const line = (key: string, name: string, value: string): Line => ({ key, name, value });

const DRAFT: Draft = {
  id: { kind: "derived" },
  label: { kind: "derived" },
  baseUrl: "https://api.example.invalid/v1",
  wireApi: "chat",
  key: "",
  timeoutMs: "",
  requestRetries: "",
  streamIdleMs: "",
  proxying: "except_local",
  headers: [line("a", "x-team", "blue"), line("b", "x-tier", "gold")],
  overrides: [],
};

// The fold over one draft, with every write recorded.
function rig() {
  const writes: { part: keyof Draft; value: unknown }[] = [];
  let renamed = 0;
  const hands: Hands = {
    setDraft: (part, value) => {
      writes.push({ part, value });
    },
    onRenamed: () => {
      renamed += 1;
    },
  };
  return { writes, renamed: () => renamed, look: lookOf(DRAFT, undefined, "en", hands) };
}

const headers = (look: ReturnType<typeof rig>["look"]) => look.tables.find((each) => each.key === "headers");

describe("a key-value table", () => {
  test("typing into one row replaces that row alone, by its key", () => {
    const { writes, look } = rig();
    headers(look)?.rows[1]?.value.input("silver");
    expect(writes).toEqual([{ part: "headers", value: [line("a", "x-team", "blue"), line("b", "x-tier", "silver")] }]);
  });

  test("removing a row keeps the others with their keys, and adding opens a blank one", () => {
    const { writes, look } = rig();
    headers(look)?.rows[0]?.remove.press();
    headers(look)?.add.press();
    expect(writes[0]).toEqual({ part: "headers", value: [line("b", "x-tier", "gold")] });
    const added = writes[1]?.value;
    expect(Array.isArray(added) ? added.slice(0, 2) : added).toEqual(DRAFT.headers);
    expect(Array.isArray(added) ? added[2] : added).toMatchObject({ name: "", value: "" });
  });
});

describe("the naming boxes", () => {
  test("a rename writes the stated name and retires the last report", () => {
    const { writes, renamed, look } = rig();
    look.naming.find((box) => box.key === "label")?.input("House");
    expect(writes).toEqual([{ part: "label", value: { kind: "typed", value: "House" } }]);
    expect(renamed()).toBe(1);
  });

  test("an empty box shows the id the URL derives as its placeholder", () => {
    const { look } = rig();
    expect(look.naming.map((box) => [box.key, box.value, box.placeholder])).toEqual([
      ["id", "", "example"],
      ["label", "", "example"],
    ]);
  });
});
