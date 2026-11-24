// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For, Show, createMemo } from "solid-js";
import type { UsesLook } from "./usage_uses";

export type PortProps = UsesLook;

export default function PartsUsageUsesLook(look: UsesLook) {
  return (
    <>
      {look.kind === "never" ? (
        <>
          <p class={"text-note text-text-faint"}>{look.said}</p>
        </>
      ) : (
        <>
          <p class={"text-note text-text-quiet"}>{look.count}</p>
          <p class={"font-mono text-note text-text-faint"}>{look.days}</p>
          <details class={"text-note"}>
            <summary class={"cursor-pointer text-text-faint hover:text-text"}>{look.fold}</summary>
            <ul class={"mt-tight flex flex-col gap-tight"}>
              <For
                each={Array.from(look.uses ?? []).map((__rrItem0, __rrIndex1) => ({
                  value: __rrItem0,
                  key: __rrIndex1,
                }))}
                keyed={(entry) => entry.key}
              >
                {(__rrItem0, __rrIndex1) => (
                  <>
                    <li class={"flex min-w-0 items-baseline gap-base text-text-faint"}>
                      <span class={"shrink-0"}>{__rrItem0().value.at}</span>
                      <span class={"min-w-0 truncate font-mono"}>{__rrItem0().value.who}</span>
                      {(() => {
                        const __rrBranch3 = createMemo(() => {
                          const __rrCaptured2 = __rrItem0().value;
                          return __rrCaptured2.part !== undefined
                            ? { yes: { __rrCaptured2 }, no: undefined }
                            : { yes: undefined, no: { __rrCaptured2 } };
                        });
                        return (
                          <Show when={__rrBranch3().yes}>
                            {(__rrActive4) => (
                              <>
                                <span class={"min-w-0 truncate font-mono"}>
                                  {__rrActive4().__rrCaptured2.part}
                                </span>
                              </>
                            )}
                          </Show>
                        );
                      })()}
                      <span class={"shrink-0"}>{__rrItem0().value.outcome}</span>
                    </li>
                  </>
                )}
              </For>
            </ul>
          </details>
        </>
      )}
    </>
  );
}
