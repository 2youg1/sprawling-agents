// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For, Show, createMemo } from "solid-js";
import Badge from "../parts/badge.solid";
import Button from "../parts/button.solid";
import type { PanelLook } from "./panel";

export type PortProps = PanelLook;

export default function CityPanelLook(look: PanelLook) {
  return (
    <>
      <div class={"flex flex-col gap-base"}>
        <div class={"flex items-baseline gap-snug"}>
          <h2 class={"min-w-0 flex-1 truncate text-heading font-heading"}>{look.name}</h2>
          <Button label={look.close.label} tone={"quiet"} onPress={look.close.onPress} />
        </div>
        {look.badges.length > 0 ? (
          <>
            <div class={"flex flex-wrap items-center gap-snug"}>
              <For
                each={Array.from(look.badges ?? []).map((__rrItem0, __rrIndex1) => ({
                  value: __rrItem0,
                  key: __rrItem0.key,
                }))}
                keyed={(entry) => entry.key}
              >
                {(__rrItem0) => (
                  <>
                    <Badge text={__rrItem0().value.text} weight={__rrItem0().value.weight} />
                  </>
                )}
              </For>
            </div>
          </>
        ) : undefined}
        <a
          {...look.enter.wire}
          class={
            "inline-flex h-control items-center self-start rounded-control bg-accent px-base text-label text-on-accent transition-colors ease-leave hover:bg-accent-hover hover:ease-arrive"
          }
        >
          {look.enter.label}
        </a>
        {look.pursuit !== null ? (
          <>
            <p class={"flex items-baseline gap-snug text-note"}>
              <span
                class={[
                  "inline-block size-dot shrink-0 rounded-pill",
                  look.pursuit.running ? "bg-accent" : "bg-mark",
                ]}
              />
              <span class={"min-w-0 flex-1 text-text-quiet"}>{look.pursuit.goal}</span>
            </p>
          </>
        ) : undefined}
        {look.problems.length > 0 ? (
          <>
            <ul
              class={
                "rounded-card border border-alert/40 px-base py-snug text-note text-text-quiet"
              }
            >
              <For
                each={Array.from(look.problems ?? []).map((__rrItem2, __rrIndex3) => ({
                  value: __rrItem2,
                  key: __rrItem2,
                }))}
                keyed={(entry) => entry.key}
              >
                {(__rrItem2) => (
                  <>
                    <li>{__rrItem2().value}</li>
                  </>
                )}
              </For>
            </ul>
          </>
        ) : undefined}
        {look.blocked.length > 0 ? (
          <>
            <ul class={"text-note text-text-quiet"}>
              <For
                each={Array.from(look.blocked ?? []).map((__rrItem4, __rrIndex5) => ({
                  value: __rrItem4,
                  key: __rrItem4.key,
                }))}
                keyed={(entry) => entry.key}
              >
                {(__rrItem4) => (
                  <>
                    <li class={"my-tight"}>
                      <span class={"text-alert"}>{__rrItem4().value.source}</span>
                      <span class={"text-text-faint"}>{__rrItem4().value.line}</span>
                    </li>
                  </>
                )}
              </For>
            </ul>
          </>
        ) : undefined}
        <section>
          <h3 class={"mb-tight text-note text-text-faint"}>{look.runs.heading}</h3>
          {look.runs.rows.length > 0 ? (
            <>
              <ul class={"text-note"}>
                <For
                  each={Array.from(look.runs.rows ?? []).map((__rrItem6, __rrIndex7) => ({
                    value: __rrItem6,
                    key: __rrItem6.key,
                  }))}
                  keyed={(entry) => entry.key}
                >
                  {(__rrItem6) => (
                    <>
                      <li class={"border-b border-edge py-snug"}>
                        <a {...__rrItem6().value.wire} class={"group flex items-center gap-snug"}>
                          <span
                            class={[
                              "inline-block size-dot shrink-0 rounded-pill",
                              __rrItem6().value.moving ? "bg-accent" : "bg-mark",
                            ]}
                          />
                          <span
                            class={
                              "min-w-0 flex-1 truncate text-text-quiet transition-colors ease-leave group-hover:text-text group-hover:ease-arrive"
                            }
                          >
                            {__rrItem6().value.title}
                          </span>
                          {(() => {
                            const __rrBranch9 = createMemo(() => {
                              const __rrCaptured8 = __rrItem6().value;
                              return __rrCaptured8.at !== null
                                ? { yes: { __rrCaptured8 }, no: undefined }
                                : { yes: undefined, no: { __rrCaptured8 } };
                            });
                            return (
                              <Show when={__rrBranch9().yes}>
                                {(__rrActive10) => (
                                  <>
                                    <span class={"shrink-0 text-text-faint"}>
                                      {__rrActive10().__rrCaptured8.at}
                                    </span>
                                  </>
                                )}
                              </Show>
                            );
                          })()}
                        </a>
                      </li>
                    </>
                  )}
                </For>
              </ul>
            </>
          ) : (
            <>
              <p class={"text-note text-text-faint"}>{look.runs.none}</p>
            </>
          )}
        </section>
      </div>
    </>
  );
}
