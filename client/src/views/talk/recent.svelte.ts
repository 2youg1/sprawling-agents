// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The model combinations this tab ended a choice with, newest first,
// which the model picker lists as its recent section. Held for the tab
// rather than for one composer, so a combination chosen in one room is
// one activation away in the next.

import type { Combination } from "./picker";
import { RECENT, sameCombination } from "./picker";

class Recent {
  #kept = $state<readonly Combination[]>([]);

  get kept(): readonly Combination[] {
    return this.#kept;
  }

  // A combination ended on moves to the front; the oldest beyond the
  // picker's recent section and the one in force is let go.
  keep(combination: Combination): void {
    this.#kept = [combination, ...this.#kept.filter((each) => !sameCombination(each, combination))].slice(0, RECENT + 1);
  }
}

export const RECENT_COMBINATIONS = new Recent();
