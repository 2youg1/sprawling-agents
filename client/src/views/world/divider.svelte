<!-- This Source Code Form is subject to the terms of the Mozilla Public
     License, v. 2.0. If a copy of the MPL was not distributed with this
     file, You can obtain one at https://mozilla.org/MPL/2.0/.
     Copyright (c) 2026 2youg1 and the sprawling contributors -->

<script lang="ts">
  // One divider of the panorama workbench (client-SPEC 7K, 7-11): APG
  // Window Splitter over the gutter between two panes. A drag snaps the
  // edge to the nearest column line of the shell's grid; Left and Right
  // move it one column, Home and End to the narrowest and widest the
  // pane before it may be, and Enter or a double click put the workbench
  // back the way it ships. The panes never collapse, so Enter's
  // collapse-and-restore in the APG reads as a reset here.
  //
  // The grid it measures is its parent, the world layer, whose columns
  // are the shell's own twelve through `subgrid`, so the column a drag
  // lands on is a line every other region of the page stands on too.
  import { fill, say } from "../../core/lang";
  import { NARROWEST, WORKBENCH, resized, widest } from "../../core/workbench";
  import type { Divider } from "../../core/workbench";
  import { ui } from "../../ui";

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

  const COLUMNS = 12;
  // The grid line the pane before the divider starts on.
  const start = $derived(divider === 0 ? 1 : 1 + $bench[0].span);
  const span = $derived(divider === 0 ? $bench[0].span : $bench[1].span);

  function set(next: number): void {
    if (next !== span) u.prefs.setWorkbench(resized($bench, divider, next));
  }

  function reset(): void {
    u.prefs.setWorkbench(WORKBENCH);
  }

  // The grid line nearest the pointer, counted from one.
  function lineAt(grid: HTMLElement, clientX: number): number {
    const box = grid.getBoundingClientRect();
    const gap = Number.parseFloat(getComputedStyle(grid).columnGap) || 0;
    const step = (box.width + gap) / COLUMNS;
    return Math.round((clientX - box.left + gap / 2) / step) + 1;
  }

  let dragging = $state(false);

  function keys(event: KeyboardEvent): void {
    switch (event.key) {
      case "ArrowLeft":
        set(span - 1);
        break;
      case "ArrowRight":
        set(span + 1);
        break;
      case "Home":
        set(NARROWEST);
        break;
      case "End":
        set(widest($bench, divider));
        break;
      case "Enter":
        reset();
        break;
      default:
        return;
    }
    event.preventDefault();
  }
</script>

<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions (a focusable separator is the APG window splitter, an interactive widget that moves with the arrow keys) -->
<div
  role="separator"
  tabindex="0"
  aria-orientation="vertical"
  aria-label={fill(say($lang, "world_divider"), { pane: label })}
  aria-controls={controls}
  aria-valuenow={span}
  aria-valuemin={NARROWEST}
  aria-valuemax={widest($bench, divider)}
  class={[
    "relative row-[1/3] -ml-gutter flex w-gutter cursor-col-resize touch-none justify-center justify-self-start outline-offset-[-2px]",
    "before:w-px before:transition-colors hover:before:bg-edge-input focus-visible:before:bg-accent",
    dragging ? "before:bg-accent" : "",
  ]}
  style:grid-column="{column} / span 1"
  onkeydown={keys}
  ondblclick={reset}
  onpointerdown={(event) => {
    event.currentTarget.setPointerCapture(event.pointerId);
    dragging = true;
  }}
  onpointermove={(event) => {
    const grid = event.currentTarget.parentElement;
    if (!dragging || grid === null) return;
    set(lineAt(grid, event.clientX) - start);
  }}
  onpointerup={() => {
    dragging = false;
  }}
  onpointercancel={() => {
    dragging = false;
  }}
></div>
