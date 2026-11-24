// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Snippet } from "../../core/snippet";

export type Weight = "notice" | "alert";

export interface BannerProps<Action = Snippet> {
  readonly text: string;
  readonly detail?: string | undefined;
  readonly weight?: Weight;
  readonly action?: Action | undefined;
}

export interface BannerLook<Action = Snippet> {
  readonly text: string;
  readonly detail: string | undefined;
  readonly weight: Weight;
  readonly strip: { readonly role: "alert" | "status" };
  readonly action: Action | undefined;
}

export function bannerOf<Action>(props: BannerProps<Action>): BannerLook<Action> {
  const weight = props.weight ?? "notice";
  return {
    text: props.text,
    detail: props.detail,
    weight,
    strip: { role: weight === "alert" ? "alert" : "status" },
    action: props.action,
  };
}
