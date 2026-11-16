// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What the archive's list of hits is given (client D95: a seat, a look
// and this file): one row per hit, the day and the kind it was filed
// under, the building as the link to it, and what was filed.

import { toFragment } from "../../core/route";
import type { ArchiveHit } from "../../wire";

// Spread on the building's anchor.
export interface HitWire {
  readonly href: string;
}

export interface HitLook {
  readonly key: string;
  readonly day: string;
  readonly kind: string;
  readonly building: string;
  readonly wire: HitWire;
  readonly subject: string;
}

export interface ArchiveLook {
  readonly hits: readonly HitLook[];
}

export function lookOf(hits: readonly ArchiveHit[]): ArchiveLook {
  return {
    hits: hits.map((hit, at) => ({
      // Two hits may say the same thing on the same day; where each
      // stands in the answer is what tells them apart.
      key: `${String(at)}\u0000${hit.building}`,
      day: String(hit.day),
      kind: hit.kind,
      building: hit.building,
      wire: { href: toFragment({ kind: "building", address: hit.building }) },
      subject: hit.subject,
    })),
  };
}
