// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The settings panel's groups: what each is called, the line under its
// heading, and how wide its body grows. One table per fact, read by
// `setup.svelte`; the order a person meets them in is the settings
// tree's (`views/settings/tree.ts`), and the set is the address bar's
// (`core/route.ts`), so a group added there leaves these tables
// refusing to compile until it has a name.

import type { Key } from "../../core/lang";
import type { SetupGroup } from "../../core/route";

// What each group is called (client/Spec.lean §4-36).
export const HEADING: Record<SetupGroup, Key> = {
  you: "setup_group_you",
  accounts: "setup_group_accounts",
  harnesses: "setup_group_harnesses",
  network: "setup_group_network",
  remote: "setup_group_remote",
  run: "setup_group_run",
  rules: "setup_group_rules",
  automation: "setup_group_automation",
  skills: "setup_group_skills",
  tools: "setup_group_tools",
  appearance: "setup_group_appearance",
  colours: "setup_group_colours",
  keys: "setup_group_keys",
  advanced: "setup_group_advanced",
  privacy: "setup_group_privacy",
  about: "release_title",
};

// The line under the heading, saying what the group governs - the one
// kind of sentence this panel is allowed (client/Spec.lean §4-10).
export const HINT: Record<SetupGroup, Key | null> = {
  you: "setup_group_hint_you",
  accounts: "setup_group_hint_accounts",
  harnesses: "setup_group_hint_harnesses",
  network: "setup_group_hint_network",
  remote: "setup_group_hint_remote",
  run: "setup_group_hint_run",
  rules: "setup_group_hint_rules",
  automation: "setup_group_hint_automation",
  skills: "setup_group_hint_skills",
  tools: "setup_group_hint_tools",
  appearance: "setup_group_hint_appearance",
  colours: "setup_group_hint_colours",
  keys: "setup_group_hint_keys",
  advanced: "setup_group_hint_advanced",
  privacy: "setup_group_hint_privacy",
  about: null,
};

// The one width table. A group of cards takes the whole body and lays
// its cards in as many measure-wide columns as it holds (`grid-fit`);
// a group that is one list, one form or one text reads badly stretched
// and stops at the conversation's width.
export const WIDTH: Record<SetupGroup, string> = {
  you: "",
  accounts: "",
  harnesses: "max-w-talk",
  network: "max-w-talk",
  remote: "max-w-talk",
  run: "",
  rules: "max-w-talk",
  automation: "max-w-talk",
  skills: "",
  tools: "",
  appearance: "",
  colours: "",
  keys: "max-w-talk",
  advanced: "",
  privacy: "",
  about: "max-w-talk",
};

// The groups whose answers `core/prefs.ts` keeps (client/Spec.lean §4-29).
export const PREFERRED: readonly SetupGroup[] = ["network", "appearance", "colours", "keys"];
