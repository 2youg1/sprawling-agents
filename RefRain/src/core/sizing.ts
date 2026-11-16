// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { BODY_PX } from "../wire";

export type Sizing =
  | { readonly kind: "cleared" }
  | { readonly kind: "sized"; readonly px: number }
  | { readonly kind: "refused" };

export function sizingOf(text: string): Sizing {
  const trimmed = text.trim();
  if (trimmed === "") return { kind: "cleared" };
  if (!/^[0-9]+$/.test(trimmed)) return { kind: "refused" };
  const px = Number.parseInt(trimmed, 10);
  return px >= BODY_PX.min ? { kind: "sized", px } : { kind: "refused" };
}
