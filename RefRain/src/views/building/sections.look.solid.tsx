// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For, Show, createMemo } from "solid-js";
import type { SectionsLook } from "./sections";

export type PortProps = SectionsLook;

export default function BuildingSectionsLook(look: SectionsLook) {
  return (
    <>
      <ul class={"flex flex-col narrow:flex-row narrow:flex-wrap narrow:gap-tight"}>
        <For
          each={Array.from(look.entries ?? []).map((__rrItem0, __rrIndex1) => ({
            value: __rrItem0,
            key: __rrItem0.key,
          }))}
          keyed={(entry) => entry.key}
        >
          {(__rrItem0) => (
            <>
              <li>
                <button
                  data-control=""
                  {...(() => {
                    const { onclick: __rrArg0, ...__rrWire } = __rrItem0().value.wire;
                    return { ...__rrWire, onClick: __rrArg0 };
                  })()}
                  class={[
                    "relative flex h-control w-full items-center rounded-control px-snug text-left text-note narrow:w-auto",
                    __rrItem0().value.current
                      ? "wash-strong text-text"
                      : "text-text-quiet hover:wash",
                  ]}
                >
                  {(() => {
                    const __rrBranch3 = createMemo(() => {
                      const __rrCaptured2 = __rrItem0().value;
                      return __rrCaptured2.current
                        ? { yes: { __rrCaptured2 }, no: undefined }
                        : { yes: undefined, no: { __rrCaptured2 } };
                    });
                    return (
                      <Show when={__rrBranch3().yes}>
                        {(__rrActive4) => (
                          <>
                            <span
                              class={"absolute inset-y-snug left-0 w-hair rounded-pill bg-accent"}
                              aria-hidden={"true"}
                            />
                          </>
                        )}
                      </Show>
                    );
                  })()}
                  {__rrItem0().value.label}
                </button>
              </li>
            </>
          )}
        </For>
      </ul>
    </>
  );
}
