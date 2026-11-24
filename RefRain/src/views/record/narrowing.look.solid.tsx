// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For } from "solid-js";
import type { NarrowingLook } from "./narrowing";

export type PortProps = NarrowingLook;

export default function RecordNarrowingLook(look: NarrowingLook) {
  return (
    <>
      <label class={"flex items-center gap-snug text-note text-text-faint"}>
        {look.label}
        <select
          {...(() => {
            const { onchange: __rrArg0, ...__rrWire } = look.wire;
            return { ...__rrWire, onChange: __rrArg0 };
          })()}
          class={[
            "h-control-sm max-w-[24ch] rounded-control bg-raised px-snug font-mono text-note text-text transition-colors still:transition-none",
            "ease-leave hover:bg-raised-hover hover:ease-arrive",
          ]}
        >
          <For
            each={Array.from(look.options ?? []).map((__rrItem0, __rrIndex1) => ({
              value: __rrItem0,
              key: __rrItem0.value,
            }))}
            keyed={(entry) => entry.key}
          >
            {(__rrItem0) => (
              <>
                <option value={__rrItem0().value.value}>{__rrItem0().value.label}</option>
              </>
            )}
          </For>
        </select>
      </label>
    </>
  );
}
