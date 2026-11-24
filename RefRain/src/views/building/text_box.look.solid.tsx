// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { TextBoxLook, TextBoxSize } from "./text_box";

export type PortProps = TextBoxLook;

export default function BuildingTextBoxLook(look: TextBoxLook) {
  const ROOM: Record<TextBoxSize, string> = {
    document: "min-h-palette leading-read",
    list: "min-h-[calc(var(--spacing-control)*3)]",
  };
  return (
    <>
      <textarea
        {...(() => {
          const { "aria-invalid": __rrArg0, oninput: __rrArg1, ...__rrWire } = look.wire;
          return {
            ...__rrWire,
            "aria-invalid":
              typeof __rrArg0 === "boolean" ? (__rrArg0 ? "true" : "false") : __rrArg0,
            onInput: __rrArg1,
          };
        })()}
        class={
          "w-full rounded-control border border-edge-input bg-raised px-base py-snug font-mono text-note text-text aria-invalid:border-alert " +
          String(ROOM[look.size])
        }
      />
    </>
  );
}
