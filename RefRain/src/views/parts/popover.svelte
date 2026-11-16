<script lang="ts">
  // This Source Code Form is subject to the terms of the Mozilla Public
  // License, v. 2.0. If a copy of the MPL was not distributed with this
  // file, You can obtain one at https://mozilla.org/MPL/2.0/.
  // Copyright (c) 2026 2youg1 and the sprawling contributors

  import type { Snippet } from "svelte";
  import { onDestroy, tick } from "svelte";
  import type { Attachment } from "svelte/attachments";

  import type { Key } from "../../core/lang";
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import { revealIn, roomOn, sideFor } from "./layer";
  import type { Side } from "./layer";
  import type { PopoverBinding, PopoverColumn, PopoverRow } from "./popover";
  import Look from "./popover.look.svelte";
  import { keysOf, listId, lookOf, rowId } from "./popover_wiring";
  import type { Hands, Keyed, Layout, Place, PopoverLook, PopoverView } from "./popover_wiring";

  interface Props {
    readonly label: Key;
    readonly columns: readonly PopoverColumn[];
    readonly layout?: Layout;
    readonly onApply: (column: PopoverColumn, row: PopoverRow) => void;
    readonly onClose: () => void;
    readonly bind?: ((binding: PopoverBinding) => void) | undefined;
    readonly onCursorChange?: ((rowId: string | null) => void) | undefined;
    readonly onCursorRow?: ((row: PopoverRow | null) => void) | undefined;
    readonly row?: Snippet<[PopoverRow]> | undefined;
    readonly header?: Snippet | undefined;
    readonly look?: Snippet<[PopoverLook]> | undefined;
  }

  const {
    label,
    columns,
    layout = "content",
    onApply,
    onClose,
    bind,
    onCursorChange,
    onCursorRow,
    row,
    header,
    look: drawing,
  }: Props = $props();

  const PREFERRED: Side = "above";

  const { lang } = ui();
  const seat = $props.id();

  let column = $state(0);
  let cursor = $state(0);
  let side = $state<Side>(PREFERRED);
  let room = $state<number | null>(null);
  let dialog = $state<HTMLDivElement | undefined>(undefined);
  const lists = $state<(HTMLUListElement | undefined)[]>([]);

  const place: Place = $derived.by(() => {
    const held = Math.min(column, Math.max(0, columns.length - 1));
    const rows = columns.at(held)?.rows.length ?? 0;
    return { column: held, cursor: Math.min(cursor, Math.max(0, rows - 1)) };
  });
  const here = $derived(columns.at(place.column)?.rows ?? []);
  const activeId = $derived(here.length === 0 ? null : rowId(seat, place.column, place.cursor));

  const revealCursor = (): void => {
    const list = lists.at(place.column);
    const item = activeId === null ? null : document.getElementById(activeId);
    if (list === undefined || item === null) return;
    revealIn(list, item);
  };

  const listHolds: Attachment<HTMLUListElement>[] = [];
  const hands: Hands = {
    place: (to) => {
      column = to.column;
      cursor = to.cursor;
      void tick().then(revealCursor);
    },
    apply: (pane, item) => {
      onApply(pane, item);
    },
    close: () => {
      onClose();
    },
    holdDialog: (node) => {
      dialog = node;
      return () => {
        dialog = undefined;
      };
    },
    holdList: (at) => {
      const kept = listHolds.at(at);
      if (kept !== undefined) return kept;
      const made: Attachment<HTMLUListElement> = (node) => {
        lists[at] = node;
        return () => {
          lists[at] = undefined;
        };
      };
      listHolds[at] = made;
      return made;
    },
  };

  const view: PopoverView = $derived({
    seat,
    layout,
    columns,
    place,
    holder: bind === undefined ? "list" : "caller",
    side,
    room,
    title: label,
    say: (key: Key) => say($lang, key),
  });
  const keys = (key: Keyed): boolean => keysOf(view, hands)(key);
  const look = $derived(lookOf(view, hands, { header, row }));

  const pointColumn = (columnId: string): void => {
    const at = columns.findIndex((pane) => pane.id === columnId);
    if (at < 0) return;
    column = at;
    cursor = Math.max(0, columns.at(at)?.rows.findIndex((each) => each.chosen === true) ?? 0);
  };

  let measured = false;
  $effect(() => {
    if (measured || dialog === undefined) return;
    measured = true;
    side = sideFor(dialog, PREFERRED);
    room = roomOn(dialog, side);
  });

  let opener: HTMLElement | null = null;
  let captured = false;
  $effect(() => {
    if (bind !== undefined) return;
    const target = lists.at(place.column);
    if (target === undefined) return;
    if (!captured) {
      captured = true;
      const held = document.activeElement;
      opener = held instanceof HTMLElement ? held : null;
    }
    target.focus({ preventScroll: true });
  });
  onDestroy(() => {
    if (bind === undefined) opener?.focus();
  });

  $effect(() => {
    bind?.({ keys, pointColumn, controls: columns.map((_pane, at) => listId(seat, at)) });
  });
  $effect(revealCursor);
  $effect(() => {
    onCursorChange?.(activeId);
  });
  $effect(() => {
    onCursorRow?.(here.at(place.cursor) ?? null);
  });
</script>

{#if drawing === undefined}
  <Look {...look} />
{:else}
  {@render drawing(look)}
{/if}
