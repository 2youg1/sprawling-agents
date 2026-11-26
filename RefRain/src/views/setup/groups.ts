// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Key } from "../../core/lang";
import type { SetupGroup } from "../../core/route";

export const HEADING: Record<SetupGroup, Key> = {
  you: "setup_group_you",
  performance: "setup_group_performance",
  accounts: "setup_group_accounts",
  agents: "acp_title",
  network: "setup_group_network",
  remote: "setup_group_remote",
  devices: "devices_title",
  run: "setup_group_run",
  rules: "setup_group_rules",
  automation: "setup_group_automation",
  writing: "setup_group_writing",
  skills: "setup_group_skills",
  tools: "setup_group_tools",
  appearance: "setup_group_appearance",
  colours: "setup_group_colours",
  keys: "setup_group_keys",
  advanced: "setup_group_advanced",
  privacy: "setup_group_privacy",
  about: "release_title",
};

export const HINT: Record<SetupGroup, Key | null> = {
  you: "setup_group_hint_you",
  performance: "setup_group_hint_performance",
  accounts: "setup_group_hint_accounts",
  agents: "acp_about",
  network: "setup_group_hint_network",
  remote: "setup_group_hint_remote",
  devices: "devices_note",
  run: "setup_group_hint_run",
  rules: "setup_group_hint_rules",
  automation: "setup_group_hint_automation",
  writing: null,
  skills: "setup_group_hint_skills",
  tools: "setup_group_hint_tools",
  appearance: "setup_group_hint_appearance",
  colours: "setup_group_hint_colours",
  keys: "setup_group_hint_keys",
  advanced: "setup_group_hint_advanced",
  privacy: "setup_group_hint_privacy",
  about: null,
};

export const WIDTH: Record<SetupGroup, string> = {
  you: "",
  performance: "max-w-talk",
  accounts: "",
  agents: "max-w-talk",
  network: "max-w-talk",
  remote: "max-w-talk",
  devices: "max-w-talk",
  run: "",
  rules: "max-w-talk",
  automation: "max-w-talk",
  writing: "max-w-talk",
  skills: "",
  tools: "",
  appearance: "",
  colours: "",
  keys: "max-w-talk",
  advanced: "",
  privacy: "",
  about: "max-w-talk",
};

export const PREFERRED: readonly SetupGroup[] = ["network", "appearance", "colours", "keys"];
