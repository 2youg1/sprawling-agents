// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What a doctor item's name link decides before it is drawn: whether
// it links to a release or installation page, and the target that keeps
// the doctor page open while the browser opens that page.

import type { DoctorItem } from "../../wire";

export interface DoctorLinkLook {
  // The text the link carries: the item's name.
  readonly name: string;
  // The release or installation page, or `null` when the table names none.
  readonly homepage: string | null;
  // The target that asks the browser to open the page in a new context.
  readonly target: "_blank";
}

/**
 * Build the wiring for a doctor item's name link. When the city gives a
 * homepage, the name becomes an anchor with `target="_blank"` so the
 * browser opens the release or installation page without leaving the
 * doctor screen. When there is no homepage, the name is plain text.
 *
 * @param item - The doctor item whose name may link to its page.
 * @returns The look the link look spreads on its element.
 */
export function lookOf(item: DoctorItem): DoctorLinkLook {
  return {
    name: item.name,
    homepage: item.homepage ?? null,
    target: "_blank",
  };
}
