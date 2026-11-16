// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

export interface SkillRowWire {
  readonly type: "button";
  readonly disabled: boolean;
  readonly onclick: () => void;
}

export interface SkillRowLook {
  readonly name: string;
  readonly summary: string;
  readonly shelf: string;
  readonly at: string;
  readonly admitted: string;
  readonly used: string;
  readonly changed: string | undefined;
  readonly wire: SkillRowWire;
}
