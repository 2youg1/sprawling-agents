// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { Show } from "solid-js";
import Glyph from "./glyph.solid";
import type { CellLook } from "./segmented";

interface ThemePreviewLook {
  readonly cell: CellLook;
  readonly lighting: "light" | "dark" | undefined;
  readonly palette: "blue" | "graphite";
}

export default function ThemePreview(props: ThemePreviewLook) {
  return (
    <button
      {...props.cell.wire}
      data-control=""
      data-theme-preview={props.cell.state === "held" ? undefined : props.lighting}
      data-palette={props.palette}
      type="button"
      class="theme-preview relative flex min-h-touch min-w-0 items-center justify-between gap-snug rounded-control border border-edge-input bg-page px-base py-tight text-label text-text"
    >
      <span class="min-w-0 text-balance">{props.cell.label}</span>
      <span aria-hidden="true" class="flex size-glyph-sm shrink-0 items-center justify-center">
        <Show
          when={props.cell.state === "held"}
          fallback={<span class="size-dot rounded-pill bg-accent" />}
        >
          <Glyph name="check" size="sm" class="text-accent" />
        </Show>
      </span>
    </button>
  );
}
