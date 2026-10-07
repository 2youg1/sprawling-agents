// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What one skill of a building is given (client D95: `skills.svelte` is
// the seat, `skill_row.look.svelte` draws the row). Every line is already
// in the person's language. A skill on an external shelf has no address
// in this city, so its row opens nothing and says so by being disabled.

export interface SkillRowWire {
  readonly type: "button";
  readonly disabled: boolean;
  readonly onclick: () => void;
}

export interface SkillRowLook {
  readonly name: string;
  // The summary its SKILL.md opens with.
  readonly summary: string;
  readonly shelf: string;
  // The address the row opens, empty for an external shelf.
  readonly at: string;
  readonly admitted: string;
  readonly used: string;
  readonly wire: SkillRowWire;
}
