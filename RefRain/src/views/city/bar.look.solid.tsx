// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For, Show, createMemo } from "solid-js";
import type { BarLook } from "./bar";

export type PortProps = BarLook;

export default function CityBarLook(look: BarLook) {
  const INK = { alert: "text-alert", plain: "text-text" } as const;
  return (
    <>
      <dl
        class={
          "grid grid-cols-[repeat(auto-fit,minmax(16ch,1fr))] gap-x-gutter gap-y-base border-b border-edge pb-base"
        }
        aria-label={look.label}
      >
        <For
          each={Array.from(look.readings ?? []).map((__rrItem0, __rrIndex1) => ({
            value: __rrItem0,
            key: __rrItem0.key,
          }))}
          keyed={(entry) => entry.key}
        >
          {(__rrItem0) => (
            <>
              <div class={"flex min-w-0 flex-col"}>
                <dt class={"truncate text-note text-text-faint"}>{__rrItem0().value.label}</dt>
                <dd class={"text-heading figure"}>
                  {(() => {
                    const __rrBranch3 = createMemo(() => {
                      const __rrCaptured2 = __rrItem0().value;
                      return __rrCaptured2.wire !== null
                        ? { yes: { __rrCaptured2 }, no: undefined }
                        : { yes: undefined, no: { __rrCaptured2 } };
                    });
                    return (
                      <Show
                        when={__rrBranch3().yes}
                        fallback={
                          <Show when={__rrBranch3().no}>
                            {(__rrActive4) => (
                              <>
                                <span class={INK[__rrActive4().__rrCaptured2.tone]}>
                                  {__rrActive4().__rrCaptured2.value}
                                </span>
                              </>
                            )}
                          </Show>
                        }
                      >
                        {(__rrActive4) => (
                          <>
                            <a
                              {...__rrActive4().__rrCaptured2.wire}
                              class={[
                                "transition-colors ease-leave hover:text-accent hover:ease-arrive",
                                INK[__rrActive4().__rrCaptured2.tone],
                              ]}
                            >
                              {__rrActive4().__rrCaptured2.value}
                            </a>
                          </>
                        )}
                      </Show>
                    );
                  })()}
                </dd>
              </div>
            </>
          )}
        </For>
      </dl>
    </>
  );
}
