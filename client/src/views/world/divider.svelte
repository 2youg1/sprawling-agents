<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // One divider of the panorama workbench (client/Spec.lean §7K, §7-11): APG
  // Window Splitter over the gutter between two panes. A drag snaps the
  // edge to the nearest column line of the shell's grid; Left and Right
  // move it one column, Home and End to the narrowest and widest the
  // pane before it may be, and Enter or a double click put the workbench
  // back the way it ships. The panes never collapse, so Enter's
  // collapse-and-restore in the APG reads as a reset here.
  //
  // The lines it snaps to are the shell's own: the world layer stands on
  // them through `subgrid`, so the column a drag lands on is a line every
  // other region of the page stands on too, the two silver lines among
  // them. The columns are not equal (client D24), so the lines are read off
  // the shell's grid as the engine laid it out rather than counted.
  //
  // This file is the seat: it places the gutter on the grid, reads the
  // column lines and writes the workbench; the keys are `./divider.ts`,
  // and the line is whatever `./divider.look.svelte` draws.
  import { fill, say } from "../../core/lang";
  import { WORKBENCH, resized, widest } from "../../core/workbench";
  import type { Divider } from "../../core/workbench";
  import { ui } from "../../ui";
  import { dividerLookOf } from "./divider";
  import type { DividerLook } from "./divider";
  import Look from "./divider.look.svelte";

  interface Props {
    readonly divider: Divider;
    // The pane before the divider: its name in the person's language,
    // and its element's id.
    readonly label: string;
    readonly controls: string;
    // The grid column the pane after the divider starts in; the divider
    // stands in the gutter on that column's left.
    readonly column: number;
  }

  const { divider, label, controls, column }: Props = $props();

  const u = ui();
  const { lang } = u;
  const bench = u.prefs.workbench;

  // The grid line the pane before the divider starts on.
  const start = $derived(divider === 0 ? 1 : 1 + $bench[0].span);
  const span = $derived(divider === 0 ? $bench[0].span : $bench[1].span);

  function set(next: number): void {
    if (next !== span) u.prefs.setWorkbench(resized($bench, divider, next));
  }

  function reset(): void {
    u.prefs.setWorkbench(WORKBENCH);
  }

  // The grid line nearest the pointer, counted from one: the middle of
  // the gutter before each column, read off the shell's frame, whose
  // resolved columns are pixels where a subgrid's are only `subgrid`.
  function lineAt(grid: HTMLElement, clientX: number): number {
    const frame = grid.closest<HTMLElement>(".frame") ?? grid;
    const style = getComputedStyle(frame);
    const gap = Number.parseFloat(style.columnGap) || 0;
    let edge = frame.getBoundingClientRect().left + (Number.parseFloat(style.paddingLeft) || 0) - gap / 2;
    const lines = style.gridTemplateColumns.split(" ").map((track) => {
      const at = edge;
      edge += Number.parseFloat(track) + gap;
      return at;
    });
    const distances = [...lines, edge].map((at) => Math.abs(clientX - at));
    return distances.indexOf(Math.min(...distances)) + 1;
  }

  let dragging = $state(false);
  let gutter = $state<HTMLElement | undefined>(undefined);

  const look: DividerLook = $derived(
    dividerLookOf(
      {
        label: fill(say($lang, "world_divider"), { pane: label }),
        controls,
        span,
        widest: widest($bench, divider),
        dragging,
      },
      {
        set,
        reset,
        drag: (now) => {
          dragging = now;
        },
        follow: (clientX) => {
          const grid = gutter?.parentElement;
          if (grid === null || grid === undefined) return;
          set(lineAt(grid, clientX) - start);
        },
      },
    ),
  );
</script>

<!-- The gutter the divider stands in: the grid's column on the left of
the pane after it, the full height of the workbench. Where it stands is
this seat's; how the line in it is drawn is the look's. -->
<div bind:this={gutter} class="row-[1/3] -ml-gutter flex w-gutter justify-self-start" style:grid-column="{column} / span 1">
  <Look {...look} />
</div>
