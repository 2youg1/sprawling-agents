// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For } from "solid-js";
import type { PairLook } from "./pair";

export type PortProps = PairLook;

export default function EditorPairLook(look: PairLook) {
  return (
    <>
      <div
        class={
          "grid grid-cols-[auto_minmax(0,1fr)] items-center gap-x-base gap-y-snug border-b border-edge px-wide py-snug"
        }
      >
        <For
          each={Array.from(look.sides ?? []).map((__rrItem0, __rrIndex1) => ({
            value: __rrItem0,
            key: __rrItem0.key,
          }))}
          keyed={(entry) => entry.key}
        >
          {(__rrItem0) => (
            <>
              <span class={"text-note text-text-faint"}>{__rrItem0().value.label}</span>
              <select
                {...(() => {
                  const { onchange: __rrArg0, ...__rrWire } = __rrItem0().value.wire;
                  return { ...__rrWire, onChange: __rrArg0 };
                })()}
                value={__rrItem0().value.held}
                class={
                  "h-control-sm w-full min-w-0 rounded-control border border-edge-input bg-raised px-snug text-note text-text transition-colors ease-leave hover:bg-raised-hover hover:ease-arrive"
                }
              >
                <For
                  each={Array.from(__rrItem0().value.options ?? []).map(
                    (__rrItem2, __rrIndex3) => ({
                      value: __rrItem2,
                      key: __rrItem2.value,
                    }),
                  )}
                  keyed={(entry) => entry.key}
                >
                  {(__rrItem2) => (
                    <>
                      <option value={__rrItem2().value.value} disabled={__rrItem2().value.disabled}>
                        {__rrItem2().value.label}
                      </option>
                    </>
                  )}
                </For>
              </select>
            </>
          )}
        </For>
        {look.more !== null ? (
          <>
            <p class={"col-span-2 text-note text-text-faint"}>{look.more}</p>
          </>
        ) : undefined}
      </div>
    </>
  );
}
