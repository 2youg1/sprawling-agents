// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For, Show, createMemo } from "solid-js";
import Button from "../parts/button.solid";
import type { CardAction, PromptsLook } from "./prompts";

export type PortProps = PromptsLook;

export default function BuildingPromptsLook(look: PromptsLook) {
  function actionKey(action: CardAction): string {
    return action.kind === "open" ? "open" : action.step;
  }
  return (
    <>
      <div class={"flex min-w-0 flex-col gap-wide"}>
        {look.rooms.length > 1 ? (
          <>
            <div
              class={"flex flex-wrap items-center gap-snug"}
              role={"group"}
              aria-label={look.roomsLabel}
            >
              <For
                each={Array.from(look.rooms ?? []).map((__rrItem0, __rrIndex1) => ({
                  value: __rrItem0,
                  key: __rrItem0.label,
                }))}
                keyed={(entry) => entry.key}
              >
                {(__rrItem0) => (
                  <>
                    <Button
                      label={__rrItem0().value.label}
                      tone={__rrItem0().value.current ? "secondary" : "quiet"}
                      onPress={__rrItem0().value.onPress}
                    />
                  </>
                )}
              </For>
            </div>
          </>
        ) : undefined}
        <div class={"flex min-w-0 flex-col gap-snug"}>
          <p class={"text-note text-text-quiet"}>{look.heading}</p>
          {look.slots.length > 0 ? (
            <>
              <ol class={"flex flex-wrap gap-x-wide gap-y-snug text-note"}>
                <For
                  each={Array.from(look.slots ?? []).map((__rrItem2, __rrIndex3) => ({
                    value: __rrItem2,
                    key: __rrItem2.key,
                  }))}
                  keyed={(entry) => entry.key}
                >
                  {(__rrItem2) => (
                    <>
                      <li class={"flex items-baseline gap-snug"}>
                        <span class={"text-text-faint"}>{__rrItem2().value.slot}</span>
                        <span class={"figure"}>{__rrItem2().value.size}</span>
                      </li>
                    </>
                  )}
                </For>
              </ol>
            </>
          ) : undefined}
          {look.run !== undefined ? (
            <>
              <a
                class={"w-fit text-note text-accent underline-offset-2 hover:underline"}
                href={look.run.href}
              >
                {look.run.label}
              </a>
            </>
          ) : undefined}
          <p class={"text-note text-text-faint"}>{look.note}</p>
        </div>
        <ul class={"flex min-w-0 flex-col gap-snug"} aria-label={look.documentsLabel}>
          <For
            each={Array.from(look.cards ?? []).map((__rrItem4, __rrIndex5) => ({
              value: __rrItem4,
              key: __rrItem4.key,
            }))}
            keyed={(entry) => entry.key}
          >
            {(__rrItem4) => (
              <>
                <li
                  class={"flex min-w-0 flex-col gap-tight rounded-panel bg-raised p-snug text-note"}
                >
                  <span class={"flex min-w-0 items-baseline gap-base"}>
                    <span
                      class={[
                        "shrink-0 font-mono",
                        __rrItem4().value.sent ? "text-text" : "text-text-faint",
                      ]}
                    >
                      {__rrItem4().value.title}
                    </span>
                    <span class={"min-w-0 flex-1 truncate text-text-faint"}>
                      {__rrItem4().value.about}
                    </span>
                    <span class={"shrink-0 text-text-faint"}>{__rrItem4().value.layer}</span>
                  </span>
                  <span class={"min-w-0 truncate font-mono text-text-faint"}>
                    {__rrItem4().value.at}
                  </span>
                  <span
                    class={[
                      "min-w-0 wrap-anywhere",
                      __rrItem4().value.sent ? "text-text-quiet" : "text-text-faint",
                    ]}
                  >
                    {__rrItem4().value.state}
                    {(() => {
                      const __rrBranch7 = createMemo(() => {
                        const __rrCaptured6 = __rrItem4().value;
                        return __rrCaptured6.standing !== undefined
                          ? { yes: { __rrCaptured6 }, no: undefined }
                          : { yes: undefined, no: { __rrCaptured6 } };
                      });
                      return (
                        <Show when={__rrBranch7().yes}>
                          {(__rrActive8) => (
                            <>
                              {" · "}
                              {__rrActive8().__rrCaptured6.standing}
                            </>
                          )}
                        </Show>
                      );
                    })()}
                  </span>
                  {(() => {
                    const __rrBranch10 = createMemo(() => {
                      const __rrCaptured9 = __rrItem4().value;
                      return __rrCaptured9.actions.length > 0
                        ? { yes: { __rrCaptured9 }, no: undefined }
                        : { yes: undefined, no: { __rrCaptured9 } };
                    });
                    return (
                      <Show when={__rrBranch10().yes}>
                        {(__rrActive11) => (
                          <>
                            <span class={"flex flex-wrap gap-snug"}>
                              <For
                                each={Array.from(__rrActive11().__rrCaptured9.actions ?? []).map(
                                  (__rrItem12, __rrIndex13) => ({
                                    value: __rrItem12,
                                    key: actionKey(__rrItem12),
                                  }),
                                )}
                                keyed={(entry) => entry.key}
                              >
                                {(__rrItem12) => (
                                  <>
                                    <Button
                                      label={look.actionLabel(__rrItem12().value)}
                                      name={look.actionName(
                                        __rrActive11().__rrCaptured9,
                                        __rrItem12().value,
                                      )}
                                      tone={"quiet"}
                                      onPress={() => {
                                        look.onAction(
                                          __rrActive11().__rrCaptured9,
                                          __rrItem12().value,
                                        );
                                      }}
                                    />
                                  </>
                                )}
                              </For>
                            </span>
                          </>
                        )}
                      </Show>
                    );
                  })()}
                </li>
              </>
            )}
          </For>
        </ul>
      </div>
    </>
  );
}
