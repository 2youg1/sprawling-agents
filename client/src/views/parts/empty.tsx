// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// What stands where a list has nothing in it: a shape the eye lands on,
// one sentence saying what is missing, and the one action that ends the
// emptiness. A grey word on its own leaves a person unsure whether the
// page is empty or broken.

import { Show, type JSX } from "solid-js";

export interface EmptyStateProps {
  // Already in the person's language.
  readonly text: string;
  // The outline the eye lands on. A dashed box stands in when the page
  // has nothing better to draw.
  readonly shape?: JSX.Element;
  // Usually one Button.
  readonly action?: JSX.Element;
}

export function EmptyState(props: EmptyStateProps) {
  return (
    <div class="flex w-full flex-col items-center gap-base px-pane py-section text-center">
      <Show
        when={props.shape}
        fallback={<div class="size-figure rounded-panel border border-dashed border-g4" aria-hidden="true" />}
      >
        {(shape) => <div aria-hidden="true">{shape()}</div>}
      </Show>
      <p class="max-w-measure text-note text-text-quiet">{props.text}</p>
      <Show when={props.action}>{(action) => action()}</Show>
    </div>
  );
}
