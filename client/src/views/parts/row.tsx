// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// One line of a list: what the thing is called, what else is worth
// knowing about it, where it stands, and what can be done to it.
//
// When the row leads somewhere, the two texts are the button that goes
// there, so the keyboard reaches the same place the pointer does and the
// actions on the right stay separately reachable.

import { Show, type JSX } from "solid-js";

export interface RowProps {
  // Already in the person's language, or an identifier.
  readonly primary: string;
  readonly secondary?: string;
  // Usually a Badge: where the thing stands.
  readonly status?: JSX.Element;
  // Usually Buttons: what can be done to it.
  readonly actions?: JSX.Element;
  // Present makes the row lead somewhere.
  readonly onOpen?: () => void;
}

export function Row(props: RowProps) {
  const texts = () => (
    <>
      <span class="truncate text-body text-text">{props.primary}</span>
      <Show when={props.secondary}>
        {(secondary) => <span class="truncate text-note text-text-faint">{secondary()}</span>}
      </Show>
    </>
  );
  return (
    <div class="flex w-full min-w-0 items-center gap-base border-b border-g2 px-base py-snug hover:bg-g1">
      <Show
        when={props.onOpen}
        fallback={<div class="flex min-w-0 flex-1 flex-col text-left">{texts()}</div>}
      >
        {(open) => (
          <button
            type="button"
            class="flex min-w-0 flex-1 flex-col text-left"
            onClick={() => {
              open()();
            }}
          >
            {texts()}
          </button>
        )}
      </Show>
      <Show when={props.status}>{(status) => <div class="shrink-0">{status()}</div>}</Show>
      <Show when={props.actions}>{(actions) => <div class="flex shrink-0 items-center gap-tight">{actions()}</div>}</Show>
    </div>
  );
}
