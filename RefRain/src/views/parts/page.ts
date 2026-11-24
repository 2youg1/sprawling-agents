// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Snippet } from "../../core/snippet";

export type Rank = "page" | "section";

export interface PageProps<Content = Snippet> {
  readonly title: string;
  readonly rank?: Rank | undefined;
  readonly note?: string | undefined;
  readonly aside?: Content | undefined;
  readonly children: Content;
}

export interface PageLook<Content = Snippet> {
  readonly title: string;
  readonly rank: Rank;
  readonly heading: { readonly tabindex: -1 } | Record<string, never>;
  readonly note: string | undefined;
  readonly aside: Content | undefined;
  readonly children: Content;
}

export function pageOf<Content>(props: PageProps<Content>): PageLook<Content> {
  const rank = props.rank ?? "page";
  return {
    title: props.title,
    rank,
    heading: rank === "page" ? { tabindex: -1 } : {},
    note: props.note,
    aside: props.aside,
    children: props.children,
  };
}
