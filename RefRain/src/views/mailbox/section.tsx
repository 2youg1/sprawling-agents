// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { JSX } from "@solidjs/web";
import { createUniqueId, Show } from "solid-js";

export default function Section(props: {
  readonly title: string;
  readonly count: number;
  readonly empty: string;
  readonly tools?: JSX.Element;
  readonly children: JSX.Element;
}) {
  const id = createUniqueId();
  return (
    <section class="mt-wide first:mt-snug" aria-labelledby={id}>
      <h3 class="sticky top-0 flex h-control items-center gap-snug border-b border-edge bg-raised text-note text-text-faint">
        <span id={id}>{props.title}</span>
        <span class="figure text-text-quiet">{props.count}</span>
        <span class="flex-1" />
        {props.tools}
      </h3>
      <Show when={props.count === 0}>
        <p class="py-snug text-note text-text-quiet">{props.empty}</p>
      </Show>
      {props.children}
    </section>
  );
}
