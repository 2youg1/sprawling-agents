// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// How the palette groups the verbs of `core/slash.ts`: this screen's
// arrangement of the one table, never a second list of verbs.

import type { Key } from "../../core/lang";

// The three sections a verb falls under, and the word for each,
// decided by the spelling both halves of the slash seam already
// share.
export const SECTIONS = ["actions", "navigation", "sessions"] as const;
export type Section = (typeof SECTIONS)[number];
export const SECTION_WORD: Readonly<Record<Section, Key>> = {
  actions: "palette_group_actions",
  navigation: "palette_group_navigation",
  sessions: "palette_group_sessions",
};
const SECTION: Readonly<Record<string, Section>> = {
  "/dispatch": "sessions",
  "/steer": "sessions",
  "/new": "sessions",
  "/fork": "sessions",
  "/go": "navigation",
  "/mcp": "navigation",
  "/doctor": "navigation",
  "/stop": "actions",
  "/release": "actions",
  "/raise": "actions",
  "/model": "actions",
  "/effort": "actions",
  "/help": "actions",
  "/clear": "actions",
};
// A verb `core/slash.ts` grew before this screen classified it lands
// with the actions - visible and runnable, which is how its section
// gets named next time.
export const sectionOf = (spelling: string): Section => SECTION[spelling] ?? "actions";
