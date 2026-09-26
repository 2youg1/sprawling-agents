<!-- This Source Code Form is subject to the terms of the Mozilla Public
License, v. 2.0. If a copy of the MPL was not distributed with this
file, You can obtain one at https://mozilla.org/MPL/2.0/.
Copyright (c) 2026 2youg1 and the sprawling contributors -->

<!-- One compact control under the box: which model answers, which room
hears it, how hard the model thinks, which mode the run works in.
`composer.ts` owns what a pill offers and what a pick means; this is the
shape all four are drawn in.

The trigger says the pill's name faintly and its value plainly, so four
pills of one shape still tell a reader which is which. The menu opens
above the box, because the box sits at the foot of the window, and it is
as wide as its longest row rather than as wide as the trigger: every row
is a short name over one line saying what it changes, and a row cut off
at the trigger's width is a row nobody can choose by reading it. A list
long enough to scroll gets a filter; a list of six does not. -->
<script lang="ts">
  import { untrack } from "svelte";

  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import Glyph from "../parts/glyph.svelte";
  import type { Pill } from "./composer";

  // How many rows a list holds before a filter earns its place.
  const FILTER_AFTER = 8;

  interface Props {
    readonly spec: Pill;
    // The gallery draws a menu open so it is measured; every screen
    // starts closed.
    readonly starts?: "open" | "closed";
  }

  const { spec, starts = "closed" }: Props = $props();

  const u = ui();
  const { lang } = u;
  const uid = $props.id();

  let open = $state(untrack(() => starts) === "open");
  let query = $state("");
  let at = $state(0);
  // Which edge the menu hangs from: the trigger's left edge, unless the
  // menu would then run past the window's right edge.
  let edge = $state<"left" | "right">("left");
  let trigger = $state<HTMLButtonElement | undefined>(undefined);
  let list = $state<HTMLUListElement | undefined>(undefined);
  let filter = $state<HTMLInputElement | undefined>(undefined);
  let root = $state<HTMLDivElement | undefined>(undefined);

  const filtered = $derived(spec.choices.length > FILTER_AFTER);
  const rows = $derived.by(() => {
    const needle = query.trim().toLowerCase();
    if (needle === "") return spec.choices;
    return spec.choices.filter((each) =>
      `${each.label} ${each.note ?? ""} ${each.value}`.toLowerCase().includes(needle),
    );
  });
  const cursor = $derived(Math.max(0, Math.min(at, rows.length - 1)));
  const chosen = $derived(spec.choices.find((each) => each.value === spec.value));

  $effect(() => {
    if (!open) return;
    const box = trigger?.getBoundingClientRect();
    if (box !== undefined) edge = box.left + 448 > window.innerWidth ? "right" : "left";
    const chosenAt = spec.choices.findIndex((each) => each.value === untrack(() => spec.value));
    at = Math.max(0, chosenAt);
    (filtered ? filter : list)?.focus({ preventScroll: true });
  });

  function close(focus: "opener" | "leave"): void {
    open = false;
    query = "";
    if (focus === "opener") trigger?.focus();
  }

  function take(value: string | undefined): void {
    if (value === undefined) return;
    spec.pick(value);
    close("opener");
  }

  function onKeydown(event: KeyboardEvent): void {
    const moves: Readonly<Record<string, () => number>> = {
      ArrowDown: () => cursor + 1,
      ArrowUp: () => cursor - 1,
      Home: () => 0,
      End: () => rows.length - 1,
    };
    const move = moves[event.key];
    if (move !== undefined) {
      event.preventDefault();
      at = move();
      document.getElementById(`${uid}-${String(Math.max(0, Math.min(at, rows.length - 1)))}`)?.scrollIntoView({
        block: "nearest",
      });
      return;
    }
    if (event.key === "Enter") {
      event.preventDefault();
      take(rows[cursor]?.value);
      return;
    }
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      close("opener");
      return;
    }
    if (event.key === "Tab") close("leave");
  }

  function onFocusout(event: FocusEvent): void {
    if (!open) return;
    const next = event.relatedTarget;
    if (next instanceof Node && root?.contains(next)) return;
    close("leave");
  }
</script>

<div class="relative" bind:this={root} onfocusout={onFocusout}>
  <button
    type="button"
    bind:this={trigger}
    class="inline-flex h-control-sm max-w-[16rem] min-w-0 items-center gap-tight rounded-pill px-snug text-note text-text-quiet hover:bg-raised-hover hover:text-text aria-expanded:bg-raised-hover aria-expanded:text-text"
    aria-label={`${spec.label}: ${chosen?.label ?? spec.placeholder}`}
    aria-haspopup="listbox"
    aria-expanded={open}
    aria-controls={open ? `${uid}-list` : undefined}
    onclick={() => {
      if (open) close("opener");
      else open = true;
    }}
  >
    <span class="shrink-0 text-text-faint">{spec.label}</span>
    <span class="truncate">{chosen?.label ?? spec.placeholder}</span>
    <Glyph name="chevron" size="sm" class="shrink-0 rotate-90 text-text-faint" />
  </button>
  {#if open}
    <div
      class={[
        "rise absolute bottom-full z-20 mb-tight flex w-max max-w-[min(28rem,calc(100vw-2rem))] min-w-[16rem] flex-col rounded-panel border border-edge-panel bg-raised p-tight shadow-float",
        edge === "left" ? "left-0" : "right-0",
      ]}
    >
      <p class="px-base pt-tight pb-snug text-note text-text-faint">{spec.about ?? spec.label}</p>
      {#if filtered}
        <input
          bind:this={filter}
          bind:value={query}
          class="mb-tight h-control w-full rounded-control bg-page px-base text-body text-text placeholder:text-text-faint"
          placeholder={spec.placeholder}
          aria-label={spec.label}
          role="combobox"
          aria-expanded={open}
          aria-controls="{uid}-list"
          aria-activedescendant={rows.length > 0 ? `${uid}-${String(cursor)}` : undefined}
          oninput={() => {
            at = 0;
          }}
          onkeydown={onKeydown}
        />
      {/if}
      <ul
        id="{uid}-list"
        bind:this={list}
        class="max-h-[min(70vh,36rem)] overflow-y-auto outline-none"
        role="listbox"
        tabindex={filtered ? -1 : 0}
        aria-label={spec.label}
        aria-activedescendant={filtered || rows.length === 0 ? undefined : `${uid}-${String(cursor)}`}
        onkeydown={onKeydown}
        onmousedown={(event) => {
          event.preventDefault();
        }}
      >
        {#each rows as row, index (row.value)}
          <!-- svelte-ignore a11y_click_events_have_key_events (the row is picked by pointer; the key table belongs to the list, which holds the focus) -->
          <li
            id="{uid}-{index}"
            role="option"
            aria-selected={row.value === spec.value}
            class={[
              "flex cursor-pointer items-start gap-snug rounded-control px-base py-snug",
              index === cursor ? "bg-raised-hover text-text" : "text-text-quiet",
            ]}
            onmouseenter={() => {
              at = index;
            }}
            onclick={() => {
              take(row.value);
            }}
          >
            <span class="flex min-w-0 flex-1 flex-col">
              <span class="text-body break-words">{row.label}</span>
              {#if row.note !== undefined && row.note !== ""}
                <span class="text-note break-words text-text-faint">{row.note}</span>
              {/if}
            </span>
            <span class="mt-tight size-glyph-sm shrink-0">
              {#if row.value === spec.value}
                <Glyph name="check" size="sm" class="text-text-quiet" />
              {/if}
            </span>
          </li>
        {:else}
          <li role="presentation" class="px-base py-snug text-note text-text-faint">
            {say($lang, "part_no_match")}
          </li>
        {/each}
      </ul>
    </div>
  {/if}
</div>
