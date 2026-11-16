<!--
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors
-->

<script lang="ts" module>
</script>

<script lang="ts">
  import { tick } from "svelte";
  import type { Attachment } from "svelte/attachments";

  import { fill, say } from "../../core/lang";
  import { ui } from "../../ui";
  import { holdsDraft } from "../editor/session.svelte";
  import { itemKey, sameItem, type RightItem } from "./open.svelte";
  import type { Region, Tab } from "./reading";
  import { lookOf, type StripLook, type StripTab } from "./strip";
  import Look from "./strip.look.svelte";

  interface Props {
    readonly tabs: readonly Tab[];
    readonly front: RightItem | null;

    readonly panels: Readonly<Record<Region, string>>;
    readonly onPick: (item: RightItem) => void;
    readonly onClose: (item: RightItem) => void;
    readonly onCloseAll: () => void;

    readonly linkOf: (item: RightItem) => string | null;
  }

  const { tabs, front, panels, onPick, onClose, onCloseAll, linkOf }: Props = $props();

  const lang = ui().lang;

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
    tabs.map((tab): StripTab => ({
      key: itemKey(tab.item),
      label: tab.label,
      terminal: tab.region === "terminal",
      unsaved: tab.item.kind === "document" && holdsDraft(tab.item),
      front: sameItem(front, tab.item),
      href: linkOf(tab.item),
      controls: panels[tab.region],
    })),
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
