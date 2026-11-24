// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { get, readonly, writable } from "../../core/store";

import type { Combination } from "./picker";
import { RECENT, sameCombination } from "./picker";

class Recent {
  readonly #kept = writable<readonly Combination[]>([]);
  readonly choices = readonly(this.#kept);

  get kept(): readonly Combination[] {
    return get(this.#kept);
  }

  keep(combination: Combination): void {
    this.#kept.update((kept) =>
      [combination, ...kept.filter((each) => !sameCombination(each, combination))].slice(
        0,
        RECENT + 1,
      ),
    );
  }
}

export const RECENT_COMBINATIONS = new Recent();
