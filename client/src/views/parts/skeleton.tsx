// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// The shape of what is coming, drawn while it is on its way. It has the
// same rows, the same heights and the same left edge as the real thing,
// so the page does not jump when the answer lands.
//
// The bars mean nothing to a screen reader, so the block announces
// itself once with the caller's word and hides the rest.

import { For } from "solid-js";

// The widths a line of text takes, so a block of them reads as prose
// rather than as a chart. Repeated for as many rows as are asked for.
const WIDTHS = ["w-full", "w-5/6", "w-2/3", "w-3/4"] as const;

export interface SkeletonProps {
  // What a screen reader is told is loading, in the person's language.
  readonly label: string;
  readonly rows?: number;
  // A block whose rows are one taller line each, the way a list of rows
  // loads, rather than a paragraph.
  readonly tall?: boolean;
}

export function Skeleton(props: SkeletonProps) {
  const rows = () => Array.from({ length: Math.max(props.rows ?? 3, 1) }, (_unused, index) => index);
  return (
    <div class="flex w-full flex-col gap-snug" role="status" aria-label={props.label} aria-busy="true">
      <For each={rows()}>
        {(index) => (
          <div
            aria-hidden="true"
            class={`animate-pulse rounded-control bg-g2 ${props.tall === true ? "h-step" : "h-base"} ${
              WIDTHS[index % WIDTHS.length] ?? "w-full"
            }`}
          />
        )}
      </For>
    </div>
  );
}
