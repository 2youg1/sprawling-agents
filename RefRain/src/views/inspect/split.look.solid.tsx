// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { onCleanup } from "solid-js";
import type { SplitLook } from "./split";
import "./split.solid.css";

export default function SplitLookView(look: SplitLook) {
  return (
    <div
      class="split inspect-resize relative shrink-0 cursor-row-resize touch-none border-y border-edge bg-chrome"
      ref={(node) => {
        const cleanup = look.hold(node);
        if (cleanup) onCleanup(cleanup);
      }}
      role={look.wire.role}
      tabindex={look.wire.tabindex}
      aria-orientation={look.wire["aria-orientation"]}
      aria-label={look.wire["aria-label"]}
      aria-controls={look.wire["aria-controls"]}
      aria-valuenow={look.wire["aria-valuenow"]}
      aria-valuemin={look.wire["aria-valuemin"]}
      aria-valuemax={look.wire["aria-valuemax"]}
      onKeyDown={(event) => look.wire.onkeydown(event)}
      onPointerDown={(event) => look.wire.onpointerdown(event)}
      onPointerMove={(event) => look.wire.onpointermove(event)}
      onPointerUp={() => look.wire.onpointerup()}
      onPointerCancel={() => look.wire.onpointercancel()}
      onLostPointerCapture={() => look.wire.onpointercancel()}
    />
  );
}
