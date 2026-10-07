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
  // This is the seat: it holds the cursor and the elements focus,
  // scrolling and measuring need, and draws whatever
  // `./popover.look.svelte` is. What each key does is decided in
  // `./popover_wiring`; which side the list opens on and how far it
  // scrolls is `./layer`, the rule the combobox follows too.
  //
  // Two triggers want two different things from the keyboard, and that
  // is the only fork in here. A button hands the keyboard over: the
  // popover takes focus and gives it back when it closes. A text box
  // somebody is still typing in cannot, so it takes the handler instead
  // through `bind` and forwards the keys it does not want itself. One
  // key table either way, which is the point.

  import type { Snippet } from "svelte";
  import { onDestroy, tick } from "svelte";
  import type { Attachment } from "svelte/attachments";

  import type { Key } from "../../core/lang";
  import { say } from "../../core/lang";
  import { ui } from "../../ui";
  import { revealIn, sideFor } from "./layer";
  import type { Side } from "./layer";
  import type { PopoverBinding, PopoverColumn, PopoverRow } from "./popover";
  import Look from "./popover.look.svelte";
  import { keysOf, listId, lookOf, rowId } from "./popover_wiring";
  import type { Hands, Keyed, Layout, Place, PopoverView } from "./popover_wiring";

  interface Props {
    // The accessible name of the dialog, a lang.json key.
    readonly label: Key;
    readonly columns: readonly PopoverColumn[];
    readonly layout?: Layout;
    readonly onApply: (column: PopoverColumn, row: PopoverRow) => void;
    readonly onClose: () => void;
    // Bind mode: the caller's text box keeps the focus and forwards the
    // keys it does not want through the binding handed over here. Its
    // listbox ids supply aria-controls, and the handler answers whether
    // the popover used the key, so the caller knows whether to let the
    // character through.
    readonly bind?: ((binding: PopoverBinding) => void) | undefined;
    // The cursor row's DOM id whenever the cursor lands on a row, `null`
    // when the list holds none. Bind mode writes it as
    // `aria-activedescendant` on the caller's own focused text box - the
    // unfocused list must never carry it, or the change reaches no
    // screen reader at all.
    readonly onCursorChange?: ((rowId: string | null) => void) | undefined;
    // The row the cursor is on, as data, for a caller whose own keys act
    // on it: the composer's Tab takes this row's verb into the box.
    readonly onCursorRow?: ((row: PopoverRow | null) => void) | undefined;
    // Where the data is not enough: renders one row's body in place of
    // the label and the secondary cell.
    readonly row?: Snippet<[PopoverRow]> | undefined;
    // An owned input or toolbar above the columns.
    readonly header?: Snippet | undefined;
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
  }: Props = $props();

  // A popover opens over the box or the button below it, where the
  // page has its room.
  const PREFERRED: Side = "above";

  const { lang } = ui();
  const seat = $props.id();

  let column = $state(0);
  let cursor = $state(0);
  let side = $state<Side>(PREFERRED);
  let dialog = $state<HTMLDivElement | undefined>(undefined);
  const lists = $state<(HTMLUListElement | undefined)[]>([]);

  // A list that shrank under a cursor - somebody typed another letter -
  // leaves the raw cursor on a row that is no longer there, so both
  // positions are clamped where they are read rather than repaired
  // where they are written.
  const place: Place = $derived.by(() => {
    const held = Math.min(column, Math.max(0, columns.length - 1));
    const rows = columns.at(held)?.rows.length ?? 0;
    return { column: held, cursor: Math.min(cursor, Math.max(0, rows - 1)) };
  });
  const here = $derived(columns.at(place.column)?.rows ?? []);
  // The cursor row's element id, or `null` on an empty list.
  const activeId = $derived(here.length === 0 ? null : rowId(seat, place.column, place.cursor));

  const revealCursor = (): void => {
    const list = lists.at(place.column);
    const item = activeId === null ? null : document.getElementById(activeId);
    if (list === undefined || item === null) return;
    revealIn(list, item);
  };

  // One attachment per list for the life of the seat, so a redraw does
  // not let go of an element and take it again.
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
    title: label,
    say: (key: Key) => say($lang, key),
  });
  // Read at the moment of the key rather than bound to one state, so a
  // caller holding it from `bind` is not handed a new one at every move.
  const keys = (key: Keyed): boolean => keysOf(view, hands)(key);
  const look = $derived(lookOf(view, hands, { header, row }));

  const pointColumn = (columnId: string): void => {
    const at = columns.findIndex((pane) => pane.id === columnId);
    if (at < 0) return;
    column = at;
    cursor = 0;
  };

  // Measured once, on the side it was first drawn on: a side that
  // changed with every letter typed into the composer would make the
  // list jump while the person reads it.
  let measured = false;
  $effect(() => {
    if (measured || dialog === undefined) return;
    measured = true;
    side = sideFor(dialog, PREFERRED);
  });

  // What had the keyboard a moment ago is what gets it back: a person
  // who opened a menu and closed it is where they were, not at the top
  // of the page. The capture sits inside the one effect that takes
  // focus, so it cannot race with it - and bind mode never takes focus,
  // so it never gives any back.
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
    // The column that holds the cursor is the one that holds the focus.
    // Without scrolling: the list opens beside what the person just
    // pressed, on the side that has room, and a list drawn open on load
    // must not move the page under the lists measured after it.
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

<Look {...look} />
