// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For, Show } from "solid-js";
import Glyph from "../parts/glyph.solid";
import { PHASE_FILL } from "../runs/phase";
import type { TableLook } from "./table";

const COUNT = "inline-flex w-count shrink-0 items-center justify-end gap-tight font-mono figure";

export default function TableLookView(look: TableLook) {
  return (
    <section class="@container/table flex w-full min-w-0 flex-col" aria-label={look.label}>
      <div
        class="flex min-w-0 flex-wrap items-end gap-x-base border-b border-edge pr-snug pb-tight font-mono figure text-note text-text-quiet"
        aria-hidden="true"
      >
        <span class="hidden min-w-0 flex-1 @lg/table:block"></span>
        <span class="relative h-base w-full shrink-0 @lg/table:w-[40%]">
          <For each={look.folds} keyed={(item) => item.minutes}>
            {(fold) => (
              <>
                <span
                  class="absolute top-0 border-l border-edge-input pl-tight"
                  style={{ left: `${fold().at}%` }}
                >
                  {fold().label}
                </span>
              </>
            )}
          </For>
          <span class="absolute top-0 right-0 font-sans text-text">{look.now}</span>
        </span>
      </div>
      <ul class="flex min-w-0 flex-col">
        <For each={look.rows} keyed={(item) => item.key}>
          {(row) => (
            <>
              <li class="border-b border-edge">
                <button
                  data-control=""
                  type={row().wire.type}
                  aria-pressed={row().wire["aria-pressed"] ? "true" : "false"}
                  aria-label={row().wire["aria-label"]}
                  onClick={() => row().wire.onclick()}
                  class={[
                    "flex w-full min-w-0 flex-wrap items-center gap-x-base gap-y-tight py-snug pr-snug text-left text-note",
                    "transition-colors ease-leave hover:wash hover:ease-arrive",
                    row().picked ? "wash-strong shadow-[inset_2px_0_0_var(--color-accent)]" : "",
                  ]}
                >
                  <span class="flex min-w-0 flex-1 items-center gap-base">
                    <span
                      class="shrink-0 pl-snug font-mono whitespace-pre text-text-quiet"
                      aria-hidden="true"
                    >
                      {row().guide}
                    </span>
                    <span class="flex min-w-0 flex-1 items-baseline gap-base">
                      <span class="min-w-0 truncate font-mono font-label text-text">
                        {row().name}
                      </span>
                      <span
                        class="min-w-0 truncate font-mono text-note text-text-faint"
                        title={row().path}
                      >
                        {row().path}
                      </span>
                    </span>
                    <span class={[COUNT, row().waiting === 0 ? "text-text-quiet" : "text-alert"]}>
                      <Glyph name="hand" size="sm" />
                      {String(row().waiting)}
                    </span>
                    <span class={[COUNT, row().working === 0 ? "text-text-quiet" : "text-accent"]}>
                      <Glyph name="pulse" size="sm" />
                      {String(row().working)}
                    </span>
                    <span class={[COUNT, "text-text-quiet"]}>
                      <Glyph name="check" size="sm" />
                      {String(row().done)}
                    </span>
                    <span class="hidden w-[12ch] shrink-0 truncate text-right figure text-text-quiet @lg/table:block">
                      {row().latest}
                    </span>
                  </span>
                  <span
                    class="relative h-snug w-full shrink-0 @lg/table:w-[40%]"
                    aria-hidden="true"
                  >
                    <span class="absolute inset-0 border-r border-edge-input"></span>
                    <Show when={row().live}>
                      {(live) => (
                        <>
                          <span
                            class="absolute inset-y-0 rounded-pill bg-edge"
                            style={{
                              left: `${live().left.toFixed(3)}%`,
                              width: `${live().width.toFixed(3)}%`,
                            }}
                          ></span>
                        </>
                      )}
                    </Show>
                    <For each={row().starts} keyed={(item) => item.key}>
                      {(start) => (
                        <>
                          <span
                            class={[
                              "absolute inset-y-0 w-tight rounded-pill",
                              PHASE_FILL[start().phase],
                            ]}
                            style={{
                              left: `clamp(0px, calc(${start().at.toFixed(3)}% - var(--spacing-tight) / 2), calc(100% - var(--spacing-tight)))`,
                            }}
                          ></span>
                        </>
                      )}
                    </For>
                  </span>
                </button>
              </li>
            </>
          )}
        </For>
      </ul>
    </section>
  );
}
