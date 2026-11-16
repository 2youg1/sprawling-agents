// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Snippet } from "svelte";

export type Weight = "notice" | "alert";

export interface BannerProps {
  readonly text: string;
  readonly detail?: string;
  readonly weight?: Weight;
  readonly action?: Snippet;
}

export interface BannerLook {
  readonly text: string;
  readonly detail: string | undefined;
  readonly weight: Weight;
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
