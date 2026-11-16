<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  import type { Stands } from "./sheet";

  // Open and closed are one property with two displays, so the opening
  // and the closing are the same transition read in two directions:
  // the closed state leaves, the open state arrives. `display` and
  // `overlay` are discrete properties: without `transition-discrete`
  // the box would vanish on the first frame of the closing and take its
  // fade with it. A shadowed face draws no border
  // (docs/frontend-method.md §4-34).
  //
  // **The scrim is the dialog's own backdrop, dimmed rather than
  // washed.** `::backdrop` reaches the page's colour tokens only where
  // an engine inherits custom properties into it, and a token that
  // fails to resolve leaves `background-color` at its initial value - a
  // modal with no scrim at all, on an engine nobody tested. A
  // brightness filter asks the platform for no colour, so it darkens
  // the page under both lightings and cannot fail quietly.
  const FACE =
    "m-auto hidden w-full max-w-measure flex-col gap-base rounded-panel bg-raised p-pane opacity-0 " +
    "shadow-sheet transition-[opacity,display,overlay] transition-discrete duration-panel ease-leave " +
    "open:flex open:opacity-100 open:ease-arrive starting:open:opacity-0 still:transition-none " +
    "backdrop:bg-transparent backdrop:backdrop-brightness-50";

  // A specimen's `<dialog open>` is not modal, so the platform places it
  // `absolute`; `static` puts it back in the flow of the fold it is in.
  const STANDS: Record<Stands, string> = {
    centre: "",
    "below-top": "mt-section",
    "in-flow": "mt-section static",
  };
</script>

<script lang="ts">
  import type { SheetLook } from "./sheet";

  const { wire, stands, children }: SheetLook = $props();
</script>

<dialog {...wire} class={[FACE, STANDS[stands]]}>
  {@render children()}
</dialog>
