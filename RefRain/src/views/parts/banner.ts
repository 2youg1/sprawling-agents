// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// A strip across the top of a page for the condition the whole page is
// under: the city is stopped, the link is down, somebody is waiting to
// be answered. It is not a field's error and not a toast - it belongs to
// the page, it stays while the condition lasts, and it carries the one
// action that ends the condition.

import type { Snippet } from "svelte";

// A condition a person should know about, against one they must act on.
// This two-step scale is the banner's own; it is not the three paint
// tiers of `glyph.ts`.
export type Weight = "notice" | "alert";

export interface BannerProps {
  // Already in the person's language.
  readonly text: string;
  readonly detail?: string;
  readonly weight?: Weight;
  // Usually a Button: the way out of the condition.
  readonly action?: Snippet;
}

// What a look of the banner is given.
export interface BannerLook {
  readonly text: string;
  readonly detail: string | undefined;
  readonly weight: Weight;
  // Spread on the strip. The role is the whole contract: a page-level
  // alert announces itself the moment it appears, a notice waits its
  // turn.
  readonly strip: { readonly role: "alert" | "status" };
  readonly action: Snippet | undefined;
}

export function bannerOf(props: BannerProps): BannerLook {
  const weight = props.weight ?? "notice";
  return {
    text: props.text,
    detail: props.detail,
    weight,
    strip: { role: weight === "alert" ? "alert" : "status" },
    action: props.action,
  };
}
