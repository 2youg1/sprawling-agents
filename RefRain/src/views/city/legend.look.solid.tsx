// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { For, Show, createMemo } from "solid-js";
import type { LegendLook } from "./legend";

export type PortProps = LegendLook;

export default function CityLegendLook(look: LegendLook) {
  return (
    <>
      <details class={"text-note text-text-faint"}>
        <summary
          class={
            "cursor-pointer text-text-quiet transition-colors ease-leave hover:text-text hover:ease-arrive"
          }
        >
          {look.summary}
        </summary>
        <ul class={"mt-snug flex flex-wrap items-center gap-wide"}>
          <For
            each={Array.from(look.marks ?? []).map((__rrItem0, __rrIndex1) => ({
              value: __rrItem0,
              key: __rrItem0.mark,
            }))}
            keyed={(entry) => entry.key}
          >
            {(__rrItem0) => (
              <>
                <li class={"flex items-center gap-tight"}>
                  <svg viewBox={"0 0 16 16"} class={"size-glyph shrink-0"} aria-hidden={"true"}>
                    {(() => {
                      const __rrBranch3 = createMemo(() => {
                        const __rrCaptured2 = __rrItem0().value;
                        return __rrCaptured2.mark === "window"
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
                                  {(() => {
                                    const __rrBranch6 = createMemo(() => {
                                      const __rrCaptured5 = __rrActive4().__rrCaptured2;
                                      return __rrCaptured5.mark === "figure"
                                        ? { yes: { __rrCaptured5 }, no: undefined }
                                        : { yes: undefined, no: { __rrCaptured5 } };
                                    });
                                    return (
                                      <Show
                                        when={__rrBranch6().yes}
                                        fallback={
                                          <Show when={__rrBranch6().no}>
                                            {(__rrActive7) => (
                                              <>
                                                {(() => {
                                                  const __rrBranch9 = createMemo(() => {
                                                    const __rrCaptured8 =
                                                      __rrActive7().__rrCaptured5;
                                                    return __rrCaptured8.mark === "flag"
                                                      ? { yes: { __rrCaptured8 }, no: undefined }
                                                      : { yes: undefined, no: { __rrCaptured8 } };
                                                  });
                                                  return (
                                                    <Show
                                                      when={__rrBranch9().yes}
                                                      fallback={
                                                        <Show when={__rrBranch9().no}>
                                                          {(__rrActive10) => (
                                                            <>
                                                              {(() => {
                                                                const __rrBranch12 = createMemo(
                                                                  () => {
                                                                    const __rrCaptured11 =
                                                                      __rrActive10().__rrCaptured8;
                                                                    return __rrCaptured11.mark ===
                                                                      "lamp"
                                                                      ? {
                                                                          yes: { __rrCaptured11 },
                                                                          no: undefined,
                                                                        }
                                                                      : {
                                                                          yes: undefined,
                                                                          no: { __rrCaptured11 },
                                                                        };
                                                                  },
                                                                );
                                                                return (
                                                                  <Show
                                                                    when={__rrBranch12().yes}
                                                                    fallback={
                                                                      <Show
                                                                        when={__rrBranch12().no}
                                                                      >
                                                                        {(__rrActive13) => (
                                                                          <>
                                                                            {(() => {
                                                                              const __rrBranch15 =
                                                                                createMemo(() => {
                                                                                  const __rrCaptured14 =
                                                                                    __rrActive13()
                                                                                      .__rrCaptured11;
                                                                                  return __rrCaptured14.mark ===
                                                                                    "plinth"
                                                                                    ? {
                                                                                        yes: {
                                                                                          __rrCaptured14,
                                                                                        },
                                                                                        no: undefined,
                                                                                      }
                                                                                    : {
                                                                                        yes: undefined,
                                                                                        no: {
                                                                                          __rrCaptured14,
                                                                                        },
                                                                                      };
                                                                                });
                                                                              return (
                                                                                <Show
                                                                                  when={
                                                                                    __rrBranch15()
                                                                                      .yes
                                                                                  }
                                                                                >
                                                                                  {(
                                                                                    __rrActive16,
                                                                                  ) => (
                                                                                    <>
                                                                                      <rect
                                                                                        x={"1"}
                                                                                        y={"6"}
                                                                                        width={"14"}
                                                                                        height={"4"}
                                                                                        rx={"2"}
                                                                                        class={
                                                                                          "fill-drawn-line"
                                                                                        }
                                                                                      />
                                                                                      <rect
                                                                                        x={"1"}
                                                                                        y={"6"}
                                                                                        width={"8"}
                                                                                        height={"4"}
                                                                                        rx={"2"}
                                                                                        class={
                                                                                          "fill-accent"
                                                                                        }
                                                                                      />
                                                                                    </>
                                                                                  )}
                                                                                </Show>
                                                                              );
                                                                            })()}
                                                                          </>
                                                                        )}
                                                                      </Show>
                                                                    }
                                                                  >
                                                                    {(__rrActive13) => (
                                                                      <>
                                                                        <line
                                                                          x1={"8"}
                                                                          y1={"6"}
                                                                          x2={"8"}
                                                                          y2={"14"}
                                                                          class={
                                                                            "stroke-drawn-part"
                                                                          }
                                                                          stroke-width={"1.2"}
                                                                        />
                                                                        <circle
                                                                          cx={"8"}
                                                                          cy={"4.4"}
                                                                          r={"2.6"}
                                                                          class={"fill-alert"}
                                                                        />
                                                                      </>
                                                                    )}
                                                                  </Show>
                                                                );
                                                              })()}
                                                            </>
                                                          )}
                                                        </Show>
                                                      }
                                                    >
                                                      {(__rrActive10) => (
                                                        <>
                                                          <line
                                                            x1={"5"}
                                                            y1={"2"}
                                                            x2={"5"}
                                                            y2={"14"}
                                                            class={"stroke-drawn-stem"}
                                                            stroke-width={"1.2"}
                                                          />
                                                          <path
                                                            d={"M5 3 l7 1.8 l-7 2.2 z"}
                                                            class={"fill-accent"}
                                                          />
                                                        </>
                                                      )}
                                                    </Show>
                                                  );
                                                })()}
                                              </>
                                            )}
                                          </Show>
                                        }
                                      >
                                        {(__rrActive7) => (
                                          <>
                                            <circle
                                              cx={"8"}
                                              cy={"5"}
                                              r={"2.4"}
                                              class={"fill-drawn-figure"}
                                            />
                                            <path
                                              d={"M4.6 13 q3.4 -6 6.8 0 z"}
                                              class={"fill-drawn-figure"}
                                            />
                                          </>
                                        )}
                                      </Show>
                                    );
                                  })()}
                                </>
                              )}
                            </Show>
                          }
                        >
                          {(__rrActive4) => (
                            <>
                              <rect
                                x={"5"}
                                y={"3"}
                                width={"6"}
                                height={"8"}
                                rx={"1"}
                                class={"fill-accent-solid"}
                              />
                            </>
                          )}
                        </Show>
                      );
                    })()}
                  </svg>
                  <span>{__rrItem0().value.label}</span>
                </li>
              </>
            )}
          </For>
        </ul>
      </details>
    </>
  );
}
