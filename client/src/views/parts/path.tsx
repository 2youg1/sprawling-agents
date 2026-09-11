// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// One path, drawn the same way everywhere a path is printed: what the
// reader already knows is cut from the front, the whole of it arrives
// under the pointer, the left button opens it inside the page, and a
// second control offers to show it in the file manager.
//
// That second control is inert in this build and says so: the wire has
// no `Reveal` command, and a control whose only possible answer is
// silence owes the person the sentence saying why.

import { Show, createMemo } from "solid-js";

import { useSay } from "../../ui";

export interface PathProps {
  // As the city spells it: relative to the city, never to a disk.
  readonly path: string;
  // The part of the path the reader is already looking at, cut from
  // the front of what is drawn.
  readonly base?: string;
  // Absent leaves the path as text, which is what a path this build
  // cannot open should look like.
  readonly onOpen?: (() => void) | undefined;
}

export function Path(props: PathProps) {
  const say = useSay();
  const shown = createMemo(() => {
    const base = props.base;
    if (base === undefined || !props.path.startsWith(`${base}/`)) return props.path;
    return props.path.slice(base.length + 1);
  });
  return (
    <span class="inline-flex min-w-0 max-w-full items-center gap-tight">
      <Show
        when={props.onOpen}
        fallback={
          <span class="min-w-0 truncate font-mono text-note text-text-quiet" title={props.path}>
            {shown()}
          </span>
        }
      >
        {(open) => (
          <button
            type="button"
            class="min-w-0 truncate font-mono text-note text-text-quiet underline decoration-g4 underline-offset-2 hover:text-text"
            title={props.path}
            onClick={() => {
              open()();
            }}
          >
            {shown()}
          </button>
        )}
      </Show>
      <button
        type="button"
        class="shrink-0 rounded-control text-text-disabled"
        aria-disabled="true"
        aria-label={say("path_reveal")}
        title={say("path_reveal_inert")}
      >
        <svg viewBox="0 0 16 16" class="size-glyph" fill="none" stroke="currentColor" stroke-width="1.3" aria-hidden="true">
          <path d="M1.5 12.5v-9h4l1.4 1.8h7.6v7.2z" stroke-linejoin="round" />
          <path d="M6.5 9.5l4-3.5M8 6h2.8v2.6" stroke-linecap="round" stroke-linejoin="round" />
        </svg>
      </button>
    </span>
  );
}
