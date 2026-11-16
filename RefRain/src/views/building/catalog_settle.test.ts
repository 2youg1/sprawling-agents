// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { describe, expect, test } from "bun:test";

import type { CatalogAnswer, Exposure } from "../../wire";
import { Address, B3Hash } from "../../wire";
import { settleSave } from "./catalog";
import type { Row, Selection } from "./catalog";
import { catalogLook } from "./catalog_look";

function row(
  name: number,
  ns: number,
  bytes: number,
  fixed: boolean,
  inherited: Exposure,
  nearer?: Exposure,
): Row {
  return {
    tool: {
      name: `t${String(name)}`,
      namespace: `n${String(ns)}`,
      current: {
        bytes,
        hint: "",
        description: "",
        digest: B3Hash.make(name.toString(16).padStart(64, "0")),
        input_schema: {},
      },
      pinned: null,
      change: null,
      exposure: inherited,
      from: fixed ? "fixed" : "default",
      fallback: inherited,
      runs_used: 0,
      last_used: null,
    },
    fixed,
    inherited,
    nearer,
  };
}

function selection(
  reading: readonly string[],
  skills: Readonly<Record<string, Exposure>> = {},
): Selection {
  return {
    catalog: { tools: {}, namespaces: {}, skills },
    references: [],
    skillPins: {},
    readingRoom: reading,
  };
}

function selectionOfStanding(): Selection {
  return {
    catalog: { tools: {}, namespaces: {}, skills: {} },
    references: [],
    skillPins: {},
    readingRoom: [],
  };
}

describe("a building with no run yet", () => {
  const hands = {
    scope: () => undefined,
    pane: () => undefined,
    edit: () => undefined,
    save: () => undefined,
    unused: () => undefined,
    copy: () => undefined,
  };
  const answer: CatalogAnswer = {
    building: Address.make("lab"),
    run: null,
    core_bytes: 0,
    index_bytes: 0,
    reading_bytes: 0,
    always: 1,
    dormant: 1,
    skills: 0,
    changed: 0,
    open_sessions: 0,
    runs_seen: 0,
    tools: [row(0, 0, 10, false, "core").tool, row(1, 0, 8, false, "dormant").tool],
    skill_rows: [],
    references: [],
    skill_pins: {},
    reading_room: [],
    city: {},
    stated: {},
  };

  test("lists the standing tools and says a save reaches the first run", () => {
    const look = catalogLook(
      {
        answer,
        scope: "building",
        draft: selectionOfStanding(),
        edited: false,
        now: 0,
        unusedOnly: false,
        buildings: [],
        pane: "tools",
      },
      "en",
      hands,
    );
    expect(look.empty).toBeUndefined();
    expect(look.beforeRun).toContain("No run yet");
    expect(look.groups.length).toBeGreaterThan(0);
  });

  test("an empty tools pane still says there is nothing to edit", () => {
    const look = catalogLook(
      {
        answer: { ...answer, tools: [] },
        scope: "building",
        draft: selectionOfStanding(),
        edited: false,
        now: 0,
        unusedOnly: false,
        buildings: [],
        pane: "tools",
      },
      "en",
      hands,
    );
    expect(look.beforeRun).toBeUndefined();
    expect(look.empty).toContain("No run yet");
  });
});

describe("settleSave", () => {
  test("a landed save acknowledges only a draft the person has not moved on from", () => {
    const sent = selection([]);
    expect(settleSave(sent, sent, sent, "building")).toEqual({
      landed: true,
      edited: false,
      saved: true,
    });
    const edited = selection(["guide"]);
    expect(settleSave(sent, edited, sent, "building")).toEqual({
      landed: true,
      edited: true,
      saved: false,
    });
    const earlier = selection(["other"]);
    expect(settleSave(earlier, sent, sent, "building").landed).toBe(false);
  });

  test("the city layer settles on the catalog table alone", () => {
    const sent = selection([]);
    const held = { ...sent, readingRoom: ["guide"] };
    expect(settleSave(held, sent, sent, "city").landed).toBe(true);
    const skills: Readonly<Record<string, Exposure>> = { guide: "off" };
    const edited = { ...sent, catalog: { tools: {}, namespaces: {}, skills } };
    expect(settleSave(sent, edited, sent, "city")).toEqual({
      landed: true,
      edited: true,
      saved: false,
    });
  });
});
