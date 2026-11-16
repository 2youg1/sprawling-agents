// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The deduplication key of one outward action: `idem1-` then 32 hex
// digits, drawn from the browser's entropy. Minted once per press, so a
// retry after a dropped socket carries the same key and the city does
// the thing once.

import type { IdemKey } from "../wire";
import { IdemKey as IdemKeySchema } from "../wire";

export function mintIdem(): IdemKey {
  const bytes = new Uint8Array(16);
  crypto.getRandomValues(bytes);
  let hex = "";
  for (const byte of bytes) {
    hex += byte.toString(16).padStart(2, "0");
  }
  return IdemKeySchema.make(`idem1-${hex}`);
}
