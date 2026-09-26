<script lang="ts">
  // This Source Code Form is subject to the terms of the Mozilla Public
  // License, v. 2.0. If a copy of the MPL was not distributed with this
  // file, You can obtain one at https://mozilla.org/MPL/2.0/.
  // Copyright (c) 2026 2youg1 and the sprawling contributors

  // The one list that opens over something else: one column or three,
  // arrow keys to choose, Tab to change column, Enter to apply, Escape
  // to close. The combined selector beside the composer and the `/`
  // menu are the same list with different rows.
  //
  // Two triggers want two different things from the keyboard, and that
  // is the only fork in here. A button hands the keyboard over: the
  // popover takes focus and gives it back when it closes. A text box
  // somebody is still typing in cannot, so it takes the handler instead
  // through `bind` and forwards the keys it does not want itself. One
  // key table either way, which is the point.

  import type { Snippet } from "svelte";
  import { onDestroy } from "svelte";

  import type { Key } from "../../core/lang";
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import type { PopoverColumn, PopoverRow } from "./popover";

  interface Props {
    // The accessible name of the dialog, a lang.json key.
    readonly label: Key;
    readonly columns: readonly PopoverColumn[];
    readonly onApply: (column: PopoverColumn, row: PopoverRow) => void;
    readonly onClose: () => void;
    // Bind mode: the caller's text box keeps the focus and forwards the
    // keys it does not want through the handler handed over here. The
    // handler answers whether the popover used the key, so the caller
    // knows whether to let the character through.
    readonly bind?: ((keys: (event: KeyboardEvent) => boolean) => void) | undefined;
    // The cursor row's DOM id whenever the cursor lands on a row, `null`
    // when the list holds none. Bind mode writes it as
    // `aria-activedescendant` on the caller's own focused text box - the
    // unfocused list must never carry it, or the change reaches no
    // screen reader at all.
    readonly onCursorChange?: ((rowId: string | null) => void) | undefined;
    // Where the data is not enough: renders one row's body in place of
    // the label and the secondary cell.
    readonly row?: Snippet<[PopoverRow]> | undefined;
  }

  const {
    label,
    columns,
    onApply,
    onClose,
    bind,
    onCursorChange,
    row,
  }: Props = $props();

  const { lang } = ui();
  // Unique within the document, so `aria-activedescendant` points at one
  // row and not at every popover that ever opened.
  const seat = $props.id();
  const rowSeat = (at: number, index: number): string =>
    `${seat}-r${String(at)}-${String(index)}`;

  let column = $state(0);
  let cursor = $state(0);
  const lists = $state<(HTMLUListElement | undefined)[]>([]);

  // A list that shrank under a cursor - somebody typed another letter -
  // leaves the raw cursor on a row that is no longer there, so both
  // positions are clamped where they are read rather than repaired
  // where they are written.
  const heldColumn = $derived(
    Math.min(column, Math.max(0, columns.length - 1)),
  );
  const here = $derived(columns.at(heldColumn));
  const rows = $derived(here?.rows ?? []);
  const heldRow = $derived(Math.min(cursor, Math.max(0, rows.length - 1)));
  // The cursor row's element id, or `null` on an empty list.
  const activeId = $derived(
    rows.length === 0 ? null : rowSeat(heldColumn, heldRow),
  );

  // Where the cursor is: the arrows walk it within one column, and the
  // pointer picks it up wherever it lands - whichever row the pointer
  // is over holds it. It never marks what is applied; `chosen` does.
  const move = (by: number): void => {
    if (rows.length === 0) return;
    cursor = Math.min(rows.length - 1, Math.max(0, heldRow + by));
  };
  const step = (by: number): void => {
    const count = columns.length;
    if (count < 2) return;
    column = (heldColumn + by + count) % count;
    cursor = 0;
  };
  const apply = (): void => {
    const pane = here;
    const item = rows.at(heldRow);
    if (pane === undefined || item === undefined) return;
    onApply(pane, item);
  };

  // Answers whether the popover used the key, so a text box that
  // forwards its keys knows whether to let the character through. Tab
  // is claimed here in both modes: a column change is what Tab means
  // inside this dialog, and a caller that wants Tab for something else
  // takes it before forwarding, as the composer's completion does.
  const keys = (event: KeyboardEvent): boolean => {
    if (event.key === "ArrowDown") {
      move(1);
      return true;
    }
    if (event.key === "ArrowUp") {
      move(-1);
      return true;
    }
    if (event.key === "Home") {
      cursor = 0;
      return true;
    }
    if (event.key === "End") {
      cursor = Math.max(0, rows.length - 1);
      return true;
    }
    if (event.key === "Tab") {
      step(event.shiftKey ? -1 : 1);
      return true;
    }
    if (event.key === "Enter") {
      apply();
      return true;
    }
    if (event.key === "Escape") {
      onClose();
      return true;
    }
    return false;
  };
  const down = (event: KeyboardEvent): void => {
    if (keys(event)) event.preventDefault();
  };

  // What had the keyboard a moment ago is what gets it back: a person
  // who opened a menu and closed it is where they were, not at the top
  // of the page. The capture sits inside the one effect that takes
  // focus, so it cannot race with it - and bind mode never takes focus,
  // so it never gives any back.
  let opener: HTMLElement | null = null;
  let captured = false;
  $effect(() => {
    if (bind !== undefined) return;
    const target = lists.at(heldColumn);
    if (target === undefined) return;
    if (!captured) {
      captured = true;
      const held = document.activeElement;
      opener = held instanceof HTMLElement ? held : null;
    }
    // The column that holds the cursor is the one that holds the focus.
    target.focus();
  });
  onDestroy(() => {
    if (bind === undefined) opener?.focus();
  });

  $effect(() => {
    bind?.(keys);
  });
  $effect(() => {
    onCursorChange?.(activeId);
  });
</script>

<!-- No stacking number on the panel below: a positioned box is painted
     after every box that is not positioned, so the list already covers
     the text box and the buttons it opens over. It rises into place
     through the theme's one entrance for a popover, which falls back to
     a cut under `prefers-reduced-motion`. -->
<div
  class="absolute bottom-full left-0 mb-snug w-max min-w-full max-w-full rounded-panel border border-edge-panel bg-raised p-snug shadow-float rise"
  role="dialog"
  aria-label={say($lang, label)}
>
  <div class="flex gap-snug">
    {#each columns as pane, at (pane.id)}
      <div class="flex min-w-0 flex-col">
        <div class="mb-tight px-snug text-note text-text-faint">
          {say($lang, pane.label)}
        </div>
        <ul
          bind:this={lists[at]}
          class="max-h-palette overflow-y-auto"
          role="listbox"
          aria-label={say($lang, pane.label)}
          tabindex={bind === undefined && at === heldColumn ? 0 : -1}
          aria-activedescendant={at === heldColumn && bind === undefined
            ? activeId
            : null}
          onkeydown={down}
        >
          {#each pane.rows as item, index (item.id)}
            <!-- svelte-ignore a11y_click_events_have_key_events (the key table lives on the listbox, which holds the focus and names this row through aria-activedescendant; a pointer may still land on a row directly) -->
            <li
              id={rowSeat(at, index)}
              role="option"
              aria-selected={item.chosen === true}
              class={[
                "flex cursor-pointer items-center justify-between gap-snug rounded-control px-snug py-tight text-body",
                at === heldColumn && index === heldRow ? "bg-raised-hover" : "",
                item.chosen === true ? "text-text" : "text-text-quiet",
              ]}
              onmouseenter={() => {
                column = at;
                cursor = index;
              }}
              onclick={() => {
                onApply(pane, item);
              }}
            >
              {#if row}
                {@render row(item)}
              {:else}
                <!-- The name never shrinks; the secondary cell does.
                     Both are capped rather than one holding its width
                     against the other, so a one-word hint beside a long
                     label survives whole, and letting the long secondary
                     cell take the row's width is what used to paint
                     `high` as `hi…`. -->
                <span class="min-w-0 max-w-[24ch] truncate">
                  {#if item.chosen === true}<span
                      class="mr-tight inline-block size-dot rounded-pill bg-accent align-middle"
                      aria-hidden="true"
                    ></span>{/if}
                  {item.label}
                </span>
                {#if item.secondary !== undefined}
                  <span
                    class="min-w-0 max-w-[20ch] line-clamp-2 text-note text-text-faint"
                    >{item.secondary}</span
                  >
                {/if}
              {/if}
            </li>
          {/each}
        </ul>
      </div>
    {/each}
  </div>
</div>
