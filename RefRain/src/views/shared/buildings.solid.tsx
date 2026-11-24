// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For } from "solid-js";
import type { BuildingColumnProps } from "./buildings";
import { HALL } from "./buildings";

export type PortProps = BuildingColumnProps;

export default function SharedBuildings(props: BuildingColumnProps) {
  return (
    <>
      <nav class={"w-tree max-w-full shrink-0"} aria-label={props.label}>
        <ul class={"flex flex-col gap-tight"}>
          <For
            each={Array.from(props.buildings ?? []).map((__rrItem0, __rrIndex1) => ({
              value: __rrItem0,
              key: __rrItem0,
            }))}
            keyed={(entry) => entry.key}
          >
            {(__rrItem0) => (
              <>
                <li>
                  <button
                    data-control=""
                    type={"button"}
                    class={[
                      "h-control-sm w-full rounded-control px-base text-left text-note",
                      props.chosen === __rrItem0().value
                        ? "bg-raised text-text"
                        : "text-text-quiet hover:bg-chrome",
                    ]}
                    aria-current={props.chosen === __rrItem0().value ? "true" : undefined}
                    onClick={() => {
                      props.onPick(__rrItem0().value);
                    }}
                  >
                    {__rrItem0().value === HALL ? props.hall : __rrItem0().value}
                  </button>
                </li>
              </>
            )}
          </For>
        </ul>
      </nav>
    </>
  );
}
