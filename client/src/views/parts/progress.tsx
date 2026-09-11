// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// How far along something is, as a bar and as the two numbers the bar
// was drawn from. The numbers are there because a bar alone cannot be
// read out, compared, or believed.
//
// A total of zero means the end is not known yet: the bar then says it
// is busy instead of claiming a fraction it does not have.

import { Show } from "solid-js";

export interface ProgressProps {
  // The accessible name of the bar, already in the person's language.
  readonly label: string;
  readonly done: number;
  // Zero or less means the end is unknown.
  readonly total: number;
}

export function Progress(props: ProgressProps) {
  const known = () => props.total > 0;
  const share = () => (known() ? Math.min(Math.max(props.done / props.total, 0), 1) : 0);
  return (
    <div class="flex w-full min-w-0 items-center gap-base">
      <div
        class="h-snug min-w-0 flex-1 overflow-hidden rounded-pill bg-g2"
        role="progressbar"
        aria-label={props.label}
        aria-valuemin={0}
        aria-valuemax={known() ? props.total : undefined}
        aria-valuenow={known() ? props.done : undefined}
        aria-busy={!known()}
      >
        <Show
          when={known()}
          fallback={<div class="h-full w-1/3 animate-pulse rounded-pill bg-g5" />}
        >
          <div
            class="h-full rounded-pill bg-progress-done transition-[opacity,transform] duration-200 ease-[cubic-bezier(0.2,0,0,1)] motion-reduce:transition-none"
            style={{ width: `${String(Math.round(share() * 100))}%` }}
          />
        </Show>
      </div>
      <Show when={known()}>
        <span class="shrink-0 font-mono text-note text-text-quiet">
          {String(props.done)} / {String(props.total)}
        </span>
      </Show>
    </div>
  );
}
