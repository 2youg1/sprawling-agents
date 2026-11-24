// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import "./text_key.look.solid.css";
import type { TextKeyLook } from "./text_key";

export type PortProps = TextKeyLook;

export default function InspectTextKeyLook(look: TextKeyLook) {
  const SHAPE =
    "text-key flex h-control-sm shrink-0 items-center rounded-control px-snug text-note text-text-faint hover:bg-raised hover:text-text aria-pressed:bg-raised aria-pressed:text-text";
  return (
    <>
      {"href" in look.wire ? (
        <>
          <a {...look.wire} class={SHAPE} data-rf-80faae5d8c60="">
            {look.label}
          </a>
        </>
      ) : (
        <>
          <button
            data-control=""
            {...(() => {
              const { "aria-pressed": __rrArg0, onclick: __rrArg1, ...__rrWire } = look.wire;
              return {
                ...__rrWire,
                "aria-pressed":
                  typeof __rrArg0 === "boolean" ? (__rrArg0 ? "true" : "false") : __rrArg0,
                onClick: __rrArg1,
              };
            })()}
            class={SHAPE}
            data-rf-80faae5d8c60=""
          >
            {look.label}
          </button>
        </>
      )}
    </>
  );
}
