// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { SourceToggleLook } from "./source_toggle";

export type PortProps = SourceToggleLook;

export default function BuildingSourceToggleLook(look: SourceToggleLook) {
  return (
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
        class={[
          "h-control-sm rounded-pill px-snug font-mono text-note transition-colors still:transition-none",
          look.pressed
            ? "bg-raised text-text ease-arrive"
            : "text-text-faint ease-leave hover:bg-raised hover:text-text-quiet",
        ]}
      >
        {look.label}
      </button>
    </>
  );
}
