// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// Copyright (c) 2026 2youg1 and the sprawling contributors

import type { Snippet } from "svelte";
import { createAttachmentKey } from "svelte/attachments";
import type { Attachment } from "svelte/attachments";

export interface Lens {
  readonly id: string;

  readonly label: string;
}

export interface TabsProps {
  readonly label: string;
  readonly lenses: readonly Lens[];

  readonly current: string;
  readonly onPick: (id: string) => void;

  readonly panel: Snippet<[Lens]>;

  readonly mark?: Snippet<[Lens]>;
}

export type Pressed = Pick<KeyboardEvent, "key" | "preventDefault">;

const HOLD = createAttachmentKey();

export interface ListWire {
  readonly role: "tablist";
  readonly "aria-label": string;
}

export interface TabWire {
  readonly role: "tab";
  readonly id: string;
  readonly "aria-controls": string;
  readonly "aria-selected": boolean;
  readonly tabindex: 0 | -1;
  readonly onclick: () => void;
  readonly onkeydown: (event: Pressed) => void;
  readonly [hold: symbol]: Attachment<HTMLElement>;
}

export interface PanelWire {
  readonly role: "tabpanel";
  readonly id: string;
  readonly "aria-labelledby": string;
  readonly hidden: boolean;

  readonly tabindex: 0 | undefined;

  readonly [watch: symbol]: Attachment<HTMLElement>;
}

export interface TabLook {
  readonly lens: Lens;

  readonly current: boolean;
  readonly wire: TabWire;
}

export interface PanelLook {
  readonly lens: Lens;

  readonly shown: boolean;
  readonly wire: PanelWire;
}

export interface TabsLook {
  readonly list: ListWire;
  readonly tabs: readonly TabLook[];
  readonly panels: readonly PanelLook[];
  readonly panel: Snippet<[Lens]>;
  readonly mark: Snippet<[Lens]> | undefined;
}

export interface Hands {
  readonly focus: (id: string) => void;

  readonly keep: (id: string) => Attachment<HTMLElement>;

  readonly watch: Attachment<HTMLElement>;

  readonly reachable: boolean;
}

export const REACHABLE = [
  "a[href]",
  "button:not([disabled])",
  "input:not([disabled]):not([type=hidden])",
  "select:not([disabled])",
  "textarea:not([disabled])",
  "summary",
  "[contenteditable]:not([contenteditable=false])",
  "[tabindex]:not([tabindex='-1'])",
].join(", ");

export function landing(total: number, at: number, key: string): number | undefined {
  const last = total - 1;
  if (total === 0) return undefined;
  switch (key) {
    case "ArrowRight":
      return at === last ? 0 : at + 1;
    case "ArrowLeft":
      return at <= 0 ? last : at - 1;
    case "Home":
      return 0;
    case "End":
      return last;
    default:
      return undefined;
  }
}

export function lookOf(props: TabsProps, uid: string, hands: Hands): TabsLook {
  const { label, lenses, current, onPick } = props;
  const at = lenses.findIndex((lens) => lens.id === current);
  const tabId = (lens: Lens): string => `${uid}-tab-${lens.id}`;
  const panelId = (lens: Lens): string => `${uid}-panel-${lens.id}`;

  const travel = (event: Pressed): void => {
    const to = landing(lenses.length, at, event.key);
    const lens = to === undefined ? undefined : lenses[to];
    if (lens === undefined) return;
    event.preventDefault();
    onPick(lens.id);
    hands.focus(lens.id);
  };

  const stop = lenses[at === -1 ? 0 : at]?.id;

  const tabOf = (lens: Lens): TabLook => {
    const chosen = lens.id === current;
    return {
      lens,
      current: chosen,
      wire: {
        role: "tab",
        id: tabId(lens),
        "aria-controls": panelId(lens),
        "aria-selected": chosen,
        tabindex: lens.id === stop ? 0 : -1,
        onclick: () => {
          onPick(lens.id);
        },
        onkeydown: travel,
        [HOLD]: hands.keep(lens.id),
      },
    };
  };

  const panelOf = (lens: Lens): PanelLook => {
    const shown = lens.id === current;
    const wire: PanelWire = {
      role: "tabpanel",
      id: panelId(lens),
      "aria-labelledby": tabId(lens),
      hidden: !shown,
      tabindex: shown && !hands.reachable ? 0 : undefined,
    };
    return { lens, shown, wire: shown ? { ...wire, [HOLD]: hands.watch } : wire };
  };

  return {
    list: { role: "tablist", "aria-label": label },
    tabs: lenses.map(tabOf),
    panels: lenses.map(panelOf),
    panel: props.panel,
    mark: props.mark,
  };
}
