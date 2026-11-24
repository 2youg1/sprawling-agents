// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { JSX } from "@solidjs/web";
import { createEffect, createMemo, createSignal, For, Show } from "solid-js";

import { say } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import { crumbsOf, gutterOf } from "./code";
import Copy from "./copy.solid";
import Inked from "./inked.solid";

export interface CodeProps {
  readonly path: string;
  readonly text: string;
  readonly cited?: number | undefined;
  readonly aside?: JSX.Element;
}

export default function Code(props: CodeProps) {
  const lang = storeValue(ui().lang);
  const crumbs = createMemo(() => crumbsOf(props.path));
  const gutter = createMemo(() => gutterOf(props.text));
  const [scroller, setScroller] = createSignal<HTMLDivElement>();
  const [mark, setMark] = createSignal<HTMLDivElement>();
  createEffect(
    () => ({ scroller: scroller(), mark: mark(), cited: props.cited }),
    (held) => {
      if (held.scroller === undefined || held.mark === undefined || held.cited === undefined)
        return;
      const frame = requestAnimationFrame(() => {
        const { scroller, mark } = held;
        if (scroller !== undefined && mark?.isConnected) {
          scroller.scrollTop = Math.max(0, mark.offsetTop - scroller.clientHeight / 2);
        }
      });
      return () => cancelAnimationFrame(frame);
    },
  );

  return (
    <div class="group flex min-h-0 min-w-0 flex-col">
      <div class="flex items-center gap-tight border-b border-edge px-snug py-tight text-note">
        <Show when={crumbs().length > 0}>
          <nav
            class="flex min-w-0 flex-wrap items-center gap-tight"
            aria-label={say(lang(), "code_crumbs")}
          >
            <For each={crumbs()} keyed={false}>
              {(crumb, index) => (
                <>
                  <Show when={index > 0}>
                    <span class="text-text-faint" aria-hidden="true">
                      /
                    </span>
                  </Show>
                  <span class={crumb().last ? "text-text-quiet" : "text-text-faint"}>
                    {crumb().part}
                  </span>
                </>
              )}
            </For>
          </nav>
        </Show>
        <span
          class={[
            "pointer-events-none ms-auto flex shrink-0 opacity-0 transition-opacity ease-leave still:transition-none",
            "group-hover:pointer-events-auto group-hover:opacity-100 group-hover:ease-arrive",
            "group-focus-within:pointer-events-auto group-focus-within:opacity-100 group-focus-within:ease-arrive",
          ]}
        >
          <Copy text={props.text} note={say(lang(), "code_copy_note")} form="bare" />
        </span>
        <Show when={props.aside}>
          {(aside) => <span class="flex shrink-0 items-center">{aside()}</span>}
        </Show>
      </div>
      <div class="min-h-0 flex-1 overflow-auto" ref={(node) => setScroller(node)}>
        <div class="relative flex min-w-max font-mono text-note leading-read">
          <pre
            class="sticky left-0 shrink-0 select-none bg-chrome px-snug text-right text-text-faint"
            aria-hidden="true"
          >
            {gutter()}
          </pre>
          <pre class="px-snug text-text-quiet">
            <Inked text={props.text} source={props.path} />
          </pre>
          <Show when={(props.cited ?? 0) >= 1}>
            <div
              ref={(node) => setMark(node)}
              class="cited pointer-events-none absolute inset-x-0 h-lh bg-accent/12 shadow-[inset_var(--spacing-hair)_0_0_var(--color-accent)]"
              style={{ top: `calc(${(props.cited ?? 1) - 1} * 1lh)` }}
              aria-hidden="true"
            />
          </Show>
        </div>
      </div>
    </div>
  );
}
