// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Combination } from "./picker";
import { RECENT, sameCombination } from "./picker";

class Recent {
  #kept = $state<readonly Combination[]>([]);

  get kept(): readonly Combination[] {
    return this.#kept;
  }

  keep(combination: Combination): void {
    this.#kept = [
      combination,
      ...this.#kept.filter((each) => !sameCombination(each, combination)),
    ].slice(0, RECENT + 1);
  }
}

export const RECENT_COMBINATIONS = new Recent();
