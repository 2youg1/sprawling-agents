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

import { createAttachmentKey, type Attachment } from "svelte/attachments";

import type { Key } from "../../core/lang";
import { initialOf, type LineMove } from "../../core/lines";
import { toFragment, type SetupGroup, type View } from "../../core/route";
import type { Address } from "../../wire";
import { HEADING } from "../setup/groups";
import type { Fold, Folds } from "./folds";

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
    entries: [group("run"), group("rules"), group("performance")],
    more: [group("automation")],
  },
  {
    word: "settings_branch_extensions",
    entries: [group("skills"), { kind: "page", view: { kind: "mcp" }, word: "nav_mcp" }],
    more: [],
  },
  {
    word: "settings_branch_preferences",
    entries: [group("appearance"), group("colours"), group("keys")],
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
    more: [group("about"), { kind: "page", view: { kind: "welcome" }, word: "welcome_title" }, group("advanced"), group("privacy")],
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
  return placeOf((leaf) => offers(leaf, beneath)) ?? placeOf((leaf) => leaf.kind === "group" && leaf.group === drawn) ?? FIRST;
}

const FIRST: Standing = { branch: 0, more: false };

function placeOf(holds: (entry: Entry) => boolean): Standing | null {
  const branch = TREE.findIndex((each) => each.entries.some(holds) || each.more.some(holds));
  const found = TREE[branch];
  return found === undefined ? null : { branch, more: !found.entries.some(holds) };
}

// Whether an entry of the tree is the way to the page beneath. A nest
// offers every page of its kind; the conversation is offered by none.
function offers(entry: Entry, beneath: View): boolean {
  switch (entry.kind) {
    case "group":
      return false;
    case "page":
      return entry.view.kind === beneath.kind;
    case "nest":
      return entry.nest === nestOf(beneath);
  }
}

// One page per building the city names, the hall among them, so a city
// of one building still lists it.
export function buildingPages(addresses: readonly Address[]): readonly Page[] {
  return addresses.map((address) => ({ kind: "page", view: { kind: "building", address }, word: "settings_buildings" }));
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

// The entry a line key moves the focus to from the entry at `at`; -1
// is a focus outside the entries.
export function stepped<T>(entries: readonly T[], at: number, move: LineMove | null): T | undefined {
  switch (move) {
    case "line.next":
      return entries[Math.min(at + 1, entries.length - 1)];
    case "line.previous":
      return entries[Math.max(at - 1, 0)];
    case "line.first":
      return entries[0];
    case "line.last":
      return entries.at(-1);
    case "line.open":
    case "line.close":
    case null:
      return undefined;
  }
}

// The next entry after the focus whose letter is `letter`, from the top
// again past the last one (client D40).
export function byInitial<T>(
  entries: readonly T[],
  at: number,
  letter: string,
  initial: (entry: T) => string | undefined,
): T | undefined {
  return [...entries.slice(at + 1), ...entries.slice(0, at + 1)].find((each) => initial(each) === letter);
}

// What the tree's look is given. Every word is already in the person's
// language, and every role, state, id and handler sits in a wire bag
// the look spreads on the element it belongs to.

// The bag spread on the `<nav>`. Its symbol key is a Svelte attachment
// that hands the seat the element whose entries the line keys walk.
export interface NavWire {
  readonly "aria-label": string;
  readonly onkeydown: (event: KeyboardEvent) => void;
  readonly [hold: symbol]: Attachment<HTMLElement>;
}

// The bag spread on a button that opens a list of its own: a branch, a
// nest, a "more". `data-entry` marks every element the keys walk.
export interface FoldWire {
  readonly type: "button";
  readonly "data-entry": "";
  readonly "aria-expanded": boolean;
  readonly "aria-controls": string;
  readonly onclick: () => void;
}

// The bag spread on a group's button; `data-initial` is the letter the
// keys reach it by.
export interface GroupWire {
  readonly type: "button";
  readonly "data-entry": "";
  readonly "data-initial": string;
  readonly "aria-current": "true" | undefined;
  readonly onclick: () => void;
}

// The bag spread on a page's link.
export interface PageWire {
  readonly "data-entry": "";
  readonly href: string;
  readonly "aria-current": "page" | undefined;
}

export interface FoldLook {
  readonly label: string;
  readonly open: boolean;
  // The id of the list the fold opens.
  readonly list: string;
  readonly wire: FoldWire;
}

// `here` is the group drawn in the panel's body.
export interface GroupLook {
  readonly kind: "group";
  readonly key: string;
  readonly label: string;
  readonly initial: string;
  readonly here: boolean;
  readonly wire: GroupWire;
}

// `here` is the page under the panel; `reading` is the one line the
// performance page's entry carries about the city's processes.
export interface PageLook {
  readonly kind: "page";
  readonly key: string;
  readonly label: string;
  readonly here: boolean;
  readonly reading: string | null;
  readonly wire: PageWire;
}

export type LeafLook = GroupLook | PageLook;

export interface NestLook {
  readonly kind: "nest";
  readonly key: string;
  readonly fold: FoldLook;
  readonly pages: readonly PageLook[];
}

export interface MoreLook {
  readonly fold: FoldLook;
  readonly entries: readonly LeafLook[];
}

export interface BranchLook {
  readonly key: string;
  readonly fold: FoldLook;
  readonly entries: readonly (LeafLook | NestLook)[];
  readonly more: MoreLook | null;
}

export interface TreeLook {
  readonly nav: NavWire;
  readonly branches: readonly BranchLook[];
}

// What the tree reads besides its own presses: the group drawn, the
// page beneath, the folds, each nest's pages, and the performance
// reading.
export interface TreeState {
  readonly group: SetupGroup;
  readonly beneath: View;
  readonly folds: Folds;
  readonly leaves: Readonly<Record<Nest, readonly Page[]>>;
  readonly reading: string | null;
}

// What only the seat holds: the words of the language on the page, the
// presses, the key handler, and the attachment that finds the `<nav>`.
export interface TreeHands {
  readonly words: (key: Key) => string;
  readonly pick: (group: SetupGroup) => void;
  readonly toggle: (fold: Fold) => void;
  readonly walk: (event: KeyboardEvent) => void;
  readonly hold: Attachment<HTMLElement>;
}

const HOLD = createAttachmentKey();

// The whole value the tree's look draws.
export function lookOf(state: TreeState, uid: string, hands: TreeHands): TreeLook {
  const foldOf = (label: Key, open: boolean, list: string, fold: Fold): FoldLook => ({
    label: hands.words(label),
    open,
    list,
    wire: {
      type: "button",
      "data-entry": "",
      "aria-expanded": open,
      "aria-controls": list,
      onclick: () => {
        hands.toggle(fold);
      },
    },
  });
  const leafOf = (leaf: Leaf): LeafLook =>
    leaf.kind === "group" ? groupOf(leaf.group, state.group, hands) : pageOf(leaf, state, hands.words);
  const nestLookOf = (nest: Nest, word: Key): NestLook => ({
    kind: "nest",
    key: nest,
    fold: foldOf(word, state.folds.nests[nest], `${uid}-${nest}`, { kind: "nest", nest }),
    pages: state.leaves[nest].map((page) => pageOf(page, state, hands.words)),
  });
  return {
    nav: { "aria-label": hands.words("settings_tree"), onkeydown: hands.walk, [HOLD]: hands.hold },
    branches: TREE.map((branch, at) => {
      const list = `${uid}-${String(at)}`;
      return {
        key: branch.word,
        fold: foldOf(branch.word, state.folds.branch === at, list, { kind: "branch", at }),
        entries: branch.entries.map((entry) => (entry.kind === "nest" ? nestLookOf(entry.nest, entry.word) : leafOf(entry))),
        more:
          branch.more.length === 0
            ? null
            : { fold: foldOf("settings_more", state.folds.more, `${list}-more`, { kind: "more" }), entries: branch.more.map(leafOf) },
      };
    }),
  };
}

function groupOf(named: SetupGroup, drawn: SetupGroup, hands: TreeHands): GroupLook {
  const label = hands.words(HEADING[named]);
  const initial = initialOf(label, named);
  const here = named === drawn;
  return {
    kind: "group",
    key: named,
    label,
    initial,
    here,
    wire: {
      type: "button",
      "data-entry": "",
      "data-initial": initial,
      "aria-current": here ? "true" : undefined,
      onclick: () => {
        hands.pick(named);
      },
    },
  };
}

// A building's entry is named by its address, which is the city's word
// and not this page's.
function pageOf(page: Page, state: TreeState, words: (key: Key) => string): PageLook {
  const href = toFragment(page.view);
  const here = href === toFragment(state.beneath);
  return {
    kind: "page",
    key: href,
    label: page.view.kind === "building" ? page.view.address : words(page.word),
    here,
    reading: page.view.kind === "monitor" ? state.reading : null,
    wire: { "data-entry": "", href, "aria-current": here ? "page" : undefined },
  };
}
