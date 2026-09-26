// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The settings page's groups: their order, what each is called, the
// line under its heading, how wide its body grows, and what the
// navigation down the left holds. One table per fact, read by
// `setup.svelte`.

import type { Key } from "../../core/lang";

const GROUPS = [
  "accounts", "run", "network", "tools", "skills", "appearance", "keys", "advanced", "about",
] as const;

export type Group = (typeof GROUPS)[number];

// What each group is called and how wide its body may grow
// (client-SPEC 4-33, 4-36), one table per fact.
export const HEADING: Record<Group, Key> = {
  accounts: "setup_group_accounts",
  run: "setup_group_run",
  network: "setup_group_network",
  tools: "setup_group_tools",
  skills: "setup_group_skills",
  appearance: "setup_group_appearance",
  keys: "setup_group_keys",
  advanced: "setup_group_advanced",
  about: "release_title",
};

// The line under the heading, saying what the group governs - the one
// kind of sentence this page is allowed (client-SPEC 4-10).
export const HINT: Record<Group, Key | null> = {
  accounts: "setup_group_hint_accounts",
  run: "setup_group_hint_run",
  network: "setup_group_hint_network",
  tools: "setup_group_hint_tools",
  skills: "setup_group_hint_skills",
  appearance: "setup_group_hint_appearance",
  keys: "setup_group_hint_keys",
  advanced: "setup_group_hint_advanced",
  about: null,
};

// The one width table: accounts, tools and skills take the page tier,
// the rest the measure; the account group's two exceptions are in its
// own body below.
export const WIDTH: Record<Group, string> = {
  accounts: "max-w-page",
  run: "max-w-measure",
  network: "max-w-measure",
  tools: "max-w-page",
  skills: "max-w-page",
  appearance: "max-w-measure",
  keys: "max-w-measure",
  advanced: "max-w-measure",
  about: "max-w-measure",
};

// The groups whose answers `core/prefs.ts` keeps (client-SPEC 4-29).
export const PREFERRED: readonly Group[] = ["network", "appearance", "keys"];

// What the navigation holds: the groups in their own order, with the
// MCP door standing where its group stood. The door leaves this page,
// so it is a link and carries no `aria-current`.
type NavEntry = { readonly kind: "group"; readonly group: Group } | { readonly kind: "door" };

export const NAV: readonly NavEntry[] = GROUPS.flatMap((each): readonly NavEntry[] =>
  each === "run" ? [{ kind: "door" }, { kind: "group", group: each }] : [{ kind: "group", group: each }]);

// One drawing for a navigation row; the 2px accent bar says which row
// is this page's current group (client-SPEC 7B).
export const NAV_WEAR =
  "flex h-bar shrink-0 items-center border-b-2 px-base text-left text-label " +
  "hover:bg-chrome hover:text-text @lg/page:h-auto @lg/page:border-b-0 @lg/page:border-l-2 @lg/page:py-snug";
export const NAV_HERE = "border-accent text-text";
export const NAV_THERE = "border-transparent text-text-faint";
