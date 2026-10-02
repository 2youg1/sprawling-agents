<script lang="ts" generics="V extends string">
  // This Source Code Form is subject to the terms of the Mozilla Public
  // License, v. 2.0. If a copy of the MPL was not distributed with this
  // file, You can obtain one at https://mozilla.org/MPL/2.0/.
  // Copyright (c) 2026 2youg1 and the sprawling contributors

  // One exclusive choice among a few, drawn as a track with a slider on
  // it. It replaces the pill rows the settings pages grew one at a time,
  // each of which announced itself as a row of unrelated pressed buttons.
  //
  // Three things this control owes a person that a row of pills does not
  // give them. A screen reader hears one question with several answers,
  // because the track is a `radiogroup` and every cell reports whether it
  // is the chosen one. A keyboard crosses the whole control in two keys,
  // because only the chosen cell is a tab stop and the arrows move
  // between cells. And a cell that cannot be chosen says why - through
  // `Tip`, so the reason reaches a pointer, a keyboard and a touch screen
  // alike, and arrives before the click rather than as a refusal after
  // it.
  //
  // The key table is the APG Radio Group one (client-SPEC 7-4): the
  // arrows Right, Left, Down and Up all move to the next or previous
  // choosable cell and select it, wrapping once around, and Space
  // selects the focused cell. Selection follows focus, so a cell that
  // cannot be chosen is stepped over rather than landed on. Tab enters
  // and leaves the control at the one stop `tabStop` decides.
  //
  // The slider travels on `transform` over cells of one width, so the
  // move is a compositor job and no script measures anything. The one
  // stylesheet decides whether it travels at all.
  //
  // The words are the caller's: this file holds no prose.

  import Tip from "./tip.svelte";

  import { bands, nextStop, tabStop } from "./segmented";
  import type { Band, Choice, Group, SegmentedProps, Tone } from "./segmented";

  // The fill each tone paints the chosen cell with, spelled for
  // Tailwind to read out of this file as text.
  const FILL: Record<Tone, string> = {
    plain: "bg-raised-hover",
    alert: "bg-alert",
  };

  const { label, options, held, onPick, tone }: SegmentedProps<V> = $props();

  const runs: readonly Band<V>[] = $derived(bands(options));
  const stop: number = $derived(tabStop(options, held));

  // The drawn cells, keyed by their value, so `bind:this` keeps each
  // element with its cell when the list is edited rather than with the
  // position it happened to sit at.
  const cells: Record<string, HTMLButtonElement | undefined> = {};

  // Which tone fills a cell is decided by the group it states, falling
  // back to the control's own tone; the slider and the ink of the
  // chosen cell read that one answer, so the text is always set
  // against the fill actually under it.
  const toneOf = (group: Group | undefined): Tone => group?.tone ?? tone ?? "plain";

  // The ink a cell takes. The chosen cell on an `alert` fill is read
  // against a coloured solid and takes the page's own rung; on the
  // plain fill it is read against a surface one step up, and takes the
  // full text token the rest of the page is set in.
  const ink = (choice: Choice<V>): string => {
    if (choice.why !== undefined) return "aria-disabled:text-text-disabled";
    if (choice.value !== held) return "text-text-quiet hover:text-text";
    return toneOf(choice.group) === "alert" ? "text-on-accent" : "text-text";
  };

  // The chosen cell's position inside its band, or -1 when the chosen
  // cell sits in another band or nobody has chosen.
  const here = (band: Band<V>): number => band.cells.findIndex((choice) => choice.value === held);

  // Select the cell at `at` and take focus there. A cell that cannot be
  // chosen is never selected - not by a click, not by an arrow landing
  // back on it when nothing else can be chosen, not by Space.
  const choose = (at: number): void => {
    const choice = options[at];
    if (choice === undefined || choice.why !== undefined) return;
    onPick(choice.value);
    cells[choice.value]?.focus();
  };

  const move = (at: number, step: 1 | -1): void => {
    choose(nextStop(options, at, step));
  };

  // The whole APG Radio Group key table, on each cell. Enter reaches
  // `onclick` through the platform's button activation and needs no
  // line here; Space is spelled out because selection follows focus
  // and the key must select the focused cell without also firing that
  // activation twice.
  const travel = (event: KeyboardEvent, at: number): void => {
    if (event.key === "ArrowRight" || event.key === "ArrowDown") {
      event.preventDefault();
      move(at, 1);
      return;
    }
    if (event.key === "ArrowLeft" || event.key === "ArrowUp") {
      event.preventDefault();
      move(at, -1);
      return;
    }
    if (event.key === " ") {
      event.preventDefault();
      choose(at);
    }
  };
</script>

<div class="inline-flex items-end gap-snug" role="radiogroup" aria-label={label}>
  {#each runs as band (band.from)}
    {#if band.from > 0}
      <span aria-hidden="true" class="w-hair self-stretch bg-edge-panel"></span>
    {/if}
    <div class="flex flex-col gap-tight">
      {#if band.group}
        <span class="px-base text-note text-text-quiet">{band.group.label}</span>
      {/if}
      <div
        class="relative grid auto-cols-fr grid-flow-col rounded-pill bg-track"
        style:--cells={String(band.cells.length)}
        style:--held={String(Math.max(here(band), 0))}
      >
        {#if here(band) >= 0}
          <!-- The slider's geometry is written once because neither half
              depends on which band draws it: the width is one cell of
              however many the track holds, and the travel is that width
              times the cell that is chosen. The track states both numbers
              as custom properties and the slider inherits them, so a
              change of choice moves one value. -->
          <span
            aria-hidden="true"
            class={[
              "pointer-events-none absolute inset-y-0 left-0 rounded-pill transition-transform motion-reduce:transition-none",
              FILL[toneOf(band.group)],
            ]}
            style:width="calc(100% / var(--cells))"
            style:transform="translateX(calc(var(--held) * 100%))"
          ></span>
        {/if}
        {#each band.cells as choice, inBand (choice.value)}
          {#snippet cell(hint: string | undefined)}
            <button
              bind:this={cells[choice.value]}
              type="button"
              role="radio"
              aria-checked={choice.value === held}
              aria-disabled={choice.why !== undefined}
              aria-describedby={hint}
              tabindex={band.from + inBand === stop ? 0 : -1}
              class={[
                "relative w-full rounded-pill px-base py-tight text-label whitespace-nowrap",
                ink(choice),
              ]}
              onclick={() => {
                choose(band.from + inBand);
              }}
              onkeydown={(event) => {
                travel(event, band.from + inBand);
              }}
            >
              {choice.label}
            </button>
          {/snippet}
          <!-- One definition of the cell, drawn bare or inside its
              reason. `Tip` wraps the cell in a box of its own and hands
              the hint's id to its children snippet, which is the
              `aria-describedby` half of the contract (client-SPEC 7-4):
              the cell is named by its own text, and the hint says why it
              cannot be chosen. -->
          {#if choice.why !== undefined}
            <Tip text={choice.why}>
              {#snippet children(hint: string)}
                <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
                {@render cell(hint)}
              {/snippet}
            </Tip>
          {:else}
            <!-- eslint-disable-next-line @typescript-eslint/no-confusing-void-expression -->
            {@render cell(undefined)}
          {/if}
        {/each}
      </div>
    </div>
  {/each}
</div>
