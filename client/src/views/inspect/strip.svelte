<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
  // The inspector's tab strip (docs/frontend-method.md §7F, client/Spec.lean §7-11): one tab per open
  // item, in the order they were opened, the one in front drawn on the
  // page's own fill so it reads as the sheet the region below is cut
  // from; a terminal's tab carries the terminal mark, and a document whose
  // RefRain session holds words the city has not taken carries an alert
  // dot (7F). At the right end, the key that closes the whole inspector.
  //
  // **A tab is a link** to its item beside this conversation (4-63), so
  // the browser's own "copy link" and "open in a new tab" read the
  // locator; a press only brings the tab forward and leaves the address
  // bar alone (client D41). An item with no spelling there has no `href`.
  //
  // An APG Tabs pattern with a roving tab stop: the strip is one stop,
  // ←/→ walk it and bring each tab forward, Home/End go to either end,
  // and Delete closes the tab under the focus. Each tab also has its own
  // close mark for a pointer; it is not a second Tab stop, because Delete
  // is that action's key.
  //
  // This file is the seat (client D95): it resolves each open item into
  // the words and facts the strip shows, holds the drawn tabs to move the
  // focus between them, and draws whatever `./strip.look.svelte` is; the
  // key table and the wire bags are `./strip.ts`'s.
</script>

<script lang="ts">
  import { tick } from "svelte";
  import type { Attachment } from "svelte/attachments";

  import { fill, say } from "../../core/lang";
  import { ui } from "../../ui";
  import { holdsDraft } from "../refrain/session.svelte";
  import { itemKey, sameItem, type RightItem } from "./open.svelte";
  import type { Region, Tab } from "./reading";
  import { lookOf, type StripLook, type StripTab } from "./strip";
  import Look from "./strip.look.svelte";

  interface Props {
    readonly tabs: readonly Tab[];
    readonly front: RightItem | null;
    // Which region's panel each kind of tab controls.
    readonly panels: Readonly<Record<Region, string>>;
    readonly onPick: (item: RightItem) => void;
    readonly onClose: (item: RightItem) => void;
    readonly onCloseAll: () => void;
    // The address-bar locator of an item, `null` for one the address bar
    // has no spelling for.
    readonly linkOf: (item: RightItem) => string | null;
  }

  const { tabs, front, panels, onPick, onClose, onCloseAll, linkOf }: Props = $props();

  const lang = ui().lang;

  // Two registries no draw reads, so plain records rather than reactive
  // maps: the derived look hands out the attachments that write them.
  const drawn: Record<string, HTMLElement | undefined> = {};
  const holds: Record<string, Attachment<HTMLElement> | undefined> = {};

  function hold(key: string): Attachment<HTMLElement> {
    const kept = holds[key];
    if (kept !== undefined) return kept;
    const made: Attachment<HTMLElement> = (node) => {
      drawn[key] = node;
      return () => {
        if (drawn[key] === node) drawn[key] = undefined;
      };
    };
    holds[key] = made;
    return made;
  }

  // The focus lands on the tab at that place in the strip as it is
  // drawn after the change, which a Delete has just shortened.
  async function focusAt(at: number): Promise<void> {
    await tick();
    const tab = tabs[at];
    if (tab !== undefined) drawn[itemKey(tab.item)]?.focus();
  }

  function itemAt(at: number, then: (item: RightItem) => void): void {
    const tab = tabs[at];
    if (tab !== undefined) then(tab.item);
  }

  const shown = $derived(
    tabs.map(
      (tab): StripTab => ({
        key: itemKey(tab.item),
        label: tab.label,
        terminal: tab.region === "terminal",
        unsaved: tab.item.kind === "document" && holdsDraft(tab.item),
        front: sameItem(front, tab.item),
        href: linkOf(tab.item),
        controls: panels[tab.region],
      }),
    ),
  );

  const look: StripLook = $derived(
    lookOf(
      shown,
      {
        tabs: say($lang, "inspect_tabs"),
        unsaved: say($lang, "inspect_unsaved"),
        closeAll: say($lang, "inspect_close"),
        closeItem: (name) => fill(say($lang, "inspect_close_item"), { name }),
      },
      {
        pick: (at) => {
          itemAt(at, onPick);
        },
        close: (at) => {
          itemAt(at, onClose);
        },
        closeAll: onCloseAll,
        focus: (at) => {
          void focusAt(at);
        },
        hold,
      },
    ),
  );
</script>

<Look {...look} />
