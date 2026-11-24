// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For } from "solid-js";
import type { AgainstLook } from "./against";

export type PortProps = AgainstLook;

export default function EditorFormatsAgainstLook(props: AgainstLook) {
  return (
    <>
      <div class={"flex items-center gap-base border-b border-edge px-wide py-tight"}>
        <label class={"flex min-w-0 flex-1 items-center gap-snug text-note text-text-faint"}>
          {props.label}
          <select
            class={
              "h-control-sm min-w-0 flex-1 rounded-control border border-edge-input bg-raised px-snug text-note text-text"
            }
            {...(() => {
              const { onchange: __rrArg0, ...__rrWire } = props.wire;
              return { ...__rrWire, onChange: __rrArg0 };
            })()}
          >
            <option value={""}>{props.none}</option>
            <For
              each={Array.from(props.options ?? []).map((__rrItem0, __rrIndex1) => ({
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
      </div>
    </>
  );
}
