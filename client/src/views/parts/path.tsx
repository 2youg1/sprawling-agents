// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

// One path, drawn the same way everywhere a path is printed: what the
// reader already knows is cut from the front, the whole of it is on the
// screen, the left button opens it inside the page, and a second
// control offers to show it in the file manager.
//
// **The whole path is drawn, and it wraps after a `/`.** Cutting it off
// under an ellipsis put the only copy of the value in a `title`, which
// no key reaches and no touch screen shows; breaking it at any
// character split `completions` across two lines. A `<wbr>` after each
// separator gives the engine the places a reader expects a path to
// break, and `overflow-wrap: anywhere` is what it falls back on when a
// single segment is still wider than the column.
//
// The second control asks the city to hand the path to the desktop's own
// file manager. A path this build cannot turn into an address stays text
// with the control saying why, because a button whose only possible
// answer is silence owes the person that sentence.

import { Option, Schema } from "effect";
import { For, Show, createMemo } from "solid-js";

import { Address } from "../../wire";
import { reveal } from "../../core/commands";
import { useCommand, useSay } from "../../ui";
import { Tip } from "./tip";

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

// The pieces a path may wrap between: each separator stays with the
// segment it closes, so a line never begins with a bare `/`.
function upToEachSlash(path: string): readonly string[] {
  const parts = path.split("/");
  return parts.map((part, at) => (at === parts.length - 1 ? part : `${part}/`));
}

// The path with a break opportunity after every separator. A single
// element rather than one per segment, because a row of inline boxes
// would let the engine put a gap between them.
function Broken(props: { readonly path: string }) {
  const parts = createMemo(() => upToEachSlash(props.path));
  return (
    <For each={parts()}>
      {(part, at) => (
        <>
          <Show when={at() > 0}>
            <wbr />
          </Show>
          {part}
        </>
      )}
    </For>
  );
}

const TEXT = "min-w-0 wrap-anywhere font-mono text-note text-text-quiet";

export function Path(props: PathProps) {
  const say = useSay();
  const send = useCommand();
  const address = createMemo(() => Schema.decodeOption(Address)(props.path));
  const shown = createMemo(() => {
    const base = props.base;
    if (base === undefined || !props.path.startsWith(`${base}/`)) return props.path;
    return props.path.slice(base.length + 1);
  });
  return (
    <span class="inline-flex min-w-0 max-w-full items-baseline gap-tight">
      <Show
        when={props.onOpen}
        fallback={
          <span class={TEXT}>
            <Broken path={shown()} />
          </span>
        }
      >
        {(open) => (
          <button
            type="button"
            class={`${TEXT} text-left underline decoration-g4 underline-offset-2 hover:text-text`}
            onClick={() => {
              open()();
            }}
          >
            <Broken path={shown()} />
          </button>
        )}
      </Show>
      <Tip text={Option.isNone(address()) ? say("path_reveal_inert") : say("path_reveal")}>
        {(hint) => (
          <button
            type="button"
            class="shrink-0 rounded-control text-text-disabled hover:text-text-quiet"
            aria-disabled={Option.isNone(address()) ? "true" : undefined}
            aria-label={say("path_reveal")}
            aria-describedby={hint}
            onClick={() => {
              Option.map(address(), (at) => send(reveal(at)));
            }}
          >
            <svg viewBox="0 0 16 16" class="size-glyph" fill="none" stroke="currentColor" stroke-width="1.3" aria-hidden="true">
              <path d="M1.5 12.5v-9h4l1.4 1.8h7.6v7.2z" stroke-linejoin="round" />
              <path d="M6.5 9.5l4-3.5M8 6h2.8v2.6" stroke-linecap="round" stroke-linejoin="round" />
            </svg>
          </button>
        )}
      </Tip>
    </span>
  );
}
