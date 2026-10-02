<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // The line between the editor and the terminal (client/Spec.lean §7-11): an
  // APG Window Splitter that moves in whole lines of the code below it,
  // so the two regions always end on a line rather than through one. A
  // drag starts on the line and nowhere else, so selecting text in either
  // region never resizes them (roadmap §3-14).
</script>

<script lang="ts">
  import { say } from "../../core/lang";
  import { ui } from "../../ui";

  interface Props {
    // How many lines the editor region holds now, and the most and least
    // it may hold.
    readonly lines: number;
    readonly least: number;
    readonly most: number;
    // The id of the editor region this line sizes.
    readonly controls: string;
    // The pixel height of one line, which a drag is counted in.
    readonly step: number;
    readonly onLines: (lines: number) => void;
    readonly onReset: () => void;
  }

  const { lines, least, most, controls, step, onLines, onReset }: Props = $props();

  const lang = ui().lang;

  function clamped(next: number): number {
    return Math.min(most, Math.max(least, next));
  }

  function keys(event: KeyboardEvent): void {
    switch (event.key) {
      case "ArrowUp":
        onLines(clamped(lines - 1));
        break;
      case "ArrowDown":
        onLines(clamped(lines + 1));
        break;
      case "Home":
        onLines(least);
        break;
      case "End":
        onLines(most);
        break;
      case "Enter":
        onReset();
        break;
      default:
        return;
    }
    event.preventDefault();
  }

  let from: { readonly y: number; readonly lines: number } | null = null;

  function grab(event: PointerEvent): void {
    if (!(event.currentTarget instanceof HTMLElement)) return;
    event.currentTarget.setPointerCapture(event.pointerId);
    from = { y: event.clientY, lines };
  }

  function drag(event: PointerEvent): void {
    if (from === null) return;
    onLines(clamped(from.lines + Math.round((event.clientY - from.y) / step)));
  }
</script>

<!-- A focusable separator is the APG Window Splitter, a widget in
WAI-ARIA 1.2; the checker reads every separator as static. -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  role="separator"
  tabindex="0"
  aria-orientation="horizontal"
  aria-label={say($lang, "inspect_split")}
  aria-controls={controls}
  aria-valuenow={lines}
  aria-valuemin={least}
  aria-valuemax={most}
  class="relative h-[calc(var(--spacing-baseline)+1px)] shrink-0 cursor-row-resize touch-none border-y border-edge bg-chrome after:absolute after:top-[3px] after:left-1/2 after:h-hair after:w-section after:-translate-x-1/2 after:rounded-pill after:bg-edge-panel after:content-[''] hover:after:bg-edge-input focus-visible:after:bg-accent"
  onkeydown={keys}
  onpointerdown={grab}
  onpointermove={drag}
  onpointerup={() => {
    from = null;
  }}
  onpointercancel={() => {
    from = null;
  }}
></div>
