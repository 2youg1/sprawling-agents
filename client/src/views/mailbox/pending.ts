// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Notice } from "../../core/belief";

export interface Stopped {
  readonly asks: readonly Notice[];
  readonly failures: readonly Notice[];
}

export function stoppedOf(notices: readonly Notice[]): Stopped {
  return { asks: [], failures: [...notices].reverse() };
}
