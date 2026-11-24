// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For } from "solid-js";
import type { ArmSelectLook } from "./arm_select";

export type PortProps = ArmSelectLook;

export default function BuildingArmSelectLook(look: ArmSelectLook) {
  return (
    <>
      <select
        {...(() => {
          const { onchange: __rrArg0, ...__rrWire } = look.wire;
          return { ...__rrWire, onChange: __rrArg0 };
        })()}
        class={
          "h-control w-full rounded-control border border-edge-input bg-raised px-base text-note text-text"
        }
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
    </>
  );
}
