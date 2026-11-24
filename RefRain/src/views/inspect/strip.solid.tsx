// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import { createMemo, flush } from "solid-js";
import { fill, say } from "../../core/lang";
import { storeValue } from "../../core/solid_store";
import { ui } from "../../ui";
import { holdsDraft, pendingDocuments } from "../editor/session";
import { itemKey, sameItem, type RightItem } from "./open";
import type { Region, Tab } from "./reading";
import { lookOf, type StripTab } from "./strip";
import Look from "./strip.look.solid";

export interface StripProps {
  readonly tabs: readonly Tab[];
  readonly front: RightItem | null;
  readonly panels: Readonly<Record<Region, string>>;
  readonly onPick: (item: RightItem) => void;
  readonly onClose: (item: RightItem) => void;
  readonly onCloseAll: () => void;
  readonly linkOf: (item: RightItem) => string | null;
}

export default function InspectorStrip(props: StripProps) {
  const lang = storeValue(ui().lang);
  const changes = storeValue(pendingDocuments);
  const drawn = new Map<string, HTMLElement>();
  function itemAt(at: number, take: (item: RightItem) => void): void {
    const tab = props.tabs[at];
    if (tab) take(tab.item);
  }
  const shown = createMemo(() =>
    props.tabs.map((tab): StripTab => ({
      key: itemKey(tab.item),
      label: tab.label,
      terminal: tab.region === "terminal",
      unsaved: tab.item.kind === "document" && holdsDraft(tab.item, changes()),
      front: sameItem(props.front, tab.item),
      href: props.linkOf(tab.item),
      controls: props.panels[tab.region],
    })),
  );
  const look = createMemo(() =>
    lookOf(
      shown(),
      {
        tabs: say(lang(), "inspect_tabs"),
        unsaved: say(lang(), "inspect_unsaved"),
        closeAll: say(lang(), "inspect_close"),
        closeItem: (name) => fill(say(lang(), "inspect_close_item"), { name }),
      },
      {
        pick: (at) => itemAt(at, props.onPick),
        close: (at) => itemAt(at, props.onClose),
        closeAll: () => props.onCloseAll(),
        focus: (at) => {
          queueMicrotask(() => {
            flush();
            itemAt(at, (item) => drawn.get(itemKey(item))?.focus());
          });
        },
        hold: (key) => (element) => {
          drawn.set(key, element);
          return () => {
            if (drawn.get(key) === element) drawn.delete(key);
          };
        },
      },
    ),
  );
  return <Look {...look()} />;
}
