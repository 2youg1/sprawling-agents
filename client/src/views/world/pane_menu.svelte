<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // A workbench pane's label, which is also the menu that moves the pane
  // (client/Spec.lean §7K, §7-11): APG Menu Button, its keys in
  // `./menu.ts`. The order is the person's and is kept in this browser
  // (`core/workbench.ts`, `prefs.ts`).
  //
  // This file is the seat (client/Spec.lean §7K, the world layer's
  // drawing): it owns whether the menu is open and the elements the
  // focus moves between, and draws whatever `./pane_menu.look.svelte` is.
  import { tick } from "svelte";

  import { fill, say } from "../../core/lang";
  import { moved } from "../../core/workbench";
  import type { Pane, Side } from "../../core/workbench";
  import { ui } from "../../ui";
  import { drawnElements } from "./drawn";
  import { paneMenuLookOf, possible } from "./pane_menu";
  import type { Arranged, PaneMenuLook } from "./pane_menu";
  import Look from "./pane_menu.look.svelte";

  interface Props {
    readonly pane: Pane;
    // The pane's name, already in the person's language.
    readonly label: string;
    readonly arranged: Arranged;
  }

  const { pane, label, arranged }: Props = $props();

  const u = ui();
  const { lang } = u;
  const bench = u.prefs.workbench;
  const uid = $props.id();

  let open = $state(false);

  const drawn = drawnElements();

  const SIDES: readonly Side[] = ["left", "right"];
  function live(): HTMLElement[] {
    return SIDES.filter((side) => possible($bench, pane, side)).flatMap((side) => drawn.get(side) ?? []);
  }

  function show(): void {
    open = true;
    queueMicrotask(() => live()[0]?.focus());
  }

  // The focus goes back to the label once the page has redrawn: a move
  // re-seats the pane in the grid, and an element moved in the document
  // loses the focus it held.
  function close(): void {
    open = false;
    void tick().then(() => drawn.get("trigger")?.focus());
  }

  const look: PaneMenuLook = $derived(
    paneMenuLookOf(
      { pane, bench: $bench, arranged, open, uid },
      {
        label,
        arrange: fill(say($lang, "world_arrange"), { pane: label }),
        left: say($lang, "world_move_left"),
        right: say($lang, "world_move_right"),
      },
      {
        live,
        show,
        close,
        leave: () => {
          open = false;
        },
        move: (side) => {
          u.prefs.setWorkbench(moved($bench, pane, side));
          close();
        },
        hold: drawn.hold,
      },
    ),
  );
</script>

<Look {...look} />
