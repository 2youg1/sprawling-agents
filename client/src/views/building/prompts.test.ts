// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// Which steps each prompt document's card offers, and where it opens:
// the city decides what is sent, and the page decides only which of the
// three whole-document steps a card may send.

import { describe, expect, test } from "bun:test";

import { Address, B3Hash } from "../../wire";
import type { LayerDocument } from "../../wire";
import { actionsOf, pathOf } from "./prompts";

const SHOP = Address.make("shop");
const VERSION = B3Hash.make("af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262");

function doc(at: string, part: Pick<LayerDocument, "kind" | "layer" | "state"> & Partial<LayerDocument>): LayerDocument {
  return { at: Address.make(at), version: VERSION, slot: "city", shipped: null, ...part };
}

describe("a prompt card's steps", () => {
  test("the city's City.md opens and restores, and restores nothing once it is the default", () => {
    const edited = doc("City.md", { kind: "city", layer: "city", state: { sent: { bytes: 5 } }, shipped: { version: VERSION, standing: "edited" } });
    const current = doc("City.md", { kind: "city", layer: "city", state: { sent: { bytes: 5 } }, shipped: { version: VERSION, standing: "current" } });
    expect([actionsOf(edited, SHOP), actionsOf(current, SHOP)]).toEqual([
      [
        { kind: "open", path: "/City.md" },
        { kind: "step", step: "restore" },
      ],
      [{ kind: "open", path: "/City.md" }],
    ]);
  });

  test("a building's own City.md is started where it is absent and removed where it is there", () => {
    const absent = doc("shop/.sprawling/City.md", { kind: "city", layer: "building", state: "absent" });
    const there = doc("shop/.sprawling/City.md", { kind: "city", layer: "building", state: { sent: { bytes: 9 } } });
    expect([actionsOf(absent, SHOP), actionsOf(there, SHOP)]).toEqual([
      [{ kind: "step", step: "own" }],
      [
        { kind: "open", path: ".sprawling/City.md" },
        { kind: "step", step: "inherit" },
      ],
    ]);
  });

  test("a project's conventions and a withheld one only open, and a withheld one does not", () => {
    const sent = doc("shop/AGENTS.md", { kind: "conventions", layer: "building", state: { sent: { bytes: 3 } } });
    const withheld = doc("shop/AGENTS.md", { kind: "conventions", layer: "building", state: { withheld: { why: "a directory" } } });
    expect([actionsOf(sent, SHOP), actionsOf(withheld, SHOP)]).toEqual([[{ kind: "open", path: "AGENTS.md" }], []]);
  });

  test("a document outside the building opens by its path from the city root", () => {
    expect([pathOf(Address.make(".sprawling/MAYOR.md"), SHOP), pathOf(Address.make("shop/x/AGENTS.md"), SHOP)]).toEqual([
      "/.sprawling/MAYOR.md",
      "x/AGENTS.md",
    ]);
  });
});
