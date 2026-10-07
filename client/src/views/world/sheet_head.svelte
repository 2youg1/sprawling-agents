<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // The head of the world's sheet on one column (client/Spec.lean §4-52): the
  // back key at the top of the side the sheet came from, and one tab per
  // pane in the person's order, one pane shown at a time. The panes are
  // the workbench's own, mounted by `workspace.svelte` and laid out there,
  // so turning a phone or widening a window keeps them, their scroll and
  // their questions; that is why this is a tab list of its own and not
  // `parts/tabs.svelte`, which mounts each panel inside itself.
  //
  // APG Tabs with automatic activation, as `parts/tabs.svelte` reads it:
  // one stop on the way in, the arrows, Home and End once inside.
  //
  // This file is the seat: it holds the drawn tabs the focus moves
  // between; the keys are `./sheet_head.ts`, and `./sheet_head.look.svelte`
  // draws the key and the strip.
  import { say } from "../../core/lang";
  import type { Pane } from "../../core/workbench";
  import { ui } from "../../ui";
  import { leaveSheet } from "../sheets.svelte";
  import { drawnElements } from "./drawn";
  import { sheetHeadOf } from "./sheet_head";
  import type { SheetHeadLook } from "./sheet_head";
  import Look from "./sheet_head.look.svelte";

  interface Props {
    // The panes in the person's order, each with the name its column
    // carries on a wide shell.
    readonly tabs: readonly { readonly pane: Pane; readonly label: string }[];
    readonly shown: Pane;
    // The prefix of the ids the panes carry, `<prefix>-<pane>`; a tab is
    // `<prefix>-tab-<pane>`, which its pane is labelled by.
    readonly prefix: string;
    readonly onShow: (pane: Pane) => void;
  }

  const { tabs, shown, prefix, onShow }: Props = $props();
  const { lang } = ui();

  const drawn = drawnElements();

  const look: SheetHeadLook = $derived(
    sheetHeadOf(
      { tabs, shown, prefix, words: { back: say($lang, "world_back"), strip: say($lang, "world_panes") } },
      {
        leave: () => {
          leaveSheet("world");
        },
        show: onShow,
        focus: (pane) => drawn.get(pane)?.focus(),
        hold: drawn.hold,
      },
    ),
  );
</script>

<Look {...look} />
