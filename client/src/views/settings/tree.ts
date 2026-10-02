// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The settings tree: six branches grouped by what a setting is about,
// one open at a time, each with a "more" fold for its second-rank
// entries, and the two entries that open a level of their own
// (client/Spec.lean §7L, D52, D53). An entry is either a group the panel
// draws in its body, or a page the address bar moves to; the tree is
// the one way every page but the conversation is reached (4-8, client
// D19), so this table is also the list of those pages.
//
// The branches answer one question each: the city's own facts, what
// the city can reach, what a run is allowed, what the city has taken
// in, what belongs to this person and this browser alone, and where to
// look when something went wrong. An entry whose page does not exist
// yet is not in the table, because a row that goes nowhere is a dead
// button.

import type { Key } from "../../core/lang";
import type { SetupGroup, View } from "../../core/route";
import type { Address } from "../../wire";

// A page and the word the tree names it by.
export interface Page {
  readonly kind: "page";
  readonly view: View;
  readonly word: Key;
}

// The two entries with entries of their own: one per building, and one
// per lens of the record. Their leaves are read when they are drawn,
// because the buildings are the city's answer.
export type Nest = "buildings" | "record";

// What a "more" fold may hold: never a nest, so a branch, its "more"
// and the entry inside it are the three levels the tree is allowed.
export type Leaf = { readonly kind: "group"; readonly group: SetupGroup } | Page;

export type Entry = Leaf | { readonly kind: "nest"; readonly nest: Nest; readonly word: Key };

export interface Branch {
  readonly word: Key;
  readonly entries: readonly Entry[];
  readonly more: readonly Leaf[];
}

const group = (named: SetupGroup): Leaf => ({ kind: "group", group: named });

export const TREE: readonly Branch[] = [
  {
    word: "settings_branch_city",
    entries: [
      group("you"),
      { kind: "page", view: { kind: "city" }, word: "settings_city_overview" },
      { kind: "nest", nest: "buildings", word: "settings_buildings" },
    ],
    more: [
      { kind: "page", view: { kind: "registry" }, word: "nav_registry" },
      { kind: "page", view: { kind: "cost" }, word: "cost_title" },
    ],
  },
  {
    word: "settings_branch_access",
    entries: [group("accounts"), group("harnesses")],
    more: [group("network"), group("remote")],
  },
  {
    word: "settings_branch_running",
    entries: [group("run"), group("rules")],
    more: [group("automation")],
  },
  {
    word: "settings_branch_extensions",
    entries: [group("skills"), { kind: "page", view: { kind: "mcp" }, word: "nav_mcp" }],
    more: [],
  },
  {
    word: "settings_branch_preferences",
    entries: [group("appearance"), group("keys")],
    more: [],
  },
  {
    word: "settings_branch_diagnosis",
    entries: [
      { kind: "nest", nest: "record", word: "nav_the_record" },
      { kind: "page", view: { kind: "monitor" }, word: "monitor_title" },
      group("tools"),
    ],
    // The guide stays reachable after the person has left it
    // (refrain 3-15): its steps are where a skipped setting is made.
    more: [group("about"), { kind: "page", view: { kind: "welcome" }, word: "welcome_title" }, group("advanced")],
  },
];

// Where the panel stands: the branch drawn open, and whether that
// branch's "more" fold is open with it.
export interface Standing {
  readonly branch: number;
  readonly more: boolean;
}

// The branch the panel opens on: the one holding the page beneath when
// the tree offers that page, else the one holding the group drawn.
export function standingOf(drawn: SetupGroup, beneath: View): Standing {
  return { branch: 0, more: false };
}

// One page per building the city names, the hall among them, so a city
// of one building still lists it.
export function buildingPages(addresses: readonly Address[]): readonly Page[] {
  return addresses
    .filter((address) => address !== "hall")
    .map((address) => ({ kind: "page", view: { kind: "building", address }, word: "settings_buildings" }));
}

// The group a panel opened without one shows: the first the tree
// offers, which is who the person and the Mayor are (client/Spec.lean §7L).
export const FIRST_GROUP: SetupGroup = "you";

// The nest the page beneath the panel belongs to, which starts open so
// the entry that names that page is in sight.
export function nestOf(view: View): Nest | null {
  switch (view.kind) {
    case "building":
      return "buildings";
    case "record":
      return "record";
    case "talk":
    case "city":
    case "run":
    case "setup":
    case "mcp":
    case "cost":
    case "registry":
    case "welcome":
    case "monitor":
    case "gallery":
      return null;
  }
}
