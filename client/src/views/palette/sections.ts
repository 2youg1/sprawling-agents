// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Key } from "../../core/lang";
import type { Section } from "../../core/slash";

// The word for each section a verb names for itself in `core/slash.ts`.
export const SECTION_WORD: Readonly<Record<Section, Key>> = {
  actions: "palette_group_actions",
  navigation: "palette_group_navigation",
  sessions: "palette_group_sessions",
};
