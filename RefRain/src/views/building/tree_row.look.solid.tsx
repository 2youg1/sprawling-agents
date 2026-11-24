// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createUniqueId, Match, Show, Switch } from "solid-js";

import Tip from "../parts/control_tip";
import Glyph from "../parts/glyph.solid";
import type { TreeRowLook, TreeTone } from "./tree_row";

const INK: Record<TreeTone, string> = {
  picked: "wash-strong text-text",
  hidden: "text-text-faint",
  plain: "text-text-quiet",
};

const TRANSCRIPT_ARROW = "↗";

export default function TreeRow(look: TreeRowLook) {
  const id = createUniqueId();
  return (
    <button
      data-control=""
      type={look.wire.type}
      aria-expanded={
        look.mark.kind === "directory" ? (look.mark.open ? "true" : "false") : undefined
      }
      onClick={() => look.wire.onclick()}
      class={[
        "group/row flex h-step w-full items-center gap-tight rounded-control pl-tight pr-snug text-left text-note leading-none",
        look.tone !== "picked" && "hover:wash",
        INK[look.tone],
      ]}
    >
      <span class="flex w-glyph-sm shrink-0 justify-center">
        <Switch>
          <Match when={look.mark.kind === "directory" && look.mark}>
            {(mark) => (
              <Glyph
                name="chevron"
                size="sm"
                class={[
                  "shrink-0 text-text-faint transition-transform still:transition-none",
                  mark().open ? "rotate-90 ease-arrive" : "ease-leave",
                ].join(" ")}
              />
            )}
          </Match>
          <Match when={look.mark.kind === "transcript" && look.mark}>
            {(mark) => (
              <Tip text={mark().hint} id={`${id}-transcript`}>
                <span class="text-text-faint" role="img" aria-labelledby={`${id}-transcript`}>
                  {TRANSCRIPT_ARROW}
                </span>
              </Tip>
            )}
          </Match>
        </Switch>
      </span>
      <span class={["truncate", look.coded && "font-mono text-text-faint"]}>{look.name}</span>
      <Show when={look.live}>
        {(live) => (
          <Tip text={live()} id={`${id}-live`}>
            <span
              class="ml-tight inline-block size-dot shrink-0 rounded-pill bg-accent"
              role="img"
              aria-labelledby={`${id}-live`}
            />
          </Tip>
        )}
      </Show>
      <span class="flex-1" />
      <Show when={look.size !== undefined}>
        <span class="hidden shrink-0 whitespace-nowrap font-mono text-text-faint group-hover/row:inline group-focus-visible/row:inline">
          {look.size}
        </span>
      </Show>
    </button>
  );
}
