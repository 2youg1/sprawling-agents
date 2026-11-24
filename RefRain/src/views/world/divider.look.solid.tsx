// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { DividerLook } from "./divider";

export type PortProps = DividerLook;

export default function WorldDividerLook(look: DividerLook) {
  return (
    <>
      <div
        {...(() => {
          const {
            onkeydown: __rrArg0,
            ondblclick: __rrArg1,
            onpointerdown: __rrArg2,
            onpointermove: __rrArg3,
            onpointerup: __rrArg4,
            onpointercancel: __rrArg5,
            ...__rrWire
          } = look.wire;
          return {
            ...__rrWire,
            onKeyDown: __rrArg0,
            onDblClick: __rrArg1,
            onPointerDown: __rrArg2,
            onPointerMove: __rrArg3,
            onPointerUp: __rrArg4,
            onPointerCancel: __rrArg5,
          };
        })()}
        class={[
          "relative flex flex-1 cursor-col-resize touch-none justify-center -outline-offset-2",
          "before:w-px before:transition-colors focus-visible:before:bg-accent",
          look.dragging
            ? "before:bg-accent before:ease-arrive"
            : "before:ease-leave hover:before:bg-edge-input hover:before:ease-arrive focus-visible:before:ease-arrive",
        ]}
      />
    </>
  );
}
