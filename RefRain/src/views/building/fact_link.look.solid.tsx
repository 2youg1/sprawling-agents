// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo } from "solid-js";
import type { FactInk, FactLinkLook } from "./fact_link";

export type PortProps = FactLinkLook;

export default function BuildingFactLinkLook(look: FactLinkLook) {
  const INK: Record<FactInk, string> = {
    text: "text-text",
    quiet: "text-text-quiet",
  };
  const paint = createMemo(() => [
    "transition-colors ease-leave hover:text-accent hover:ease-arrive focus-visible:text-accent still:transition-none",
    look.face === "figure" ? "figure" : "",
    INK[look.ink],
  ]);
  return (
    <>
      {"href" in look.wire ? (
        <>
          <a {...look.wire} class={paint()}>
            {look.label}
          </a>
        </>
      ) : (
        <>
          <button
            data-control=""
            {...(() => {
              const { "aria-expanded": __rrArg0, onclick: __rrArg1, ...__rrWire } = look.wire;
              return {
                ...__rrWire,
                "aria-expanded":
                  typeof __rrArg0 === "boolean" ? (__rrArg0 ? "true" : "false") : __rrArg0,
                onClick: __rrArg1,
              };
            })()}
            class={paint()}
          >
            {look.label}
          </button>
        </>
      )}
    </>
  );
}
